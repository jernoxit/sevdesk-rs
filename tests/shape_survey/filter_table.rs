//! The filter table of `filters_all`: every list filter the OpenAPI spec names for
//! the five list endpoints the crate has, plus the filters the crate already sends
//! (controls, `from_crate`). Generated once from the spec's `parameters` and the
//! "How to filter ..." tables of the tag descriptions; the probe needs no YAML.
//! Spelling is the spec's, typos included (`customerIntenalNote`,
//! `creditNoteNumberNumberSmaller`); each typo row is followed by the corrected
//! spelling as an extra row.
//!
//! `field` is the object field the filter presumably acts on (`a|b`: any of
//! several; empty: none). `spec_type` is the spec's type/format or, for filters
//! that only appear in the prose table, the form the prose shows.

pub(crate) const INVOICE: &str = "Invoice";
pub(crate) const CREDIT_NOTE: &str = "CreditNote";
pub(crate) const VOUCHER: &str = "Voucher";
pub(crate) const CONTACT: &str = "Contact";
pub(crate) const TRANSACTION: &str = "CheckAccountTransaction";

pub(crate) const ENDPOINTS: &[&str] = &[INVOICE, CREDIT_NOTE, VOUCHER, CONTACT, TRANSACTION];

#[derive(Clone, Copy)]
pub(crate) enum DateOp {
    Ge,
    Gt,
    Le,
    Lt,
    Day,
}

