//! Case 4: invoices with a duplicate, preset number.
use crate::build::Kind;
use crate::calls::*;
use crate::probe::Probe;
use time::OffsetDateTime;

/// Two drafts get the same `invoiceNumber` by raw PUT, then are finalized and
/// enshrined, one after the other (`concurrent` false) or at the same time.
async fn duplicate(p: &mut Probe, concurrent: bool, number: String) {
    let tag = if concurrent { "b" } else { "a" };
    let mut ids = [None, None];
    for (slot, name) in ["A", "B"].into_iter().enumerate() {
        let doc = p
            .env
            .new_draft(Kind::Invoice, &format!("{tag}-{name}"), 1)
            .await
            .unwrap();
        eprintln!("SETUP draft invoice {} ({tag}-{name})", doc.id);
        ids[slot] = Some(doc.id);
    }
    let put = |ctx, id| inv_set_number(ctx, id, number.clone());
    if concurrent {
        p.concurrent(&format!("{tag}_put_number"), Kind::Invoice, ids, put)
            .await;
        p.concurrent(&format!("{tag}_finalize"), Kind::Invoice, ids, inv_finalize)
            .await;
        p.concurrent(&format!("{tag}_enshrine"), Kind::Invoice, ids, inv_enshrine)
            .await;
    } else {
        for (slot, name) in ["A", "B"].into_iter().enumerate() {
            let Some(id) = ids[slot] else { continue };
            let fut = put(p.ctx(0), id);
            p.step(
                &format!("{tag}_put_number_{name}"),
                Kind::Invoice,
                Some(id),
                fut,
            )
            .await;
            let fut = inv_finalize(p.ctx(0), id);
            p.step(
                &format!("{tag}_finalize_{name}"),
                Kind::Invoice,
                Some(id),
                fut,
            )
            .await;
            let fut = inv_enshrine(p.ctx(0), id);
            p.step(
                &format!("{tag}_enshrine_{name}"),
                Kind::Invoice,
                Some(id),
                fut,
            )
            .await;
        }
    }
    p.half_written(tag, Kind::Invoice, None, Some(&number), None)
        .await;
}

pub(crate) async fn case4() {
    let Some(mut p) = Probe::new("case4_invoices_duplicate_number").await else {
        return;
    };
    let ts = OffsetDateTime::now_utc().unix_timestamp();
    duplicate(&mut p, false, format!("PROBE-{ts}-a")).await;
    duplicate(&mut p, true, format!("PROBE-{ts}-b")).await;
}
