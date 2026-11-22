//! Part 1, contact data and the transaction: each created with every option
//! set, then updated with every field of its `*Update` type.

use crate::Gate;
use crate::docs::*;
use crate::gate::expect_keys;
use crate::harness::{BANK, ZEIT, account, eur};
use sevdesk::*;
use time::macros::{date, datetime};

/// The contact, its address and its communication way, then the transaction;
/// returns the contact head and the address head as read back after their
/// updates, and the transaction head.
pub(crate) async fn all(
    g: &mut Gate<'_>,
) -> (serde_json::Value, serde_json::Value, serde_json::Value) {
    let (contact_id, contact, address) = contact_and_address(g).await;
    communication_way(g, contact_id).await;
    let transaction = transaction(g).await;
    (contact, address, transaction)
}

const NEW_CONTACT_KEYS: [&str; 24] = [
    "surename",
    "familyname",
    "category",
    "status",
    "buyerReference",
    "vatNumber",
    "customerNumber",
    "parent",
    "titel",
    "description",
    "academicTitle",
    "gender",
    "name2",
    "birthday",
    "bankAccount",
    "bankNumber",
    "defaultCashbackTime",
    "defaultCashbackPercent",
    "defaultTimeToPay",
    "taxNumber",
    "taxOffice",
    "exemptVat",
    "defaultDiscountAmount",
    "governmentAgency",
];

const CONTACT_UPDATE_KEYS: [&str; 25] = [
    "name",
    "surename",
    "familyname",
    "buyerReference",
    "vatNumber",
    "customerNumber",
    "category",
    "status",
    "parent",
    "titel",
    "description",
    "academicTitle",
    "gender",
    "name2",
    "birthday",
    "bankAccount",
    "bankNumber",
    "defaultCashbackTime",
    "defaultCashbackPercent",
    "defaultTimeToPay",
    "taxNumber",
    "taxOffice",
    "exemptVat",
    "defaultDiscountAmount",
    "governmentAgency",
];

