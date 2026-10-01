//! The transport: authentication, throttling, 429 backoff, and the one place
//! where a response body becomes either an error or a value.
//!
//! **An error in a 200 is an error.** sevDesk answers some failures with HTTP
//! 200 and an `error` object (measured: the storno of a partially paid invoice
//! answers 200 with code 390), so EVERY body is checked for one, whatever the
//! status. What a body's `objects` look like is decoded one layer up.

use crate::Amount;
use crate::error::{ApiErrorBody, SevdeskError};
use crate::pacing::{Pacer, Pacing};
use crate::wire::Guard;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, RETRY_AFTER};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::fmt;
use std::time::Duration;

pub const DEFAULT_BASE_URL: &str = "https://my.sevdesk.de/api/v1";

/// The `X-Version` we send. sevDesk versions single resources through this
/// header and answers an unknown value with an error listing the valid ones.
/// Probed 2026-09-29 against the test account: `default` (the documented
/// "oldest version" value) is accepted on every endpoint used here, while
/// `1.0`/`2.0` are accepted on Voucher/Invoice but refused on
/// `/Tools/bookkeepingSystemVersion` — whose "valid values" list is EMPTY, so
/// the error names no alternative. Pinned to `default` so a future default
/// change on sevDesk's side does not move us silently.
pub const DEFAULT_X_VERSION: &str = "default";

const USER_AGENT: &str = "sevdesk-rs (jernoxit)";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// Attempts per request including the first; only HTTP 429 repeats it. The
/// project owner decided (2026-09-30) that a 429 means the request was NOT
/// processed (the limiter rejects it before the service), so repeating a write
/// needs no read-back. That is the one exception to "an error response does
/// not mean not written"; every other status keeps it.
const MAX_ATTEMPTS: u32 = 8;
/// Wait after a 429 without `Retry-After`: doubles per attempt (1, 2, 4, ...
/// 30 s; about 90 s over all attempts).
const BACKOFF_BASE: Duration = Duration::from_secs(1);
const BACKOFF_CAP: Duration = Duration::from_secs(30);

/// The API token. Debug and Display never show it.
#[derive(Clone)]
pub struct ApiToken(String);

impl ApiToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }
}

impl fmt::Debug for ApiToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiToken(<redacted>)")
    }
}

impl fmt::Display for ApiToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Everything the client needs, passed in; the library reads no environment.
#[derive(Debug, Clone)]
pub struct SevdeskClientConfig {
    pub base_url: String,
    pub api_token: ApiToken,
    pub pacing: Pacing,
    /// `None` sends [`DEFAULT_X_VERSION`].
    pub x_version: Option<String>,
}

impl SevdeskClientConfig {
    /// The production URL, the default spacing and the pinned `X-Version`.
    pub fn new(api_token: ApiToken) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.into(),
            api_token,
            pacing: Pacing::DEFAULT,
            x_version: None,
        }
    }
}

pub struct SevdeskClient {
    http: reqwest::Client,
    base_url: String,
    /// Shared by every caller of this client, so the spacing holds across
    /// all of them.
    pacer: Pacer,
}

impl fmt::Debug for SevdeskClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SevdeskClient")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

/// A successful body, error-checked.
pub(crate) struct Body(pub(crate) String);

