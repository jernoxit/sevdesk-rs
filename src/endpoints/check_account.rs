use crate::Amount;
use crate::client::SevdeskClient;
use crate::error::SevdeskError;
use crate::ids::CheckAccountId;
use crate::payload::{CheckAccountUpdate, CreateFileImportAccount};
use crate::wire::CheckAccount;
use reqwest::Method;
use time::Date;

impl SevdeskClient {
    /// `GET /CheckAccount`.
    pub async fn list_check_accounts(&self) -> Result<Vec<CheckAccount>, SevdeskError> {
        self.get("/CheckAccount", &[]).await?.list()
    }

    /// `GET /CheckAccount/{id}`.
    pub async fn check_account(&self, id: CheckAccountId) -> Result<CheckAccount, SevdeskError> {
        self.get(&format!("/CheckAccount/{id}"), &[]).await?.one()
    }

    /// `PUT /CheckAccount/{id}`.
    pub async fn update_check_account(
        &self,
        id: CheckAccountId,
        update: &CheckAccountUpdate,
    ) -> Result<CheckAccount, SevdeskError> {
        self.send_json(Method::PUT, &format!("/CheckAccount/{id}"), update)
            .await?
            .one()
    }

    /// `POST /CheckAccount/Factory/fileImportAccount`: an online account that
    /// takes transactions per API.
    pub async fn create_file_import_account(
        &self,
        account: &CreateFileImportAccount,
    ) -> Result<CheckAccount, SevdeskError> {
        self.send_json(
            Method::POST,
            "/CheckAccount/Factory/fileImportAccount",
            account,
        )
        .await?
        .one()
    }

    /// `GET /CheckAccount/{id}/getBalanceAtDate`, in the account's currency.
    /// sevDesk cuts the day in BERLIN time: for 31.08. a transaction at
    /// 23:30+02:00 counts, one at 00:30+02:00 on 01.09. does not.
    pub async fn balance_at_date(
        &self,
        id: CheckAccountId,
        date: Date,
    ) -> Result<Amount, SevdeskError> {
        let date = format!(
            "{:04}-{:02}-{:02}",
            date.year(),
            u8::from(date.month()),
            date.day()
        );
        self.get(
            &format!("/CheckAccount/{id}/getBalanceAtDate"),
            &[("date", date)],
        )
        .await?
        .amount("getBalanceAtDate")
    }
}