async fn contact_and_address(
    g: &mut Gate<'_>,
) -> (ContactId, serde_json::Value, serde_json::Value) {
    let ctx = g.ctx;
    let n = ctx.marker.clone();
    let org = g.inp.customer.contact.id;
    let mut new = NewContact::new(ContactName::Person {
        surename: "Gate".into(),
        familyname: format!("Probe {n}"),
    });
    let o = &mut new.options;
    o.buyer_reference = Some("GATE-BR-1".into());
    o.vat_number = Some("DE123456789".into());
    o.customer_number = Some(format!("{n}-C"));
    o.status = Some(ContactStatus::Pending);
    o.parent = Some(ObjectRef::new(org));
    o.titel = Some("Prof.".into());
    o.description = Some("Gate contact description".into());
    o.academic_title = Some("Dr.".into());
    o.gender = Some("m".into());
    o.name2 = Some("Gatesecond".into());
    o.birthday = Some(date!(1985 - 03 - 17));
    o.bank_account = Some("DE89370400440532013000".into());
    o.bank_number = Some("37040044".into());
    o.default_cashback_time = Some(10);
    o.default_cashback_percent_bp = Some(250);
    o.default_time_to_pay = Some(21);
    o.tax_number = Some("12/345/67890".into());
    o.tax_office = Some("Gate Tax Office".into());
    o.exempt_vat = Some(true);
    o.default_discount_amount = Some(eur(1250));
    o.government_agency = Some(true);
    let sent = serde_json::to_value(&new).unwrap();
    expect_keys("create_contact", &sent, &NEW_CONTACT_KEYS, &[]);

    g.may_create();
    let created = match ctx.c.create_contact(&new).await {
        Ok(c) => c,
        Err(e) => g.create_failed("create_contact", &e),
    };
    let id = created.id;
    g.created("Contact", id.get());
    let path = format!("/Contact/{id}");
    let head = g
        .check("contact", "create_contact", &sent, None, &path, false)
        .await;

    let update = ContactUpdate {
        name: Some(format!("Gate Name {n}")),
        surename: Some("GateU".into()),
        familyname: Some(format!("Probe-U {n}")),
        buyer_reference: Some("GATE-BR-2".into()),
        vat_number: Some("DE987654321".into()),
        customer_number: Some(format!("{n}-CU")),
        category: Some(ObjectRef::new(
            g.inp.other_category.unwrap_or(CUSTOMER_CATEGORY),
        )),
        status: Some(ContactStatus::Active),
        parent: Some(ObjectRef::new(g.inp.other_org.unwrap_or(org))),
        titel: Some("Dipl.-Ing.".into()),
        description: Some("Gate contact description updated".into()),
        academic_title: Some("Prof. Dr.".into()),
        gender: Some("w".into()),
        name2: Some("Gatesecondupdated".into()),
        birthday: Some(date!(1990 - 07 - 23)),
        bank_account: Some("DE02120300000000202051".into()),
        bank_number: Some("12030000".into()),
        default_cashback_time: Some(14),
        default_cashback_percent_bp: Some(375),
        default_time_to_pay: Some(30),
        tax_number: Some("98/765/43210".into()),
        tax_office: Some("Gate Tax Office updated".into()),
        exempt_vat: Some(false),
        default_discount_amount: Some(eur(2575)),
        government_agency: Some(false),
    };
    let sent_u = serde_json::to_value(&update).unwrap();
    expect_keys("update_contact", &sent_u, &CONTACT_UPDATE_KEYS, &[]);
    let result = ctx.c.update_contact(id, &update).await;
    let head_u = g
        .update_step("contact", "update_contact", sent_u, &head, &path, result)
        .await;

    // The address.
    let mut new_address = NewContactAddress {
        contact: ObjectRef::new(id),
        street: "Gateaddressstrasse 1".into(),
        zip: "10117".into(),
        city: "Gateaddresscity".into(),
        country: ObjectRef::new(GERMANY),
        category: ObjectRef::new(BILLING_ADDRESS),
        options: Default::default(),
    };
    new_address.options.name = Some(format!("Gate Address {n}"));
    let sent = serde_json::to_value(&new_address).unwrap();
    expect_keys(
        "create_contact_address",
        &sent,
        &[
            "contact", "street", "zip", "city", "country", "category", "name",
        ],
        &[],
    );
    g.may_create();
    let address = match ctx.c.create_contact_address(&new_address).await {
        Ok(a) => a,
        Err(e) => g.create_failed("create_contact_address", &e),
    };
    g.created("ContactAddress", address.id.get());
    let path = format!("/ContactAddress/{}", address.id);
    let head = g
        .check(
            "contact_address",
            "create_contact_address",
            &sent,
            None,
            &path,
            false,
        )
        .await;
    let update = ContactAddressUpdate {
        street: Some("Gateaddressupdatedstrasse 2".into()),
        zip: Some("80331".into()),
        city: Some("Gateupdatedcity".into()),
        country: Some(ObjectRef::new(g.inp.other_country.unwrap_or(GERMANY))),
        category: Some(ObjectRef::new(
            g.inp.other_address_category.unwrap_or(BILLING_ADDRESS),
        )),
        name: Some(format!("Gate Address Updated {n}")),
    };
    let sent_u = serde_json::to_value(&update).unwrap();
    expect_keys(
        "update_contact_address",
        &sent_u,
        &["street", "zip", "city", "country", "category", "name"],
        &[],
    );
    let result = ctx.c.update_contact_address(address.id, &update).await;
    let address_u = g
        .update_step(
            "contact_address",
            "update_contact_address",
            sent_u,
            &head,
            &path,
            result,
        )
        .await;
    g.rec.flush("contact");
    g.rec.flush("contact_address");
    (id, head_u, address_u)
}

