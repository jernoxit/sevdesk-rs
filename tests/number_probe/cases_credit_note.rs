//! Cases 1, 2, 3 and 5: credit notes.
use crate::build::Kind;
use crate::calls::*;
use crate::probe::Probe;
use sevdesk::*;
use time::OffsetDateTime;

/// A draft with `number`, created by client `which`; a refused create is
/// followed by the half-written check.
async fn create(p: &mut Probe, step: &str, which: usize, number: &str, tag: &str) -> Option<i64> {
    let origin = p.origin(tag).await;
    let (save, note) = draft(p, number, tag, origin);
    let fut = cn_create(p.ctx(which), save);
    let res = p.step(step, Kind::CreditNote, None, fut).await;
    if res.id.is_none() {
        p.half_written(step, Kind::CreditNote, Some(&note), Some(number), None)
            .await;
    }
    res.id
}

fn draft(p: &Probe, number: &str, tag: &str, origin: InvoiceId) -> (SaveCreditNote, String) {
    let note = format!("{}-{tag}", p.marker());
    (
        p.env
            .credit_note_save(number.to_owned(), note.clone(), origin),
        note,
    )
}

/// `sendBy` FINALIZE, then `enshrine` when the finalize was accepted.
async fn finalize_and_enshrine(p: &mut Probe, tag: &str, which: usize, id: i64) {
    let fut = cn_finalize(p.ctx(which), id);
    let res = p
        .step(&format!("finalize_{tag}"), Kind::CreditNote, Some(id), fut)
        .await;
    if res.out.ok {
        let fut = cn_enshrine(p.ctx(which), id);
        p.step(&format!("enshrine_{tag}"), Kind::CreditNote, Some(id), fut)
            .await;
    }
}

pub(crate) async fn case1() {
    let Some(mut p) = Probe::new("case1_credit_notes_sequential").await else {
        return;
    };
    let n = p.env.ctx.c.next_credit_note_number().await.unwrap();
    eprintln!("SETUP next credit note number {n}");
    let a = create(&mut p, "create_A", 0, &n, "A").await;
    let b = create(&mut p, "create_B", 0, &n, "B").await;
    for (tag, id) in [("A", a), ("B", b)] {
        if let Some(id) = id {
            finalize_and_enshrine(&mut p, tag, 0, id).await;
        }
    }
}

pub(crate) async fn case2() {
    let Some(mut p) = Probe::new("case2_credit_notes_concurrent").await else {
        return;
    };
    let n = p.env.ctx.c.next_credit_note_number().await.unwrap();
    eprintln!("SETUP next credit note number {n}");
    let (oa, ob) = (p.origin("A").await, p.origin("B").await);
    let (sa, na) = draft(&p, &n, "A", oa);
    let (sb, nb) = draft(&p, &n, "B", ob);
    let (fa, fb) = (cn_create(p.ctx(0), sa), cn_create(p.ctx(1), sb));
    let [ra, rb] = p
        .pair("create", Kind::CreditNote, [None, None], fa, fb)
        .await;
    for (res, note) in [(&ra, &na), (&rb, &nb)] {
        if res.id.is_none() {
            p.half_written("create", Kind::CreditNote, Some(note), Some(&n), None)
                .await;
        }
    }
    let ids = [ra.id, rb.id];
    p.concurrent("finalize", Kind::CreditNote, ids, cn_finalize)
        .await;
    p.concurrent("enshrine", Kind::CreditNote, ids, cn_enshrine)
        .await;
}

pub(crate) async fn case3() {
    let Some(mut p) = Probe::new("case3_credit_note_reuses_enshrined_number").await else {
        return;
    };
    let n = p.env.ctx.c.next_credit_note_number().await.unwrap();
    eprintln!("SETUP next credit note number {n}");
    if let Some(id) = create(&mut p, "create_first", 0, &n, "first").await {
        finalize_and_enshrine(&mut p, "first", 0, id).await;
    }
    // The first note is enshrined under `n`; a second draft takes the same number.
    if let Some(id) = create(&mut p, "create_reuse", 0, &n, "reuse").await {
        finalize_and_enshrine(&mut p, "reuse", 0, id).await;
    }
}

pub(crate) async fn case5() {
    let Some(mut p) = Probe::new("case5_credit_note_number_by_sevdesk").await else {
        return;
    };
    // (a) a placeholder number: does sevDesk replace it with the sequence number?
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    let placeholder = format!("PROBE-CN-{ts}");
    let origin = p.origin("placeholder").await;
    let (save, note) = draft(&p, &placeholder, "placeholder", origin);
    let fut = cn_create(p.ctx(0), save);
    let res = p
        .step("a_create_placeholder", Kind::CreditNote, None, fut)
        .await;
    p.half_written(
        "a_create_placeholder",
        Kind::CreditNote,
        Some(&note),
        Some(&placeholder),
        None,
    )
    .await;
    if let Some(id) = res.id {
        finalize_and_enshrine(&mut p, "a_placeholder", 0, id).await;
    }
    // (b) createFromInvoice on a fresh enshrined source invoice.
    let source = p.origin("fromInvoice").await;
    let fut = cn_create_from_invoice(p.ctx(0), source.get());
    let res = p
        .step("b_createFromInvoice", Kind::CreditNote, None, fut)
        .await;
    p.half_written(
        "b_createFromInvoice",
        Kind::CreditNote,
        None,
        None,
        Some(source),
    )
    .await;
    if let Some(id) = res.id {
        finalize_and_enshrine(&mut p, "b_fromInvoice", 0, id).await;
    }
}