impl SevdeskClient {
    pub fn new(config: SevdeskClientConfig) -> Result<Self, SevdeskError> {
        let invalid = |what: &str| SevdeskError::InvalidRequest(format!("invalid {what}"));
        let mut headers = HeaderMap::new();
        // Raw token, no "Bearer".
        let mut auth =
            HeaderValue::from_str(&config.api_token.0).map_err(|_| invalid("api token"))?;
        auth.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth);
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        let version = config.x_version.as_deref().unwrap_or(DEFAULT_X_VERSION);
        headers.insert(
            "X-Version",
            HeaderValue::from_str(version).map_err(|_| invalid("X-Version"))?,
        );
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;
        Ok(Self {
            http,
            base_url: config.base_url.trim_end_matches('/').to_owned(),
            pacer: Pacer::new(config.pacing),
        })
    }

    /// The current spacing between request starts (it adapts to 429s).
    pub fn request_interval(&self) -> Duration {
        self.pacer.interval()
    }

    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    pub(crate) fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.http.request(method, self.url(path))
    }

    pub(crate) async fn get(
        &self,
        path: &str,
        query: &[(&'static str, String)],
    ) -> Result<Body, SevdeskError> {
        self.send(|| self.request(Method::GET, path).query(query))
            .await
    }

    pub(crate) async fn send_json<B: Serialize>(
        &self,
        method: Method,
        path: &str,
        body: &B,
    ) -> Result<Body, SevdeskError> {
        self.send(|| self.request(method.clone(), path).json(body))
            .await
    }

    pub(crate) async fn send_empty(
        &self,
        method: Method,
        path: &str,
    ) -> Result<Body, SevdeskError> {
        self.send(|| self.request(method.clone(), path)).await
    }

    /// Wait for the paced slot, send, and repeat on 429 (writes too: a 429
    /// was not processed, see [`MAX_ATTEMPTS`]).
    pub(crate) async fn send(
        &self,
        build: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<Body, SevdeskError> {
        for attempt in 1..=MAX_ATTEMPTS {
            let slot = self.pacer.reserve();
            tokio::time::sleep_until(slot).await;
            let response = build().send().await?;
            let status = response.status();
            if status == StatusCode::TOO_MANY_REQUESTS {
                let wait = retry_wait(
                    response
                        .headers()
                        .get(RETRY_AFTER)
                        .and_then(|v| v.to_str().ok()),
                    attempt,
                );
                tracing::warn!(
                    attempt,
                    wait_ms = wait.as_millis() as u64,
                    "sevDesk answered 429, retrying after the wait"
                );
                self.pacer.rate_limited(slot, wait);
                continue;
            }
            self.pacer.succeeded();
            let text = response.text().await?;
            return check_body(status.as_u16(), text);
        }
        Err(SevdeskError::RateLimited {
            attempts: MAX_ATTEMPTS,
        })
    }
}

/// `Retry-After` in seconds if sevDesk sent one, else exponential backoff.
fn retry_wait(header: Option<&str>, attempt: u32) -> Duration {
    let wait = header
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| BACKOFF_BASE * 2_u32.pow(attempt - 1));
    wait.min(BACKOFF_CAP)
}

/// The single place a body becomes an error or a success — for every status.
pub(crate) fn check_body(status: u16, text: String) -> Result<Body, SevdeskError> {
    #[derive(Deserialize)]
    struct Probe {
        #[serde(default)]
        error: Option<ApiErrorBody>,
    }
    match serde_json::from_str::<Probe>(&text) {
        Ok(Probe { error: Some(error) }) => Err(error.into_error(status)),
        _ if !(200..300).contains(&status) => Err(SevdeskError::Api {
            status,
            code: None,
            message: text.chars().take(200).collect(),
        }),
        _ => Ok(Body(text)),
    }
}

#[derive(Deserialize)]
struct Envelope {
    #[serde(default)]
    objects: Option<Box<RawValue>>,
}

impl Body {
    fn objects(&self, context: &'static str) -> Result<Box<RawValue>, SevdeskError> {
        let envelope: Envelope = serde_json::from_str(&self.0)
            .map_err(|source| SevdeskError::Decode { context, source })?;
        match envelope.objects {
            Some(raw) if raw.get() != "null" => Ok(raw),
            _ => Err(SevdeskError::DriftGuard {
                context,
                detail: "response has no `objects`".into(),
            }),
        }
    }

    /// `objects` as a list of `T`.
    pub(crate) fn list<T: DeserializeOwned + Guard>(&self) -> Result<Vec<T>, SevdeskError> {
        let raw = self.objects(T::NAME)?;
        let items: Vec<T> =
            serde_json::from_str(raw.get()).map_err(|source| SevdeskError::Decode {
                context: T::NAME,
                source,
            })?;
        items.iter().try_for_each(guard)?;
        Ok(items)
    }

    /// `objects` as one `T`: sevDesk answers by-id reads with a list of one
    /// and most factories with the bare object; both read.
    pub(crate) fn one<T: DeserializeOwned + Guard>(&self) -> Result<T, SevdeskError> {
        let raw = self.objects(T::NAME)?;
        let decode = |text: &str| {
            serde_json::from_str::<T>(text).map_err(|source| SevdeskError::Decode {
                context: T::NAME,
                source,
            })
        };
        let item = if raw.get().trim_start().starts_with('[') {
            self.list::<T>()?
                .into_iter()
                .next()
                .ok_or_else(|| SevdeskError::DriftGuard {
                    context: T::NAME,
                    detail: "empty `objects` list".into(),
                })?
        } else {
            decode(raw.get())?
        };
        guard(&item)?;
        Ok(item)
    }

    /// `objects` as a bare amount (`getBalanceAtDate`).
    pub(crate) fn amount(&self, context: &'static str) -> Result<Amount, SevdeskError> {
        let raw = self.objects(context)?;
        crate::codec::parse_amount(raw.get())
            .map_err(|detail| SevdeskError::DriftGuard { context, detail })
    }

    /// `objects` as an arbitrary JSON object read into `T` (no guard).
    pub(crate) fn plain<T: DeserializeOwned>(
        &self,
        context: &'static str,
    ) -> Result<T, SevdeskError> {
        let raw = self.objects(context)?;
        serde_json::from_str(raw.get()).map_err(|source| SevdeskError::Decode { context, source })
    }
}

fn guard<T: Guard>(item: &T) -> Result<(), SevdeskError> {
    if item.load_bearing_present() {
        Ok(())
    } else {
        Err(SevdeskError::DriftGuard {
            context: T::NAME,
            detail: "none of the load-bearing fields is present".into(),
        })
    }
}
