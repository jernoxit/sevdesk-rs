//! The read-only inputs of the gate: the account check, the contact person,
//! the payment method, an existing enshrined invoice and the ids the option
//! fields point at. Nothing here writes.

use crate::docs::*;
use crate::gate::id_of;
use crate::harness::*;
use reqwest::Method;
use sevdesk::*;

/// The `SevClient` of the test account; the run aborts on any other.
const TEST_SEV_CLIENT: i64 = 1303847;
/// The name prefix of the contacts the live tests create.
const CONTACT_PREFIX: &str = "SDRS-LIVE Testkunde GmbH";
/// Net amount of the credit note position, in cent.
pub(crate) const CREDIT_NET: i64 = 500;

pub(crate) struct Inputs {
    pub(crate) customer: Customer,
    pub(crate) person: SevUserId,
    pub(crate) pm: PaymentMethodId,
    /// An existing enshrined invoice the credit note refers to.
    pub(crate) origin: Invoice,
    pub(crate) document: Option<DocumentId>,
    pub(crate) accounting_type: Option<AccountingTypeId>,
    /// A second organisation, the parent of the contact after its update.
    pub(crate) other_org: Option<ContactId>,
    pub(crate) other_category: Option<ContactCategoryId>,
    pub(crate) other_country: Option<CountryId>,
    pub(crate) other_address_category: Option<AddressCategoryId>,
    pub(crate) other_key: Option<CommunicationWayKeyId>,
}

/// Aborts unless `GET /SevClient` names the test account.
pub(crate) async fn verify_test_account(ctx: &Ctx) {
    let (status, text) = ctx.raw(Method::GET, "/SevClient", None, None).await;
    assert!(status < 300, "GET /SevClient answered {status}");
    let id = raw_one(&text)["id"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok());
    assert_eq!(
        id,
        Some(TEST_SEV_CLIENT),
        "not the test account: aborting before the write"
    );
}

/// The ids of `GET <path>` that are not in `skip`; read-only. An empty answer
/// makes the fields that need it untested.
async fn lookup_ids(ctx: &Ctx, path: &str, skip: &[i64], want: usize) -> Vec<i64> {
    let (status, text) = ctx.raw(Method::GET, path, None, None).await;
    let ids: Vec<i64> = (status < 300)
        .then(|| serde_json::from_str::<serde_json::Value>(&text).ok())
        .flatten()
        .and_then(|v| v["objects"].as_array().cloned())
        .map(|list| {
            list.iter()
                .filter_map(id_of)
                .filter(|i| !skip.contains(i))
                .take(want)
                .collect()
        })
        .unwrap_or_default();
    eprintln!("LOOKUP\t{path}\tHTTP {status}\t{ids:?}");
    ids
}

async fn one_id(ctx: &Ctx, path: &str, skip: &[i64]) -> Option<i64> {
    lookup_ids(ctx, path, skip, 1).await.first().copied()
}

async fn existing_customer(ctx: &Ctx) -> Customer {
    let found = ctx.c.list_contacts_by_name(CONTACT_PREFIX).await.unwrap();
    let contact = found
        .into_iter()
        .find(|c| {
            c.name
                .as_deref()
                .is_some_and(|n| n.starts_with(CONTACT_PREFIX))
        })
        .unwrap_or_else(|| panic!("no existing contact named `{CONTACT_PREFIX}...`: aborting"));
    let name = contact.name.clone().expect("the contact has a name");
    Customer {
        contact,
        name,
        street: STREET,
        zip: ZIP,
        city: CITY,
    }
}

/// An EXISTING enshrined invoice (read-only list) that no cancellation invoice
/// cancels and that is big enough for the credit note.
async fn existing_origin(ctx: &Ctx) -> Invoice {
    let query = InvoiceQuery {
        page: Some(Page::new(1000, 0).unwrap()),
        ..Default::default()
    };
    let all = ctx.c.list_invoices(&query).await.unwrap();
    all.iter()
        .find(|i| {
            i.invoice_type == Some(InvoiceType::Invoice)
                && i.enshrined.is_some()
                && matches!(i.status, Some(InvoiceStatus::Open | InvoiceStatus::Paid))
                && i.contact.is_some()
                && i.sum_gross
                    .is_some_and(|g| g.minor() >= CREDIT_NET * 119 / 100)
                && !all.iter().any(|other| other.cancels(i.id))
        })
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "no existing enshrined, uncancelled invoice among {}: aborting before the write",
                all.len()
            )
        })
}

impl Inputs {
    pub(crate) async fn read(ctx: &Ctx) -> Self {
        let methods = ctx.c.list_payment_methods().await.unwrap();
        let pm = methods
            .iter()
            .find(|m| m.electronic_invoice_id == Some(ONLINE_PAYMENT_XML_ID))
            .expect("a payment method with electronicInvoiceId 68")
            .id;
        let customer = existing_customer(ctx).await;
        let person = contact_person(ctx).await;
        let origin = existing_origin(ctx).await;
        let own = customer.contact.id.get();
        let document = one_id(ctx, "/Document?limit=20", &[]).await;
        let accounting_type = one_id(ctx, "/AccountingType?limit=20", &[]).await;
        let other_org = one_id(ctx, "/Contact?limit=20", &[own]).await;
        let other_category = one_id(ctx, "/Category?objectType=Contact&limit=100", &[3]).await;
        let other_country = one_id(ctx, "/StaticCountry?limit=20", &[GERMANY.get()]).await;
        let other_address_category = one_id(
            ctx,
            "/Category?objectType=ContactAddress&limit=100",
            &[BILLING_ADDRESS.get()],
        )
        .await;
        let other_key = one_id(ctx, "/CommunicationWayKey?limit=20", &[WORK_KEY.get()]).await;
        eprintln!(
            "SETUP contact {} payment method {} origin invoice {}",
            customer.contact.id.get(),
            pm.get(),
            origin.id
        );
        Self {
            customer,
            person,
            pm,
            origin,
            document: document.map(DocumentId::new),
            accounting_type: accounting_type.map(AccountingTypeId::new),
            other_org: other_org.map(ContactId::new),
            other_category: other_category.map(ContactCategoryId::new),
            other_country: other_country.map(CountryId::new),
            other_address_category: other_address_category.map(AddressCategoryId::new),
            other_key: other_key.map(CommunicationWayKeyId::new),
        }
    }
}
