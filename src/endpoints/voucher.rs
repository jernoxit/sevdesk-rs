use crate::client::SevdeskClient;
use crate::error::SevdeskError;
use crate::ids::{Resource, VoucherId};
use crate::payload::{BookVoucherAmount, SaveVoucher};
use crate::query::VoucherQuery;
use crate::wire::{SavedVoucher, UploadedFile, Voucher, VoucherPosition};
use reqwest::{Method, multipart};

impl SevdeskClient {
    /// `POST /Voucher/Factory/uploadTempFile` (multipart field `file`). Attach
    /// the result to a voucher via [`SaveVoucher::filename`] (the answer also
    /// carries a large base64 preview in `content`, which is ignored); the API does not
    /// trigger sevDesk's document recognition, so all fields stay as sent.
    pub async fn upload_temp_file(
        &self,
        file_name: &str,
        mime_type: &str,
        bytes: Vec<u8>,
    ) -> Result<UploadedFile, SevdeskError> {
        let body = self
            .send(|| {
                let part = multipart::Part::bytes(bytes.clone()).file_name(file_name.to_owned());
                let part = part.mime_str(mime_type).unwrap_or_else(|_| {
                    multipart::Part::bytes(bytes.clone()).file_name(file_name.to_owned())
                });
                self.request(Method::POST, "/Voucher/Factory/uploadTempFile")
                    .multipart(multipart::Form::new().part("file", part))
            })
            .await?;
        body.one()
    }

    /// `POST /Voucher/Factory/saveVoucher`: creates a draft or, with an id,
    /// updates one. On an update ALL positions go with their ids.
    pub async fn save_voucher(&self, save: &SaveVoucher) -> Result<SavedVoucher, SevdeskError> {
        self.send_json(Method::POST, "/Voucher/Factory/saveVoucher", save)
            .await?
            .one()
    }

    /// `GET /Voucher/{id}`.
    pub async fn voucher(&self, id: VoucherId) -> Result<Voucher, SevdeskError> {
        self.get(&format!("/Voucher/{id}"), &[]).await?.one()
    }

    /// `GET /Voucher`. `description_like` is a SUBSTRING match.
    pub async fn list_vouchers(&self, query: &VoucherQuery) -> Result<Vec<Voucher>, SevdeskError> {
        self.get("/Voucher", &query.pairs()).await?.list()
    }

    /// `GET /VoucherPos?voucher[id]=…`.
    pub async fn voucher_positions(
        &self,
        id: VoucherId,
    ) -> Result<Vec<VoucherPosition>, SevdeskError> {
        self.get(
            "/VoucherPos",
            &[
                ("voucher[id]", id.to_string()),
                ("voucher[objectName]", VoucherId::OBJECT_NAME.into()),
            ],
        )
        .await?
        .list()
    }

    /// `PUT /Voucher/{id}/bookAmount`; sevDesk answers with the booking as a
    /// `VoucherLog` object (`amountPayed`, `fromStatus`, `toStatus`). The amount carries the sign of the
    /// transaction; sevDesk refuses a booking on a paid voucher with code 751
    /// and accepts the one that exceeds the total (capping `paidAmount`).
    pub async fn book_voucher_amount(
        &self,
        id: VoucherId,
        booking: &BookVoucherAmount,
    ) -> Result<(), SevdeskError> {
        self.send_json(Method::PUT, &format!("/Voucher/{id}/bookAmount"), booking)
            .await
            .map(drop)
    }

    /// `PUT /Voucher/{id}/resetToOpen`: takes back all bookings (the answer is
    /// the voucher); the transactions return to status 100 and their log
    /// entries disappear.
    pub async fn reset_voucher_to_open(&self, id: VoucherId) -> Result<(), SevdeskError> {
        self.send_empty(Method::PUT, &format!("/Voucher/{id}/resetToOpen"))
            .await
            .map(drop)
    }

    /// `PUT /Voucher/{id}/resetToDraft`: a released voucher (status 100) back
    /// to a draft. Measured: `saveVoucher` refuses to update anything but a
    /// draft (400, code 90400 "Use resetToDraft first"), and only a draft can
    /// be deleted (409, code 159).
    pub async fn reset_voucher_to_draft(&self, id: VoucherId) -> Result<(), SevdeskError> {
        self.send_empty(Method::PUT, &format!("/Voucher/{id}/resetToDraft"))
            .await
            .map(drop)
    }

    /// `DELETE /Voucher/{id}`. Measured: only a draft deletes (409, code 159
    /// otherwise): take a released voucher back with
    /// [`Self::reset_voucher_to_draft`] first.
    pub async fn delete_voucher(&self, id: VoucherId) -> Result<(), SevdeskError> {
        self.send_empty(Method::DELETE, &format!("/Voucher/{id}"))
            .await
            .map(drop)
    }
}
