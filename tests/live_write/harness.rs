//! The shared harness: one account, one test at a time, sweeping before and after.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helper code outside #[test] functions: a failure here is a test failure"
)]

use reqwest::Method;
use sevdesk::Amount;
use sevdesk::*;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use time::macros::date;

pub(crate) const ZEIT: i64 = 6306695; // "PROBE-SS Zeitzone", online
pub(crate) const BANK: i64 = 6306699; // "PROBE-SS Bank", online
/// The spacing of every request of a live write test, typed or raw.
pub(crate) const REQUEST_SPACING: Duration = Duration::from_millis(1200);
pub(crate) const PREFIX: &str = "SS-LIVE-";
pub(crate) const SUPPLIER: &str = "Stripe Payments Europe, Limited";

pub(crate) static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub(crate) fn eur(minor: i64) -> Amount {
    Amount::from_minor(minor)
}

pub(crate) fn account(id: i64) -> CheckAccountId {
    CheckAccountId::new(id)
}

pub(crate) struct Ctx {
    pub(crate) c: SevdeskClient,
    pub(crate) http: reqwest::Client,
    pub(crate) marker: String,
}

/// Runs `body` alone against the account with a fresh marker; sweeps before and
/// after, and re-raises a panic of the body only after the sweep.
pub(crate) async fn run<F, Fut>(test: &str, body: F)
where
    F: FnOnce(Arc<Ctx>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let Ok(token) = std::env::var("SEVDESK_API_KEY") else {
        eprintln!("SEVDESK_API_KEY is not set: skipping live write test");
        return;
    };
    let _alone = SERIAL.lock().await;
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let ctx = Arc::new(Ctx::new(&token, format!("{PREFIX}{ts}-{test}")));
    ctx.sweep().await;
    let result = tokio::spawn(body(ctx.clone())).await;
    ctx.sweep().await;
    if let Err(e) = result {
        match e.try_into_panic() {
            Ok(panic) => std::panic::resume_unwind(panic),
            Err(e) => panic!("test task failed: {e}"),
        }
    }
}

