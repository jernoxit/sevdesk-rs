use super::Guard;
use crate::Amount;
use crate::codec::amount_opt;
use crate::ids::*;
use crate::status::*;
use serde::{Deserialize, Deserializer};
use time::OffsetDateTime;

/// A CheckAccountTransaction. Carries no currency: its amount is in the
/// currency of its check account ([`CheckAccount::currency`](super::CheckAccount)).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id: TransactionId,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub value_date: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub entry_date: Option<OffsetDateTime>,
    /// Signed: negative for money leaving the account.
    #[serde(default, deserialize_with = "amount_opt")]
    pub amount: Option<Amount>,
    #[serde(default)]
    pub paymt_purpose: Option<String>,
    #[serde(default)]
    pub payee_payer_name: Option<String>,
    #[serde(default)]
    pub check_account: Option<ObjectRef<CheckAccountId>>,
    #[serde(default)]
    pub status: Option<TransactionStatus>,
    #[serde(default)]
    pub source_transaction: Option<ObjectRef<TransactionId>>,
    #[serde(default)]
    pub target_transaction: Option<ObjectRef<TransactionId>>,
    /// Set on a deleted transaction. `GET /{id}` still returns such a
    /// transaction (the list omits it), so this is the only way to tell.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub deleted_at: Option<OffsetDateTime>,
}

impl Transaction {
    /// The transaction was deleted; treat it as gone.
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

impl Guard for Transaction {
    const NAME: &'static str = "CheckAccountTransaction";
    fn load_bearing_present(&self) -> bool {
        self.amount.is_some() || self.status.is_some()
    }
}

/// The document a transaction was booked to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookedDocument {
    Voucher(VoucherId),
    Invoice(InvoiceId),
    CreditNote(CreditNoteId),
    Other { object_name: String, id: i64 },
}

impl<'de> Deserialize<'de> for BookedDocument {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Raw {
            #[serde(deserialize_with = "crate::codec::deserialize_i64")]
            id: i64,
            object_name: String,
        }
        let Raw { id, object_name } = Raw::deserialize(d)?;
        Ok(match object_name.as_str() {
            "Voucher" => Self::Voucher(VoucherId::new(id)),
            "Invoice" => Self::Invoice(InvoiceId::new(id)),
            "CreditNote" => Self::CreditNote(CreditNoteId::new(id)),
            _ => Self::Other { object_name, id },
        })
    }
}

/// One booking of a transaction (`GET /CheckAccountTransactionLog`, undocumented).
/// Carries no currency: `amount_paid` is in the transaction's.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionLogEntry {
    #[serde(deserialize_with = "crate::codec::deserialize_i64")]
    pub id: i64,
    #[serde(rename = "checkAccountTransaction")]
    pub transaction: ObjectRef<TransactionId>,
    /// The booked document.
    #[serde(default)]
    pub object: Option<BookedDocument>,
    /// Signed like the transaction.
    #[serde(default, deserialize_with = "amount_opt")]
    pub amount_paid: Option<Amount>,
    #[serde(default)]
    pub from_status: Option<TransactionStatus>,
    #[serde(default)]
    pub to_status: Option<TransactionStatus>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub booking_date: Option<OffsetDateTime>,
}

impl Guard for TransactionLogEntry {
    const NAME: &'static str = "CheckAccountTransactionLog";
    fn load_bearing_present(&self) -> bool {
        self.object.is_some() || self.amount_paid.is_some()
    }
}
