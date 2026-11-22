//! One probed object: how it is read back and how an update reaches it.

use crate::classify::id_i64;
use crate::harness::Ctx;
use crate::table::Scope;
use reqwest::Method;
use serde_json::{Map, Value};

/// A save factory: the full body of the creating call, with the document id and
/// the position id already in it, patched per update.
pub(crate) struct Factory {
    pub(crate) path: &'static str,
    pub(crate) base: Value,
    pub(crate) head_key: &'static str,
    pub(crate) pos_key: &'static str,
}

pub(crate) struct Target {
    /// File name and `FIELD` object column.
    pub(crate) key: &'static str,
    /// The sevDesk object name, for `@self@`.
    pub(crate) object: &'static str,
    pub(crate) id: i64,
    pub(crate) head_get: String,
    pub(crate) pos_get: Option<String>,
    /// `PUT` route of the head; without it the head goes through the factory.
    pub(crate) put: Option<String>,
    pub(crate) factory: Option<Factory>,
}

pub(crate) struct Snap {
    pub(crate) head: Value,
    pub(crate) pos: Option<Value>,
    pub(crate) pos_count: usize,
}

/// The `objects` of an answer, `None` when the body is no such answer.
pub(crate) fn objects_of(text: &str) -> Option<Value> {
    serde_json::from_str::<Value>(text)
        .ok()
        .map(|v| v["objects"].clone())
        .filter(|o| !o.is_null())
}

impl Target {
    /// Reads head and first position. A failing read is no measurement: panic.
    pub(crate) async fn snapshot(&self, ctx: &Ctx) -> Snap {
        let (status, body) = ctx.raw(Method::GET, &self.head_get, None, None).await;
        assert!(status < 300, "GET {} answered {status}", self.head_get);
        let head = match objects_of(&body).expect("the read has objects") {
            Value::Array(mut list) if !list.is_empty() => list.remove(0),
            other => other,
        };
        let (pos, pos_count) = if let Some(path) = &self.pos_get {
            let (status, body) = ctx.raw(Method::GET, path, None, None).await;
            assert!(status < 300, "GET {path} answered {status}");
            let list = objects_of(&body)
                .and_then(|o| o.as_array().cloned())
                .unwrap_or_default();
            (list.first().cloned(), list.len())
        } else {
            (None, 0)
        };
        Snap {
            head,
            pos,
            pos_count,
        }
    }

    /// One write, never retried. A factory answer that names another document
    /// than the probed one means a document was created: stop the whole probe.
    pub(crate) async fn send(
        &self,
        ctx: &Ctx,
        scope: Scope,
        patch: Map<String, Value>,
    ) -> (u16, String) {
        if let (Scope::Head, Some(put)) = (scope, &self.put) {
            return ctx
                .raw(
                    Method::PUT,
                    put,
                    Some(Value::Object(patch).to_string()),
                    None,
                )
                .await;
        }
        let f = self.factory.as_ref().expect("a factory for this route");
        let mut body = f.base.clone();
        let slot = match scope {
            Scope::Head => &mut body[f.head_key],
            Scope::Pos => &mut body[f.pos_key][0],
        };
        for (k, v) in patch {
            slot[k.as_str()] = v;
        }
        let (status, text) = ctx
            .raw(Method::POST, f.path, Some(body.to_string()), None)
            .await;
        if status < 300
            && let Ok(answer) = serde_json::from_str::<Value>(&text)
            && let Some(got) = id_i64(&answer["objects"][f.head_key])
            && got != self.id
        {
            eprintln!(
                "CREATED {} {got} (UNEXPECTED, by the factory update)",
                self.object
            );
            panic!("the factory created a new document instead of updating: stopping");
        }
        (status, text)
    }
}