impl Ctx {
    pub(crate) fn new(token: &str, marker: String) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        let mut auth = reqwest::header::HeaderValue::from_str(token).unwrap();
        auth.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, auth);
        headers.insert("X-Version", DEFAULT_X_VERSION.parse().unwrap());
        headers.insert(reqwest::header::ACCEPT, "application/json".parse().unwrap());
        Self {
            c: SevdeskClient::new(SevdeskClientConfig {
                pacing: Pacing {
                    initial_interval: REQUEST_SPACING,
                    min_interval: REQUEST_SPACING,
                    max_interval: Duration::from_secs(5),
                },
                ..SevdeskClientConfig::new(ApiToken::new(token))
            })
            .unwrap(),
            http: reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .unwrap(),
            marker,
        }
    }

    /// A raw call (for what the library does not offer, and to record bodies).
    /// `record` stores the body under `target/tmp/captures/<name>.json`.
    pub(crate) async fn raw(
        &self,
        method: Method,
        path: &str,
        body: Option<String>,
        record: Option<&str>,
    ) -> (u16, String) {
        tokio::time::sleep(REQUEST_SPACING).await;
        let mut req = self
            .http
            .request(method, format!("{DEFAULT_BASE_URL}{path}"));
        if let Some(body) = body {
            req = req.header("Content-Type", "application/json").body(body);
        }
        let response = req.send().await.unwrap();
        let status = response.status().as_u16();
        let text = response.text().await.unwrap();
        if let Some(name) = record {
            let dir = format!("{}/captures", env!("CARGO_TARGET_TMPDIR"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                format!("{dir}/{name}.json"),
                super::scrub::scrub_capture(&text),
            )
            .unwrap();
        }
        (status, text)
    }

    /// Delete everything carrying the `SS-LIVE-` prefix: vouchers (reset first
    /// when booked), then transactions of both accounts.
    pub(crate) async fn sweep(&self) {
        let page = Some(Page::new(1000, 0).unwrap());
        let vouchers = self
            .c
            .list_vouchers(&VoucherQuery {
                description_like: Some(PREFIX.into()),
                page,
                ..Default::default()
            })
            .await
            .unwrap_or_else(|e| {
                eprintln!("sweep: voucher list failed: {e}");
                vec![]
            });
        for v in vouchers.iter().filter(|v| {
            v.description
                .as_deref()
                .is_some_and(|d| d.starts_with(PREFIX))
        }) {
            if !matches!(v.status, Some(VoucherStatus::Draft | VoucherStatus::Open))
                && let Err(e) = self.c.reset_voucher_to_open(v.id).await
            {
                eprintln!("sweep: reset {} failed: {e}", v.id);
            }
            if v.status != Some(VoucherStatus::Draft) {
                self.back_to_draft(v.id).await;
            }
            let (status, text) = self
                .raw(Method::DELETE, &format!("/Voucher/{}", v.id), None, None)
                .await;
            if status >= 300 {
                eprintln!(
                    "sweep: delete voucher {} -> {status} {}",
                    v.id,
                    text.chars().take(200).collect::<String>()
                );
            }
        }
        for id in [ZEIT, BANK] {
            let mut q = TransactionQuery::new(account(id));
            q.paymt_purpose = Some(PREFIX.into());
            q.page = page;
            let list = self.c.list_transactions(&q).await.unwrap_or_else(|e| {
                eprintln!("sweep: transaction list failed: {e}");
                vec![]
            });
            for t in list.iter().filter(|t| {
                t.paymt_purpose
                    .as_deref()
                    .is_some_and(|p| p.starts_with(PREFIX))
            }) {
                // Only status 100 deletes (409 code 159 otherwise); a transfer
                // leaves its transactions at 400.
                if t.status != Some(TransactionStatus::Created) {
                    let back = TransactionUpdate {
                        status: Some(TransactionStatus::Created),
                        ..Default::default()
                    };
                    if let Err(e) = self.c.update_transaction(t.id, &back).await {
                        eprintln!("sweep: unbook transaction {} failed: {e}", t.id);
                    }
                }
                if let Err(e) = self.c.delete_transaction(t.id).await {
                    eprintln!("sweep: delete transaction {} failed: {e}", t.id);
                }
            }
        }
    }

    /// A voucher can only be deleted in status 50 (409 code 159), and
    /// `resetToOpen` stops at 100.
    pub(crate) async fn back_to_draft(&self, id: VoucherId) {
        if let Err(e) = self.c.reset_voucher_to_draft(id).await {
            eprintln!("sweep: back to draft {id} failed: {e}");
        }
    }

    pub(crate) fn new_transaction(
        &self,
        on: i64,
        minor: i64,
        at: OffsetDateTime,
    ) -> NewTransaction {
        NewTransaction {
            value_date: at,
            entry_date: None,
            amount: eur(minor),
            payee_payer_name: SUPPLIER.into(),
            paymt_purpose: Some(self.marker.clone()),
            status: TransactionStatus::Created,
            check_account: ObjectRef::new(account(on)),
            source_transaction: None,
            target_transaction: None,
        }
    }

    pub(crate) fn draft(&self) -> VoucherDraft {
        let mut v = VoucherDraft::new(
            date!(2026 - 09 - 29),
            SUPPLIER,
            &self.marker,
            TaxRuleId::new(14),
            CurrencyCode::EUR,
        );
        v.delivery_date = Some(date!(2026 - 09 - 29));
        v
    }
}

pub(crate) fn position(minor: i64, comment: &str) -> VoucherPositionDraft {
    VoucherPositionDraft::new(AccountDatevId::new(4285), 0, eur(minor), comment)
}

/// The draft's positions as saved, to send back with their ids.
pub(crate) fn with_ids(
    saved: &SavedVoucher,
    comments: &[(i64, &str)],
) -> Vec<VoucherPositionDraft> {
    assert_eq!(saved.voucher_pos.len(), comments.len());
    saved
        .voucher_pos
        .iter()
        .zip(comments)
        .map(|(p, (minor, comment))| position(*minor, comment).with_id(p.id))
        .collect()
}

/// A valid one-page PDF, xref offsets computed.
pub(crate) fn tiny_pdf() -> Vec<u8> {
    let stream = "BT /F1 12 Tf 20 100 Td (sevdesk-rs live test) Tj ET";
    let objects = [
        "<</Type/Catalog/Pages 2 0 R>>".to_string(),
        "<</Type/Pages/Kids[3 0 R]/Count 1>>".to_string(),
        "<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>".to_string(),
        format!("<</Length {}>>\nstream\n{stream}\nendstream", stream.len()),
        "<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>".to_string(),
    ];
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![];
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend(format!("{} 0 obj\n{body}\nendobj\n", i + 1).into_bytes());
    }
    let xref = pdf.len();
    pdf.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).into_bytes());
    for o in offsets {
        pdf.extend(format!("{o:010} 00000 n \n").into_bytes());
    }
    pdf.extend(
        format!(
            "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .into_bytes(),
    );
    pdf
}
