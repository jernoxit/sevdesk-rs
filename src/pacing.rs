//! Request pacing shared by every user of one client: AIMD on the spacing
//! between request starts. A 429 doubles the spacing and blocks all callers
//! for the wait; a run of successes shortens it again, down to a floor.
//!
//! sevDesk publishes no rate limit and none was reached in measurement (see
//! [`Pacing::DEFAULT`]), so a fresh client sends
//! without spacing and only a 429 introduces one.

use std::sync::Mutex;
use std::time::Duration;
use tokio::time::Instant;

const RATE_LIMITED: &str = "sevdesk_rate_limited_total";
const REQUEST_INTERVAL: &str = "sevdesk_request_interval_seconds";

/// Multiplicative increase on a 429.
const GROWTH: u32 = 2;
/// Additive decrease after [`SUCCESSES_PER_STEP`] requests without a 429.
const SHRINK_STEP: Duration = Duration::from_millis(50);
const SUCCESSES_PER_STEP: u32 = 10;

/// Bounds of the spacing between two request starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pacing {
    /// The spacing a fresh client starts with.
    pub initial_interval: Duration,
    /// The floor the spacing shrinks to while nothing is limited.
    pub min_interval: Duration,
    /// The ceiling the spacing grows to under 429s.
    pub max_interval: Duration,
}

impl Pacing {
    /// Measured 2026-09-30: no 429 without any spacing (11 to 18.5 sequential
    /// requests per second, bound by the response time) nor in parallel
    /// bursts up to 133/s. So no spacing until a 429 asks for one (the first
    /// raises it to 50 ms); 5 s is the ceiling. A quota over hours would count
    /// requests, which the work fixes, so spacing would not protect against
    /// it anyway.
    pub const DEFAULT: Pacing = Pacing {
        initial_interval: Duration::ZERO,
        min_interval: Duration::ZERO,
        max_interval: Duration::from_secs(5),
    };

    /// No spacing at all, for tests against a mock server.
    pub const NONE: Pacing = Pacing {
        initial_interval: Duration::ZERO,
        min_interval: Duration::ZERO,
        max_interval: Duration::ZERO,
    };
}

impl Default for Pacing {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub(crate) struct Pacer {
    pacing: Pacing,
    state: Mutex<State>,
}

struct State {
    interval: Duration,
    /// Earliest start of the next request under the current spacing.
    next_free: Option<Instant>,
    /// No request starts before this (after a 429).
    blocked_until: Option<Instant>,
    successes: u32,
    /// When the spacing last grew: a 429 for a request sent before that is
    /// the same event, not a second one.
    grown_at: Option<Instant>,
}

impl Pacer {
    pub(crate) fn new(pacing: Pacing) -> Self {
        metrics::gauge!(REQUEST_INTERVAL).set(pacing.initial_interval.as_secs_f64());
        Self {
            pacing,
            state: Mutex::new(State {
                interval: pacing.initial_interval,
                next_free: None,
                blocked_until: None,
                successes: 0,
                grown_at: None,
            }),
        }
    }

    pub(crate) fn interval(&self) -> Duration {
        self.lock().interval
    }

    /// Claims the next start slot and returns when to send. Callers queue in
    /// call order; the slot is taken here, the sleeping happens outside.
    pub(crate) fn reserve(&self) -> Instant {
        let mut s = self.lock();
        let start = [Some(Instant::now()), s.next_free, s.blocked_until]
            .into_iter()
            .flatten()
            .max()
            .unwrap_or_else(Instant::now);
        s.next_free = Some(start + s.interval);
        start
    }

    /// An answer other than 429.
    pub(crate) fn succeeded(&self) {
        let mut s = self.lock();
        s.successes += 1;
        if s.successes >= SUCCESSES_PER_STEP {
            s.successes = 0;
            let shrunk = s
                .interval
                .saturating_sub(SHRINK_STEP)
                .max(self.pacing.min_interval);
            s.interval = shrunk;
            metrics::gauge!(REQUEST_INTERVAL).set(shrunk.as_secs_f64());
        }
    }

    /// A 429 for the request sent at `sent`: back off for `wait`, and slow down.
    pub(crate) fn rate_limited(&self, sent: Instant, wait: Duration) {
        metrics::counter!(RATE_LIMITED).increment(1);
        let mut s = self.lock();
        let now = Instant::now();
        s.blocked_until = Some(s.blocked_until.map_or(now + wait, |b| b.max(now + wait)));
        s.successes = 0;
        if s.grown_at.is_none_or(|grown| sent >= grown) {
            let grown = (s.interval * GROWTH)
                .max(SHRINK_STEP)
                .min(self.pacing.max_interval);
            s.interval = grown;
            s.grown_at = Some(now);
            metrics::gauge!(REQUEST_INTERVAL).set(grown.as_secs_f64());
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        // The state stays consistent under a panic elsewhere: every update is
        // a plain assignment.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: Pacing = Pacing {
        initial_interval: Duration::from_millis(200),
        min_interval: Duration::from_millis(100),
        max_interval: Duration::from_millis(700),
    };

    #[tokio::test(start_paused = true)]
    async fn starts_are_spaced_by_the_interval() {
        let pacer = Pacer::new(FAST);
        let t0 = Instant::now();
        let starts: Vec<_> = (0..3).map(|_| pacer.reserve() - t0).collect();
        assert_eq!(
            starts,
            [0, 200, 400].map(Duration::from_millis),
            "slots are claimed one interval apart"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_429_doubles_the_interval_up_to_the_ceiling_and_blocks_starts() {
        let pacer = Pacer::new(FAST);
        let mut sent = Instant::now();
        let mut seen = vec![];
        for _ in 0..3 {
            pacer.rate_limited(sent, Duration::from_secs(2));
            seen.push(pacer.interval());
            tokio::time::advance(Duration::from_millis(1)).await;
            sent = Instant::now();
        }
        assert_eq!(seen, [400, 700, 700].map(Duration::from_millis));
        // Blocked for the wait from the last 429.
        assert!(pacer.reserve() - Instant::now() >= Duration::from_millis(1999));
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_429s_of_one_event_grow_the_interval_once() {
        let pacer = Pacer::new(FAST);
        let sent = Instant::now();
        tokio::time::advance(Duration::from_millis(10)).await;
        pacer.rate_limited(sent, Duration::ZERO);
        pacer.rate_limited(sent, Duration::ZERO);
        assert_eq!(pacer.interval(), Duration::from_millis(400));
    }

    #[tokio::test(start_paused = true)]
    async fn successes_shrink_the_interval_to_the_floor() {
        let pacer = Pacer::new(FAST);
        for _ in 0..(SUCCESSES_PER_STEP - 1) {
            pacer.succeeded();
        }
        assert_eq!(pacer.interval(), Duration::from_millis(200));
        pacer.succeeded();
        assert_eq!(pacer.interval(), Duration::from_millis(150));
        for _ in 0..(10 * SUCCESSES_PER_STEP) {
            pacer.succeeded();
        }
        assert_eq!(pacer.interval(), FAST.min_interval);
    }

    #[tokio::test(start_paused = true)]
    async fn a_429_resets_the_success_run() {
        let pacer = Pacer::new(FAST);
        for _ in 0..(SUCCESSES_PER_STEP - 1) {
            pacer.succeeded();
        }
        pacer.rate_limited(Instant::now(), Duration::ZERO);
        pacer.succeeded();
        assert_eq!(pacer.interval(), Duration::from_millis(400));
    }
}
