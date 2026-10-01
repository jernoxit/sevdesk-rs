use crate::client::SevdeskClient;
use crate::error::SevdeskError;
use crate::wire::{PaymentMethod, ReceiptGuidance, SevSequence, SevUser};
use serde::Deserialize;

impl SevdeskClient {
    /// `GET /ReceiptGuidance/forExpense`: the booking accounts for expenses
    /// and the tax rules each allows.
    pub async fn receipt_guidance_for_expense(&self) -> Result<Vec<ReceiptGuidance>, SevdeskError> {
        self.get("/ReceiptGuidance/forExpense", &[]).await?.list()
    }

    /// `GET /PaymentMethod`: the account's payment methods. The ids differ per
    /// account, so a caller finds its method (e.g. "Online-Zahlung") here by
    /// name or `electronic_invoice_id`.
    pub async fn list_payment_methods(&self) -> Result<Vec<PaymentMethod>, SevdeskError> {
        self.get("/PaymentMethod", &[]).await?.list()
    }

    /// `GET /SevUser`.
    pub async fn list_sev_users(&self) -> Result<Vec<SevUser>, SevdeskError> {
        self.get("/SevUser", &[]).await?.list()
    }

    /// The number the next credit note gets (`GU-1011`), from
    /// `GET /SevSequence/Factory/getByType`. The sequence only advances when
    /// a credit note is enshrined, so two drafts read the same number.
    pub async fn next_credit_note_number(&self) -> Result<String, SevdeskError> {
        let sequence: SevSequence = self
            .get(
                "/SevSequence/Factory/getByType",
                &[
                    ("objectType", "CreditNote".to_owned()),
                    ("type", "CN".to_owned()),
                ],
            )
            .await?
            .one()?;
        sequence.next_number().ok_or(SevdeskError::DriftGuard {
            context: "SevSequence",
            detail: "format has no %NUMBER".into(),
        })
    }

    /// `GET /Tools/bookkeepingSystemVersion`: `"2.0"` on the test account.
    pub async fn bookkeeping_system_version(&self) -> Result<String, SevdeskError> {
        #[derive(Deserialize)]
        struct Version {
            version: String,
        }
        let body = self.get("/Tools/bookkeepingSystemVersion", &[]).await?;
        Ok(body.plain::<Version>("bookkeepingSystemVersion")?.version)
    }
}
