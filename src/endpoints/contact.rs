use crate::client::SevdeskClient;
use crate::document_payload::{
    CommunicationWayUpdate, ContactAddressUpdate, ContactUpdate, NewCommunicationWay, NewContact,
    NewContactAddress,
};
use crate::error::SevdeskError;
use crate::ids::*;
use crate::query::{ContactQuery, Page};
use crate::wire::{CommunicationWay, Contact, ContactAddress};
use reqwest::Method;

/// The `contact[id]`/`contact[objectName]` filter pair.
fn contact_filter(contact: ContactId) -> [(&'static str, String); 2] {
    [
        ("contact[id]", contact.to_string()),
        ("contact[objectName]", ContactId::OBJECT_NAME.into()),
    ]
}

impl SevdeskClient {
    /// `GET /Contact` with the filters of [`ContactQuery`]. The default list
    /// holds organisations only; see [`crate::ContactDepth`]. Without a
    /// [`crate::Page`] in the query sevDesk returns at most 100 contacts
    /// (measured 2026-11-22): the list is silently truncated.
    pub async fn list_contacts(&self, query: &ContactQuery) -> Result<Vec<Contact>, SevdeskError> {
        self.get("/Contact", &query.pairs()).await?.list()
    }

    /// `GET /Contact?name=…`. The `name` filter is a SUBSTRING match, ignoring
    /// case: compare on the result. A substring can match more than the 100
    /// contacts sevDesk returns unpaged, so this walks the pages of
    /// [`Page::MAX_LIMIT`] until one comes back short.
    pub async fn list_contacts_by_name(&self, name: &str) -> Result<Vec<Contact>, SevdeskError> {
        let mut all = Vec::new();
        loop {
            let offset = u32::try_from(all.len()).unwrap_or(u32::MAX);
            let page = self
                .list_contacts(&ContactQuery {
                    name: Some(name.to_owned()),
                    page: Some(Page::new(Page::MAX_LIMIT, offset)?),
                    ..Default::default()
                })
                .await?;
            let full = page.len() == Page::MAX_LIMIT as usize;
            all.extend(page);
            if !full {
                return Ok(all);
            }
        }
    }

    /// `GET /Contact/{id}`.
    pub async fn contact(&self, id: ContactId) -> Result<Contact, SevdeskError> {
        self.get(&format!("/Contact/{id}"), &[]).await?.one()
    }

    /// `GET /Contact?customerNumber=…`, compared EXACTLY on the result.
    /// Measured 2026-10-01: the server filters exactly (the full number finds
    /// the contact, a prefix of it finds nothing); the comparison on the
    /// result stays as a guard against a filter sevDesk ignores (which would
    /// answer every contact). `None` when no contact has that number; several
    /// with it is a drift error, as numbers are unique. The exact filter
    /// returns at most a handful of contacts, far below the 100 sevDesk
    /// returns unpaged, so no paging is needed.
    pub async fn find_contact_by_customer_number(
        &self,
        customer_number: &str,
    ) -> Result<Option<Contact>, SevdeskError> {
        let found = self
            .list_contacts(&ContactQuery {
                customer_number: Some(customer_number.to_owned()),
                ..Default::default()
            })
            .await?;
        let mut exact = found
            .into_iter()
            .filter(|c| c.customer_number.as_deref() == Some(customer_number));
        let first = exact.next();
        if exact.next().is_some() {
            return Err(SevdeskError::DriftGuard {
                context: "GET /Contact customerNumber",
                detail: format!("several contacts have customer number {customer_number}"),
            });
        }
        Ok(first)
    }

    /// `POST /Contact`.
    pub async fn create_contact(&self, contact: &NewContact) -> Result<Contact, SevdeskError> {
        self.send_json(Method::POST, "/Contact", contact)
            .await?
            .one()
    }

    /// `PUT /Contact/{id}`; a partial update (measured).
    pub async fn update_contact(
        &self,
        id: ContactId,
        update: &ContactUpdate,
    ) -> Result<Contact, SevdeskError> {
        self.send_json(Method::PUT, &format!("/Contact/{id}"), update)
            .await?
            .one()
    }

    /// `POST /ContactAddress`. A separate call from [`Self::create_contact`],
    /// not atomic with it.
    pub async fn create_contact_address(
        &self,
        address: &NewContactAddress,
    ) -> Result<ContactAddress, SevdeskError> {
        self.send_json(Method::POST, "/ContactAddress", address)
            .await?
            .one()
    }

    /// `PUT /ContactAddress/{id}`; a partial update, answers the whole address
    /// (measured 2026-10-01).
    pub async fn update_contact_address(
        &self,
        id: ContactAddressId,
        update: &ContactAddressUpdate,
    ) -> Result<ContactAddress, SevdeskError> {
        self.send_json(Method::PUT, &format!("/ContactAddress/{id}"), update)
            .await?
            .one()
    }

    /// `GET /ContactAddress?contact[id]=…&contact[objectName]=Contact`.
    /// Measured 2026-10-01: the server filters by contact; the result is
    /// narrowed again on the client side as a guard against an ignored filter.
    pub async fn contact_addresses(
        &self,
        contact: ContactId,
    ) -> Result<Vec<ContactAddress>, SevdeskError> {
        let all = self
            .get("/ContactAddress", &contact_filter(contact))
            .await?
            .list::<ContactAddress>()?;
        Ok(all
            .into_iter()
            .filter(|a| a.contact.is_some_and(|c| c.id == contact))
            .collect())
    }

    /// `POST /CommunicationWay`. A separate call, not atomic with the others.
    pub async fn create_communication_way(
        &self,
        way: &NewCommunicationWay,
    ) -> Result<CommunicationWay, SevdeskError> {
        self.send_json(Method::POST, "/CommunicationWay", way)
            .await?
            .one()
    }

    /// `PUT /CommunicationWay/{id}`; a partial update, answers the whole way
    /// (measured 2026-10-01).
    pub async fn update_communication_way(
        &self,
        id: CommunicationWayId,
        update: &CommunicationWayUpdate,
    ) -> Result<CommunicationWay, SevdeskError> {
        self.send_json(Method::PUT, &format!("/CommunicationWay/{id}"), update)
            .await?
            .one()
    }

    /// `GET /CommunicationWay?contact[id]=…&contact[objectName]=Contact`.
    /// Measured 2026-10-01: the server filters by contact; the result is
    /// narrowed again on the client side as a guard against an ignored filter.
    pub async fn communication_ways(
        &self,
        contact: ContactId,
    ) -> Result<Vec<CommunicationWay>, SevdeskError> {
        let all = self
            .get("/CommunicationWay", &contact_filter(contact))
            .await?
            .list::<CommunicationWay>()?;
        Ok(all
            .into_iter()
            .filter(|w| w.contact.is_some_and(|c| c.id == contact))
            .collect())
    }
}