async fn communication_way(g: &mut Gate<'_>, contact: ContactId) {
    let ctx = g.ctx;
    let local = ctx.marker.to_lowercase();
    let new = NewCommunicationWay::email(contact, format!("{local}@example.com"), WORK_KEY, true);
    let sent = serde_json::to_value(&new).unwrap();
    expect_keys(
        "create_communication_way",
        &sent,
        &["contact", "type", "value", "key", "main"],
        &[],
    );
    g.may_create();
    let way = match ctx.c.create_communication_way(&new).await {
        Ok(w) => w,
        Err(e) => g.create_failed("create_communication_way", &e),
    };
    g.created("CommunicationWay", way.id.get());
    let path = format!("/CommunicationWay/{}", way.id);
    let head = g
        .check(
            "communication_way",
            "create_communication_way",
            &sent,
            None,
            &path,
            false,
        )
        .await;
    let update = CommunicationWayUpdate {
        value: Some("+493012345678".into()),
        key: Some(ObjectRef::new(g.inp.other_key.unwrap_or(WORK_KEY))),
        main: Some(false),
        kind: Some(CommunicationWayType::Phone),
    };
    let sent_u = serde_json::to_value(&update).unwrap();
    expect_keys(
        "update_communication_way",
        &sent_u,
        &["value", "key", "main", "type"],
        &[],
    );
    let result = ctx.c.update_communication_way(way.id, &update).await;
    g.update_step(
        "communication_way",
        "update_communication_way",
        sent_u,
        &head,
        &path,
        result,
    )
    .await;
    g.rec.flush("communication_way");
}

const NEW_TRANSACTION_KEYS: [&str; 9] = [
    "valueDate",
    "amount",
    "payeePayerName",
    "status",
    "checkAccount",
    "entryDate",
    "paymtPurpose",
    "payeePayerAcctNo",
    "payeePayerBankCode",
];

async fn transaction(g: &mut Gate<'_>) -> serde_json::Value {
    let ctx = g.ctx;
    let n = ctx.marker.clone();
    for field in ["sourceTransaction", "targetTransaction"] {
        eprintln!(
            "UNTESTED\tcreate_transaction/update_transaction\t{field}\twould link foreign transactions"
        );
    }
    let mut new = NewTransaction {
        value_date: datetime!(2026-11-05 12:00 +1),
        amount: eur(5000),
        payee_payer_name: format!("Gate Payee {n}"),
        status: TransactionStatus::Created,
        check_account: ObjectRef::new(account(ZEIT)),
        options: Default::default(),
    };
    let o = &mut new.options;
    o.entry_date = Some(datetime!(2026-11-06 12:00 +1));
    o.paymt_purpose = Some(format!("{n}-purpose"));
    o.payee_payer_acct_no = Some("DE89370400440532013000".into());
    o.payee_payer_bank_code = Some("COBADEFFXXX".into());
    let sent = serde_json::to_value(&new).unwrap();
    expect_keys("create_transaction", &sent, &NEW_TRANSACTION_KEYS, &[]);
    g.may_create();
    let tx = match ctx.c.create_transaction(&new).await {
        Ok(t) => t,
        Err(e) => g.create_failed("create_transaction", &e),
    };
    g.created("CheckAccountTransaction", tx.id.get());
    let path = format!("/CheckAccountTransaction/{}", tx.id);
    let head = g
        .check(
            "transaction",
            "create_transaction",
            &sent,
            None,
            &path,
            false,
        )
        .await;
    let update = TransactionUpdate {
        status: Some(TransactionStatus::Private),
        value_date: Some(datetime!(2026-11-12 12:00 +1)),
        entry_date: Some(datetime!(2026-11-13 12:00 +1)),
        paymt_purpose: Some(format!("{n}-purpose-U")),
        amount: Some(eur(7777)),
        payee_payer_name: Some(format!("Gate Payee Updated {n}")),
        payee_payer_acct_no: Some("DE02120300000000202051".into()),
        payee_payer_bank_code: Some("BYLADEM1001".into()),
        check_account: Some(ObjectRef::new(account(BANK))),
        ..Default::default()
    };
    let sent_u = serde_json::to_value(&update).unwrap();
    expect_keys(
        "update_transaction",
        &sent_u,
        &[
            "status",
            "valueDate",
            "entryDate",
            "paymtPurpose",
            "amount",
            "payeePayerName",
            "payeePayerAcctNo",
            "payeePayerBankCode",
            "checkAccount",
        ],
        &[],
    );
    let result = ctx.c.update_transaction(tx.id, &update).await;
    let head_u = g
        .update_step(
            "transaction",
            "update_transaction",
            sent_u,
            &head,
            &path,
            result,
        )
        .await;
    g.rec.flush("transaction");
    head_u
}
