//! The field table of the probe: every writable field the spec names that the
//! crate does not send today (`docs/schreibtypen-protokoll.md`, generated once
//! from the spec, committed here so the probe needs no YAML).
//!
//! Candidate values are JSON text with tokens the runner substitutes:
//! `@dmy@` (`20.11.2026`), `@unix@` (Unix seconds of 2026-11-20 12:00 UTC),
//! `@rfc@` (`2026-11-20T12:00:00+01:00`), all three the same calendar day in
//! every zone; `@marker@` (the run marker); `@self@` (a reference to the probed
//! object itself); `@ref:<Object>@` (a reference to an object found read-only in
//! the account, or the field is reported untestable). A candidate's `extra` is a
//! JSON object merged next to the field (a field that only counts together with
//! another one).

/// How a read-back value is compared with the sent one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    /// Trimmed string equality.
    Str,
    /// Decimal text or JSON number, trailing zeros ignored.
    Num,
    /// JSON bool, `"1"`/`"0"`, `1`/`0`.
    Bool,
    /// The calendar day, from `dd.mm.yyyy`, ISO, RFC 3339 or Unix seconds.
    Date,
    /// The `id` of a `{id, objectName}` reference.
    Ref,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Scope {
    Head,
    Pos,
}

/// A field either is measured in the normal pass or, as a state field, after
/// all others on its object.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Phase {
    Normal,
    State,
}

pub(crate) struct Candidate {
    pub(crate) encoding: &'static str,
    pub(crate) value: &'static str,
    pub(crate) extra: &'static str,
}

pub(crate) struct Row {
    /// Target key: `invoice`, `credit_note`, `voucher`, `contact`,
    /// `contact_address`, `communication_way`, `transaction`.
    pub(crate) object: &'static str,
    pub(crate) scope: Scope,
    pub(crate) phase: Phase,
    pub(crate) field: &'static str,
    /// The spec's type and format.
    pub(crate) spec: &'static str,
    pub(crate) kind: Kind,
    pub(crate) candidates: &'static [Candidate],
}

const fn c(encoding: &'static str, value: &'static str) -> Candidate {
    Candidate {
        encoding,
        value,
        extra: "",
    }
}

const fn row(
    object: &'static str,
    scope: Scope,
    field: &'static str,
    spec: &'static str,
    kind: Kind,
    candidates: &'static [Candidate],
) -> Row {
    Row {
        object,
        scope,
        phase: Phase::Normal,
        field,
        spec,
        kind,
        candidates,
    }
}

const fn state(
    object: &'static str,
    field: &'static str,
    spec: &'static str,
    kind: Kind,
    candidates: &'static [Candidate],
) -> Row {
    Row {
        object,
        scope: Scope::Head,
        phase: Phase::State,
        field,
        spec,
        kind,
        candidates,
    }
}

use Kind::{Bool, Date, Num, Ref, Str};
use Scope::{Head, Pos};

/// Document dates: German text first (the crate's own form), then Unix seconds,
/// then RFC 3339.
const DOC_DATES: &[Candidate] = &[
    c("dd.mm.yyyy string", "\"@dmy@\""),
    c("unix seconds integer", "@unix@"),
    c("RFC 3339 string", "\"@rfc@\""),
];
/// Transaction dates: RFC 3339 first (the crate's own form).
const TX_DATES: &[Candidate] = &[
    c("RFC 3339 string", "\"@rfc@\""),
    c("dd.mm.yyyy string", "\"@dmy@\""),
    c("unix seconds integer", "@unix@"),
];
const BOOL: &[Candidate] = &[c("json bool", "true"), c("string 1", "\"1\"")];
const TAX_TYPE: &[Candidate] = &[c("string default", "\"default\""), c("string eu", "\"eu\"")];
const ADDRESS: &[Candidate] = &[c(
    "string, newline separated",
    "\"SDRS Probe\\nProbestrasse 9\\n10115 Berlin\\nGermany\"",
)];
const SELF_REF: &[Candidate] = &[c("reference object", "@self@")];

