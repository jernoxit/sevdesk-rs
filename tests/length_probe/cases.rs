//! The cases: one draft per length, and the finalized credit note.
use crate::build::Kind;
use crate::content::{LEVELS, value};
use crate::docs::net_position;
use crate::probe::{Probe, Row};
use crate::record::Outcome;
use sevdesk::*;

#[derive(Clone, Copy)]
pub(crate) enum Field {
    Name,
    Text,
}

impl Field {
    fn wire(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Text => "text",
        }
    }

    /// The one position with `sent` in this field.
    fn position(self, sent: &str) -> DocumentPositionDraft {
        match self {
            Self::Name => net_position(1, 500, sent, None),
            Self::Text => net_position(1, 500, "LEN-PROBE", Some(sent)),
        }
    }
}

/// Saves one draft of `kind` with `sent` in `field`; a refused save is
/// followed by the half-written check, an accepted one by the re-read.
/// Returns the id of the draft when saved.
async fn save_one(
    p: &mut Probe,
    kind: Kind,
    field: Field,
    step: &str,
    level: &str,
    sent: String,
) -> Option<i64> {
    let note = format!("{}-{step}", p.marker());
    let position = field.position(&sent);
    let mut row = Row::new(step, kind, field.wire(), level);
    row.sent = Some(sent);
    let id = match kind {
        Kind::Invoice => {
            let mut save = p.env.invoice_save(note.clone(), 1);
            save.positions = vec![position];
            match p.env.ctx.c.save_invoice(&save).await {
                Ok(s) => Ok(s.invoice.id.get()),
                Err(e) => Err(Outcome::from_error(&e)),
            }
        }
        Kind::CreditNote => {
            let number = p.env.ctx.c.next_credit_note_number().await.unwrap();
            let mut save = p.env.credit_note_save(number, note.clone(), p.env.origin);
            save.positions = vec![position];
            match p.env.ctx.c.save_credit_note(&save).await {
                Ok(s) => Ok(s.credit_note.id.get()),
                Err(e) => Err(Outcome::from_error(&e)),
            }
        }
    };
    match id {
        Ok(id) => {
            row.id = Some(id);
            row.got = p.reread(kind, id, field.wire()).await.value;
            p.write(row);
            Some(id)
        }
        Err(out) => {
            row.out = out;
            row.half_written = p.half_written(kind, &note).await;
            p.write(row);
            None
        }
    }
}

/// Cases 1 to 4: one draft per length.
pub(crate) async fn lengths(case: &str, kind: Kind, field: Field) {
    let Some(mut p) = Probe::new(case).await else {
        return;
    };
    for (level, chars) in LEVELS {
        let sent = value(p.marker(), chars);
        save_one(&mut p, kind, field, &format!("save_{level}"), level, sent).await;
    }
}

/// Case 5: the realistic value in a credit note `name`, finalized, the XML
/// checked for the exact name, the position re-read.
pub(crate) async fn case5() {
    let Some(mut p) = Probe::new("case5_credit_note_realistic_finalized").await else {
        return;
    };
    let kind = Kind::CreditNote;
    let sent = value(p.marker(), 420);
    let Some(id) = save_one(
        &mut p,
        kind,
        Field::Name,
        "save_realistic",
        "realistic",
        sent.clone(),
    )
    .await
    else {
        return;
    };
    let cid = CreditNoteId::new(id);

    let mut row = Row::new("send_by_finalize", kind, "name", "realistic");
    row.id = Some(id);
    match p
        .env
        .ctx
        .c
        .send_credit_note_by(cid, &SendBy::FINALIZE)
        .await
    {
        Ok(_) => {}
        Err(e) => row.out = Outcome::from_error(&e),
    }
    let finalized = row.out.ok;
    p.write(row);

    let mut row = Row::new("xml_check", kind, "name", "realistic");
    row.id = Some(id);
    match p.env.ctx.c.credit_note_xml(cid).await {
        Ok(xml) => {
            let contains = xml.contains(&sent);
            // The longest prefix of the name that the XML still carries.
            let chars: Vec<char> = sent.chars().collect();
            let (mut lo, mut hi) = (0, chars.len());
            while lo < hi {
                let mid = (lo + hi).div_ceil(2);
                if xml.contains(&chars[..mid].iter().collect::<String>()) {
                    lo = mid;
                } else {
                    hi = mid - 1;
                }
            }
            row.equal = Some(format!("xml_contains_name={contains}"));
            row.note = Some(format!(
                "xml_chars={} prefix_chars_in_xml={lo}",
                xml.chars().count()
            ));
        }
        Err(e) => row.out = Outcome::from_error(&e),
    }
    row.sent = None;
    p.write(row);

    let mut row = Row::new("reread_after_finalize", kind, "name", "realistic");
    row.id = Some(id);
    row.sent = Some(sent);
    row.got = p.reread(kind, id, "name").await.value;
    row.half_written = format!("finalize_ok={finalized}");
    p.write(row);
}