#[derive(Clone, Copy)]
pub(crate) enum ViaMode {
    Exact,
    /// A reference; the `objectName` to send.
    Ref(&'static str),
    Text,
}

/// How a filter is exercised and judged.
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    /// Value = exact copy of the field; judged by equality / containment.
    Exact,
    /// Value = strict substring of the field.
    Text,
    /// Value = strict prefix of the field.
    Prefix,
    /// `x[id]=..&x[objectName]=..` with an id found in the field (`{id, objectName}`);
    /// the argument is the `objectName` to send when the data carries none.
    Ref(&'static str),
    Date(DateOp),
    /// Year of a date field.
    Year,
    /// Month (1..=12) of a date field.
    Month,
    /// A flag value (`true`, `all`, ...); judged by how the result set changes.
    Flag(&'static str),
    /// Ordering; the values to send. Judged by the order of the answer.
    Order(&'static [&'static str]),
    /// `startAmount` (true: lower bound) / `endAmount`.
    Amount(bool),
    /// Number comparison on the trailing integer (true: `...Greater`).
    NumCmp(bool),
    /// `onlyCredit` (true) / `onlyDebit`: the sign of the field.
    Sign(bool),
    /// Text value without a per-object predicate: only narrowing is judged.
    Narrow,
    /// A reference whose ids are not in the list objects: ids from `GET <list>`
    /// (the first three are tried), only narrowing is judged. (list, objectName)
    NarrowRef(&'static str, &'static str),
    /// Expected objects derived from a linked list: (list path, link field in
    /// that list, mode); `field` is the path inside the linked objects.
    Via(&'static str, &'static str, ViaMode),
    /// Not exercised: no suitable value or no way to judge; the reason.
    Absent(&'static str),
}

pub(crate) struct Spec {
    pub(crate) endpoint: &'static str,
    pub(crate) filter: &'static str,
    pub(crate) spec_type: &'static str,
    pub(crate) field: &'static str,
    pub(crate) kind: Kind,
    pub(crate) from_crate: bool,
}

const fn spec(
    endpoint: &'static str,
    filter: &'static str,
    spec_type: &'static str,
    field: &'static str,
    kind: Kind,
) -> Spec {
    Spec {
        endpoint,
        filter,
        spec_type,
        field,
        kind,
        from_crate: false,
    }
}

/// A filter the crate already sends (control row).
const fn ctl(
    endpoint: &'static str,
    filter: &'static str,
    spec_type: &'static str,
    field: &'static str,
    kind: Kind,
) -> Spec {
    Spec {
        from_crate: true,
        ..spec(endpoint, filter, spec_type, field, kind)
    }
}

use DateOp::{Day, Ge, Gt, Le, Lt};
use Kind::*;

const TS: &str = "timestamp (prose)";
const TS_INT: &str = "integer";
const TRUE: &str = "true (prose)";
const ASC_DESC: &[&str] = &["ASC", "DESC"];
const ORDER_TRUE: &[&str] = &["true"];
const TAG: Kind = NarrowRef("/Tag", "Tag");
const COST_CENTRE: Kind = NarrowRef("/CostCentre", "CostCentre");
const STATUS_CODE: &str = "status";

#[rustfmt::skip]
pub(crate) const SPECS: &[Spec] = &[
    // ---- GET /Invoice (33 spec filters; 4 sent by the crate) ----
    ctl(INVOICE, "customerInternalNote", "string (prose: customerIntenalNote)", "customerInternalNote", Exact),
    spec(INVOICE, "customerIntenalNote", "string (the spec's typo)", "customerInternalNote", Exact),
    ctl(INVOICE, "invoiceNumber", "string", "invoiceNumber", Exact),
    ctl(INVOICE, "status", "number enum 100|200|1000", STATUS_CODE, Exact),
    ctl(INVOICE, "updateAfter", TS, "update", Date(Gt)),
    spec(INVOICE, "startDate", TS_INT, "invoiceDate", Date(Ge)),
    spec(INVOICE, "endDate", TS_INT, "invoiceDate", Date(Le)),
    spec(INVOICE, "contact[id]", TS_INT, "contact", Ref("Contact")),
    spec(INVOICE, "partiallyPaid", TRUE, STATUS_CODE, Flag("true")),
    spec(INVOICE, "orderByDebit", TRUE, "", Order(ORDER_TRUE)),
    spec(INVOICE, "orderByDueTime", TRUE, "", Order(ORDER_TRUE)),
    spec(INVOICE, "showAll", TRUE, "invoiceType", Flag("true")),
    spec(INVOICE, "delinquent", TRUE, STATUS_CODE, Flag("true")),
    spec(INVOICE, "notdelinquent", TRUE, STATUS_CODE, Flag("true")),
    spec(INVOICE, "tags[x][id]", "tags[x][id]&tags[x][objectName]=Tag (prose)", "", TAG),
    spec(INVOICE, "costCentre[id]", "costCentre[id]&costCentre[objectName]=CostCentre (prose)", "", COST_CENTRE),
    spec(INVOICE, "createBefore", TS, "create", Date(Lt)),
    spec(INVOICE, "createAfter", TS, "create", Date(Gt)),
    spec(INVOICE, "updateBefore", TS, "update", Date(Lt)),
    spec(INVOICE, "orderByDueDate", TRUE, "", Order(ORDER_TRUE)),
    spec(INVOICE, "day", TS, "invoiceDate", Date(Day)),
    spec(INVOICE, "onlyDunned", TRUE, "dunningLevel", Flag("true")),
    spec(INVOICE, "showWkr", "all (prose)", "invoiceType", Flag("all")),
    spec(INVOICE, "showMa", "all (prose)", "invoiceType", Flag("all")),
    spec(INVOICE, "origin[id]", "origin[id]&origin[objectName] (prose)", "origin", Ref("")),
    spec(INVOICE, "invoiceType", "string (prose)", "invoiceType", Exact),
    spec(INVOICE, "paymentMethod[id]", "paymentMethod[id]&paymentMethod[objectName]=PaymentMethod (prose)", "paymentMethod", Ref("PaymentMethod")),
    spec(INVOICE, "headerStartsWith", "string (prose)", "header", Prefix),
    spec(INVOICE, "headerOrNumber", "string (prose)", "header|invoiceNumber", Text),
    spec(INVOICE, "orderByInvoiceNumber", "ASC|DESC (prose)", "invoiceNumber", Order(ASC_DESC)),
    spec(INVOICE, "invoiceNumberGreater", "number (prose)", "invoiceNumber", NumCmp(true)),
    spec(INVOICE, "invoiceNumberSmaller", "number (prose)", "invoiceNumber", NumCmp(false)),
    spec(INVOICE, "sendType", "string (prose)", "sendType", Exact),
    spec(INVOICE, "fulltextSearch", "string (prose)", "header", Narrow),
    // ---- GET /CreditNote (25 spec filters; 2 sent by the crate) ----
    ctl(CREDIT_NOTE, "customerInternalNote", "string (prose: customerIntenalNote)", "customerInternalNote", Exact),
    spec(CREDIT_NOTE, "customerIntenalNote", "string (the spec's typo)", "customerInternalNote", Exact),
    ctl(CREDIT_NOTE, "creditNoteNumber", "string", "creditNoteNumber", Exact),
    spec(CREDIT_NOTE, "status", "string enum 100|200|300|500|750|1000", STATUS_CODE, Exact),
    spec(CREDIT_NOTE, "startDate", TS_INT, "creditNoteDate", Date(Ge)),
    spec(CREDIT_NOTE, "endDate", TS_INT, "creditNoteDate", Date(Le)),
    spec(CREDIT_NOTE, "contact[id]", TS_INT, "contact", Ref("Contact")),
    spec(CREDIT_NOTE, "onlyEnshrined", TRUE, "enshrined", Flag("true")),
    spec(CREDIT_NOTE, "tags[x][id]", "tags[x][id]&tags[x][objectName]=Tag (prose)", "", TAG),
    spec(CREDIT_NOTE, "delinquent", TRUE, STATUS_CODE, Flag("true")),
    spec(CREDIT_NOTE, "notdelinquent", TRUE, STATUS_CODE, Flag("true")),
    spec(CREDIT_NOTE, "origin[id]", "origin[id]&origin[objectName] (prose)", "origin", Ref("")),
    spec(CREDIT_NOTE, "costCentre[id]", "costCentre[id]&costCentre[objectName]=CostCentre (prose)", "", COST_CENTRE),
    spec(CREDIT_NOTE, "day", TS, "creditNoteDate", Date(Day)),
    spec(CREDIT_NOTE, "paymentMethod[id]", "paymentMethod[id]&paymentMethod[objectName]=PaymentMethod (prose)", "paymentMethod", Ref("PaymentMethod")),
    spec(CREDIT_NOTE, "headerOrNumber", "string (prose)", "header|creditNoteNumber", Text),
    spec(CREDIT_NOTE, "headerStartsWith", "string (prose)", "header", Prefix),
    spec(CREDIT_NOTE, "orderByCreditNoteNumber", "ASC|DESC (prose)", "creditNoteNumber", Order(ASC_DESC)),
    spec(CREDIT_NOTE, "partiallyPaid", TRUE, STATUS_CODE, Flag("true")),
    spec(CREDIT_NOTE, "orderByDueDate", TRUE, "", Order(ORDER_TRUE)),
    spec(CREDIT_NOTE, "orderByDueTime", TRUE, "", Order(ORDER_TRUE)),
    spec(CREDIT_NOTE, "orderByDebit", TRUE, "", Order(ORDER_TRUE)),
    spec(CREDIT_NOTE, "creditNoteNumberGreater", "number (prose)", "creditNoteNumber", NumCmp(true)),
    spec(CREDIT_NOTE, "creditNoteNumberNumberSmaller", "number (the spec's spelling)", "creditNoteNumber", NumCmp(false)),
    spec(CREDIT_NOTE, "creditNoteNumberSmaller", "number (corrected spelling, not in the spec)", "creditNoteNumber", NumCmp(false)),
    spec(CREDIT_NOTE, "startAmount", "amount (prose)", "sumGross|sumNet", Amount(true)),
    spec(CREDIT_NOTE, "endAmount", "amount (prose)", "sumGross|sumNet", Amount(false)),
    // ---- GET /Voucher (34 spec filters; 2 sent by the crate) ----
    ctl(VOUCHER, "descriptionLike", "string", "description", Text),
    ctl(VOUCHER, "status", "number enum 50|100|1000", STATUS_CODE, Exact),
    spec(VOUCHER, "creditDebit", "string enum C|D", "creditDebit", Exact),
    spec(VOUCHER, "startDate", TS_INT, "voucherDate", Date(Ge)),
    spec(VOUCHER, "endDate", TS_INT, "voucherDate", Date(Le)),
    spec(VOUCHER, "contact[id]", TS_INT, "", NarrowRef("/Contact", "Contact")),
    spec(VOUCHER, "accountingType[id]", "accountingType[id]&accountingType[objectName]=AccountingType (prose)", "accountingType.id", Via("/VoucherPos", "voucher", ViaMode::Ref("AccountingType"))),
    spec(VOUCHER, "withoutCatering", TRUE, "voucherType", Flag("true")),
    spec(VOUCHER, "year", "year (prose)", "voucherDate", Year),
    spec(VOUCHER, "month", "month (prose)", "voucherDate", Month),
    spec(VOUCHER, "supplierName", "string (prose)", "supplierName", Text),
    spec(VOUCHER, "commentLike", "string (prose)", "comment", Via("/VoucherPos", "voucher", ViaMode::Text)),
    spec(VOUCHER, "searchCommentOrDescription", "string (prose)", "description", Narrow),
    spec(VOUCHER, "createBefore", TS, "create", Date(Lt)),
    spec(VOUCHER, "createAfter", TS, "create", Date(Gt)),
    spec(VOUCHER, "updateBefore", TS, "update", Date(Lt)),
    spec(VOUCHER, "updateAfter", TS, "update", Date(Gt)),
    spec(VOUCHER, "object[id]", "object[id]&object[objectName] (prose)", "", Absent("the linked object has no field in the list objects and no list to draw ids from")),
    spec(VOUCHER, "startPayDate", TS, "payDate", Date(Ge)),
    spec(VOUCHER, "endPayDate", TS, "payDate", Date(Le)),
    spec(VOUCHER, "orderByDebit", TRUE, "", Order(ORDER_TRUE)),
    spec(VOUCHER, "costCentre[id]", "costCentre[id]&costCentre[objectName]=CostCentre (prose)", "", COST_CENTRE),
    spec(VOUCHER, "voucherType", "string (prose)", "voucherType", Exact),
    spec(VOUCHER, "origin[id]", "origin[id]&origin[objectName] (prose)", "", Absent("vouchers carry no origin field and there is no list to draw ids from")),
    spec(VOUCHER, "contactOrObject[id]", "contactOrObject[id]&contactOrObject[objectName] (prose)", "", NarrowRef("/Contact", "Contact")),
    spec(VOUCHER, "orderByVoucherNumber", "ASC|DESC (prose)", "", Order(ASC_DESC)),
    spec(VOUCHER, "delinquent", TRUE, STATUS_CODE, Flag("true")),
    spec(VOUCHER, "hasDocument", TRUE, "document", Flag("true")),
    spec(VOUCHER, "noRv", TRUE, "voucherType", Flag("true")),
    spec(VOUCHER, "tags[x][id]", "tags[x][id]&tags[x][objectName]=Tag (prose)", "", TAG),
    spec(VOUCHER, "startAmount", "amount (prose)", "sumGross|sumNet", Amount(true)),
    spec(VOUCHER, "endAmount", "amount (prose)", "sumGross|sumNet", Amount(false)),
    spec(VOUCHER, "fulltextSearch", "string (prose)", "description|supplierName", Narrow),
    spec(VOUCHER, "fulltextSearchObjects", "list of object names (prose)", "", Absent("a modifier of fulltextSearch: no standalone effect to measure")),
    // ---- GET /Contact (14 spec filters; 2 sent by the crate) ----
    ctl(CONTACT, "name", "string (prose)", "name", Text),
    ctl(CONTACT, "customerNumber", "string", "customerNumber", Exact),
    spec(CONTACT, "depth", "string enum 0|1", "", Flag("1")),
    spec(CONTACT, "category[id]", "category[id]&category[objectName]=Category (prose)", "category", Ref("Category")),
    spec(CONTACT, "city", "string (prose)", "city", Via("/ContactAddress", "contact", ViaMode::Exact)),
    spec(CONTACT, "tags[x][id]", "tags[x][id]&tags[x][objectName]=Tag (prose)", "", TAG),
    spec(CONTACT, "parent[id]", "parent[id]&parent[objectName]=Contact (prose)", "", NarrowRef("/Contact", "Contact")),
    spec(CONTACT, "zip", "string (prose)", "zip", Via("/ContactAddress", "contact", ViaMode::Exact)),
    spec(CONTACT, "country[id]", "country[id]&country[objectName]=StaticCountry (prose)", "country.id", Via("/ContactAddress", "contact", ViaMode::Ref("StaticCountry"))),
    spec(CONTACT, "createBefore", TS, "create", Date(Lt)),
    spec(CONTACT, "createAfter", TS, "create", Date(Gt)),
    spec(CONTACT, "updateBefore", TS, "update", Date(Lt)),
    spec(CONTACT, "updateAfter", TS, "update", Date(Gt)),
    spec(CONTACT, "orderByCustomerNumber", "ASC|DESC (prose)", "customerNumber", Order(ASC_DESC)),
    // ---- GET /CheckAccountTransaction (8 spec filters; 5 sent by the crate, plus status) ----
    ctl(TRANSACTION, "checkAccount[id]", "integer", "checkAccount", Ref("CheckAccount")),
    ctl(TRANSACTION, "isBooked", "boolean", STATUS_CODE, Flag("true")),
    ctl(TRANSACTION, "isBooked", "boolean (value false)", STATUS_CODE, Flag("false")),
    ctl(TRANSACTION, "paymtPurpose", "string", "paymtPurpose", Text),
    ctl(TRANSACTION, "startDate", "string date-time", "entryDate|valueDate", Date(Ge)),
    ctl(TRANSACTION, "endDate", "string date-time", "entryDate|valueDate", Date(Le)),
    ctl(TRANSACTION, "status", "not in the spec (measured)", STATUS_CODE, Exact),
    spec(TRANSACTION, "payeePayerName", "string", "payeePayerName", Exact),
    spec(TRANSACTION, "onlyCredit", "boolean", "amount", Sign(true)),
    spec(TRANSACTION, "onlyDebit", "boolean", "amount", Sign(false)),
];