pub(crate) const ROWS: &[Row] = &[
    // saveInvoice: invoice head, written with `PUT /Invoice/{id}`.
    row("invoice", Head, "address", "string", Str, ADDRESS),
    row("invoice", Head, "smallSettlement", "boolean", Bool, BOOL),
    row(
        "invoice",
        Head,
        "taxType",
        "string [default/eu/noteu/custom]",
        Str,
        TAX_TYPE,
    ),
    row(
        "invoice",
        Head,
        "taxSet",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:TaxSet@")],
    ),
    row(
        "invoice",
        Head,
        "origin",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:Order@")],
    ),
    // saveInvoice: position, written through the factory with the position id.
    row(
        "invoice",
        Pos,
        "part",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:Part@")],
    ),
    row(
        "invoice",
        Pos,
        "positionNumber",
        "integer",
        Num,
        &[c("integer", "3")],
    ),
    row(
        "invoice",
        Pos,
        "discount",
        "number:float",
        Num,
        &[c("decimal string", "\"10\""), c("json number", "10")],
    ),
    row(
        "invoice",
        Pos,
        "priceGross",
        "number:float",
        Num,
        &[c("decimal string", "\"23.80\""), c("json number", "23.8")],
    ),
    row(
        "invoice",
        Pos,
        "priceTax",
        "number:float",
        Num,
        &[c("decimal string", "\"3.80\""), c("json number", "3.8")],
    ),
    // State fields of the invoice, in the order they are measured.
    state("invoice", "sendDate", "string:date-time", Date, DOC_DATES),
    state("invoice", "payDate", "string:date-time", Date, DOC_DATES),
    state(
        "invoice",
        "sendType",
        "string [VPR/VPDF/VM/VP]",
        Str,
        &[c("string VPDF", "\"VPDF\""), c("string VM", "\"VM\"")],
    ),
    state(
        "invoice",
        "invoiceNumber",
        "string",
        Str,
        &[c("string with run marker", "\"@marker@-NR\"")],
    ),
    state(
        "invoice",
        "status",
        "string [50/100/200/750/1000]",
        Num,
        &[c("string 200", "\"200\""), c("integer 200", "200")],
    ),
    // saveCreditNote: head, written with `PUT /CreditNote/{id}`.
    row("credit_note", Head, "address", "string", Str, ADDRESS),
    row(
        "credit_note",
        Head,
        "smallSettlement",
        "boolean",
        Bool,
        BOOL,
    ),
    row("credit_note", Head, "taxType", "string", Str, TAX_TYPE),
    row(
        "credit_note",
        Head,
        "taxSet",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:TaxSet@")],
    ),
    // saveCreditNote: position, written through the factory.
    row(
        "credit_note",
        Pos,
        "creditNote",
        "Ref{id,objectName}",
        Ref,
        SELF_REF,
    ),
    row(
        "credit_note",
        Pos,
        "part",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:Part@")],
    ),
    row(
        "credit_note",
        Pos,
        "priceTax",
        "number:float",
        Num,
        &[c("decimal string", "\"1.50\""), c("json number", "1.5")],
    ),
    row(
        "credit_note",
        Pos,
        "priceGross",
        "number:float",
        Num,
        &[c("decimal string", "\"9.50\""), c("json number", "9.5")],
    ),
    row(
        "credit_note",
        Pos,
        "positionNumber",
        "integer",
        Num,
        &[c("integer", "3")],
    ),
    row(
        "credit_note",
        Pos,
        "discount",
        "number:float",
        Num,
        &[c("decimal string", "\"10\""), c("json number", "10")],
    ),
    row("credit_note", Pos, "optional", "boolean", Bool, BOOL),
    state(
        "credit_note",
        "sendDate",
        "string:date-time",
        Date,
        DOC_DATES,
    ),
    state(
        "credit_note",
        "sendType",
        "string [VPR/VPDF/VM/VP]",
        Str,
        &[c("string VPDF", "\"VPDF\""), c("string VM", "\"VM\"")],
    ),
    state(
        "credit_note",
        "status",
        "string [100/200/300/500/750/1000]",
        Num,
        &[c("string 200", "\"200\""), c("integer 200", "200")],
    ),
    // saveVoucher: head and position both through the factory.
    row(
        "voucher",
        Head,
        "propertyForeignCurrencyDeadline",
        "string:date-time",
        Date,
        DOC_DATES,
    ),
    row(
        "voucher",
        Head,
        "propertyExchangeRate",
        "number:float",
        Num,
        &[c("decimal string", "\"1.25\""), c("json number", "1.25")],
    ),
    row(
        "voucher",
        Head,
        "taxSet",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:TaxSet@")],
    ),
    row("voucher", Head, "taxType", "string", Str, TAX_TYPE),
    row(
        "voucher",
        Head,
        "paymentDeadline",
        "string:date-time",
        Date,
        DOC_DATES,
    ),
    row(
        "voucher",
        Head,
        "document",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:Document@")],
    ),
    row(
        "voucher",
        Head,
        "costCentre",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:CostCentre@")],
    ),
    row(
        "voucher",
        Pos,
        "accountingType",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:AccountingType@")],
    ),
    row("voucher", Pos, "isAsset", "boolean", Bool, BOOL),
    row(
        "voucher",
        Pos,
        "sumGross",
        "number:float",
        Num,
        &[
            c("decimal string, net stays true", "\"0.42\""),
            Candidate {
                encoding: "decimal string with net=false",
                value: "\"0.42\"",
                extra: "{\"net\":false}",
            },
        ],
    ),
    state("voucher", "payDate", "string:date-time", Date, DOC_DATES),
    // createContact / updateContact: a person, so the person-only fields count.
    row(
        "contact",
        Head,
        "parent",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:Contact@")],
    ),
    row(
        "contact",
        Head,
        "titel",
        "string",
        Str,
        &[c("string", "\"Probe-Titel\"")],
    ),
    row(
        "contact",
        Head,
        "category",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object", "@ref:Category@")],
    ),
    row(
        "contact",
        Head,
        "description",
        "string",
        Str,
        &[c("string", "\"probe description\"")],
    ),
    row(
        "contact",
        Head,
        "academicTitle",
        "string",
        Str,
        &[c("string", "\"Dr.\"")],
    ),
    row(
        "contact",
        Head,
        "gender",
        "string",
        Str,
        &[
            c("string m", "\"m\""),
            c("string w", "\"w\""),
            c("string male", "\"male\""),
        ],
    ),
    row(
        "contact",
        Head,
        "name2",
        "string",
        Str,
        &[c("string", "\"Probe-Name2\"")],
    ),
    row(
        "contact",
        Head,
        "birthday",
        "string:date",
        Date,
        &[
            c("ISO date string", "\"1990-05-17\""),
            c("dd.mm.yyyy string", "\"17.05.1990\""),
        ],
    ),
    row(
        "contact",
        Head,
        "bankAccount",
        "string",
        Str,
        &[c("string IBAN", "\"DE02120300000000202051\"")],
    ),
    row(
        "contact",
        Head,
        "bankNumber",
        "string",
        Str,
        &[c("string", "\"12030000\"")],
    ),
    row(
        "contact",
        Head,
        "defaultCashbackTime",
        "integer",
        Num,
        &[c("integer", "14"), c("decimal string", "\"14\"")],
    ),
    row(
        "contact",
        Head,
        "defaultCashbackPercent",
        "number:float",
        Num,
        &[c("decimal string", "\"2.5\""), c("json number", "2.5")],
    ),
    row(
        "contact",
        Head,
        "defaultTimeToPay",
        "integer",
        Num,
        &[c("integer", "21"), c("decimal string", "\"21\"")],
    ),
    row(
        "contact",
        Head,
        "taxNumber",
        "string",
        Str,
        &[c("string", "\"12/345/67890\"")],
    ),
    row(
        "contact",
        Head,
        "taxOffice",
        "string",
        Str,
        &[c("string", "\"Finanzamt Probe\"")],
    ),
    row("contact", Head, "exemptVat", "boolean", Bool, BOOL),
    row(
        "contact",
        Head,
        "defaultDiscountAmount",
        "number:float",
        Num,
        &[c("decimal string", "\"5\""), c("json number", "5")],
    ),
    row(
        "contact",
        Head,
        "defaultDiscountPercentage",
        "boolean",
        Bool,
        BOOL,
    ),
    row("contact", Head, "governmentAgency", "boolean", Bool, BOOL),
    state(
        "contact",
        "status",
        "integer [100/500/1000]",
        Num,
        &[c("integer 500", "500"), c("string 500", "\"500\"")],
    ),
    // createContactAddress / updateContactAddress.
    row(
        "contact_address",
        Head,
        "name",
        "string",
        Str,
        &[c("string", "\"Probe Name\"")],
    ),
    row(
        "contact_address",
        Head,
        "name2",
        "string",
        Str,
        &[c("string", "\"Probe Name2\"")],
    ),
    row(
        "contact_address",
        Head,
        "name3",
        "string",
        Str,
        &[c("string", "\"Probe Name3\"")],
    ),
    row(
        "contact_address",
        Head,
        "name4",
        "string",
        Str,
        &[c("string", "\"Probe Name4\"")],
    ),
    row(
        "contact_address",
        Head,
        "contact",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object (the own contact)", "@ref:Contact@")],
    ),
    // createCommunicationWay / updateCommunicationWay.
    row(
        "communication_way",
        Head,
        "contact",
        "Ref{id,objectName}",
        Ref,
        &[c("reference object (the own contact)", "@ref:Contact@")],
    ),
    row(
        "communication_way",
        Head,
        "type",
        "string [EMAIL/PHONE/WEB/MOBILE]",
        Str,
        &[c("string PHONE", "\"PHONE\"")],
    ),
    // createTransaction / updateTransaction: the create-only fields are tried
    // on the update call (its spec schema lacks them).
    row(
        "transaction",
        Head,
        "valueDate",
        "string:date-time",
        Date,
        TX_DATES,
    ),
    row(
        "transaction",
        Head,
        "entryDate",
        "string:date-time",
        Date,
        TX_DATES,
    ),
    row(
        "transaction",
        Head,
        "paymtPurpose",
        "string",
        Str,
        &[c("string with run marker", "\"@marker@-changed\"")],
    ),
    row(
        "transaction",
        Head,
        "amount",
        "number:float",
        Num,
        &[c("decimal string", "\"61.23\""), c("json number", "61.23")],
    ),
    row(
        "transaction",
        Head,
        "payeePayerName",
        "string",
        Str,
        &[c("string", "\"Probe Payee\"")],
    ),
    row(
        "transaction",
        Head,
        "payeePayerAcctNo",
        "string",
        Str,
        &[c("string IBAN", "\"DE89370400440532013000\"")],
    ),
    row(
        "transaction",
        Head,
        "payeePayerBankCode",
        "string",
        Str,
        &[c("string BIC", "\"COBADEFFXXX\"")],
    ),
    row(
        "transaction",
        Head,
        "checkAccount",
        "Ref{id,objectName}",
        Ref,
        &[c(
            "reference object (the other probe account)",
            "@ref:CheckAccount@",
        )],
    ),
];
