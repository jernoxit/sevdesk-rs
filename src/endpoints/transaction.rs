use crate::client::SevdeskClient;
use crate::error::SevdeskError;
use crate::ids::{Resource, TransactionId};
use crate::payload::{NewTransaction, TransactionUpdate};
use crate::query::{Page, TransactionQuery};
use crate::wire::{Transaction, TransactionLogEntry};
use reqwest::Method;

impl SevdeskClient {
    /// `POST /CheckAccountTransaction`. Only online accounts take it; an
    /// offline one answers 422 ([`SevdeskError::is_offline_account`]).
    pub async fn create_transaction(
        &self,
        transaction: &NewTransaction,
    ) -> Result<Transaction, SevdeskError> {
        self.send_json(Method::POST, "/CheckAccountTransaction", transaction)
            .await?
            .one()
    }

    /// `GET /CheckAccountTransaction`. Deleted transactions are omitted here —
    /// this list, not [`Self::transaction`], says whether one exists.
    pub async fn list_transactions(
        &self,
        query: &TransactionQuery,
    ) -> Result<Vec<Transaction>, SevdeskError> {
        self.get("/CheckAccountTransaction", &query.pairs())
            .await?
            .list()
    }

    /// `GET /CheckAccountTransaction/{id}`. A DELETED transaction is still
    /// returned, with `deleted_at` set: check [`Transaction::is_deleted`].
    pub async fn transaction(&self, id: TransactionId) -> Result<Transaction, SevdeskError> {
        self.get(&format!("/CheckAccountTransaction/{id}"), &[])
            .await?
            .one()
    }

    /// `PUT /CheckAccountTransaction/{id}`.
    pub async fn update_transaction(
        &self,
        id: TransactionId,
        update: &TransactionUpdate,
    ) -> Result<Transaction, SevdeskError> {
        self.send_json(
            Method::PUT,
            &format!("/CheckAccountTransaction/{id}"),
            update,
        )
        .await?
        .one()
    }

    /// `DELETE /CheckAccountTransaction/{id}`. Measured: only a transaction in
    /// status 100 deletes; a booked or linked one (200/400) answers 409 with
    /// code 159, so take its bookings back first (`resetToOpen` on the
    /// document, or [`Self::update_transaction`] to status 100 for a transfer).
    /// The answer is `{"objects":[null]}`.
    pub async fn delete_transaction(&self, id: TransactionId) -> Result<(), SevdeskError> {
        self.send_empty(Method::DELETE, &format!("/CheckAccountTransaction/{id}"))
            .await
            .map(drop)
    }

    /// `GET /CheckAccountTransactionLog` (undocumented; the filter works):
    /// the bookings of one transaction. `resetToOpen` on the document removes
    /// its entries.
    pub async fn transaction_log(
        &self,
        id: TransactionId,
    ) -> Result<Vec<TransactionLogEntry>, SevdeskError> {
        self.get(
            "/CheckAccountTransactionLog",
            &[
                ("checkAccountTransaction[id]", id.to_string()),
                (
                    "checkAccountTransaction[objectName]",
                    TransactionId::OBJECT_NAME.into(),
                ),
            ],
        )
        .await?
        .list()
    }

    /// One page of the WHOLE booking log of the sevDesk client, all accounts
    /// (measured): the log has no filter by document (`creditNote[id]`,
    /// `object[id]`, `object[objectName]` and `objectName` are all IGNORED and
    /// answer every entry), but `limit`/`offset` page it stably. Attribute an
    /// entry to a document on the client side through [`TransactionLogEntry::object`].
    pub async fn transaction_log_page(
        &self,
        page: Page,
    ) -> Result<Vec<TransactionLogEntry>, SevdeskError> {
        let mut pairs = Vec::new();
        page.push_to(&mut pairs);
        self.get("/CheckAccountTransactionLog", &pairs)
            .await?
            .list()
    }
}
