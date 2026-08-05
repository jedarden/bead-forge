//! Shared JSON `warning` channel (Phase 7.1, child 1/5).
//!
//! Most `--json` output in bead-forge is hand-rolled inline `serde_json::json!`
//! with no shared envelope. Rather than retrofit an envelope onto every
//! command, these helpers inject an optional top-level `warning` key into an
//! already-built JSON value ([`with_warning`]) and emit the same message to
//! stderr on the human path ([`warn_stderr`]). The auto-flush layer bridges a
//! failed flush into this channel (see [`crate::autoflush::FlushOutcome`]).
//!
//! # Stable shape (failure contract — bf-3jc66)
//!
//! There is ONE shape for a flush warning across every mutation command, so
//! agent consumers (NEEDLE workers) detect it uniformly:
//!
//! * **stderr** — a single line prefixed `warning: <message>`. Emitted by
//!   [`warn_stderr`] (via [`crate::autoflush`]'s outcome surfacing) and NEVER
//!   written to stdout, so `bf <mutation> --json | jq .` always parses.
//! * **`--json`** — a top-level `"warning"` key holding the same `<message>`
//!   as a non-null string, injected by [`with_warning`]. The key is **absent on
//!   the clean path** (successful flush, or auto-flush disabled); it appears
//!   only when a flush was attempted and failed. Detect with a presence /
//!   truthiness check: `if let Some(w) = obj.get("warning")`.
//!
//! A flush failure never fails the mutation — the write already committed and
//! the dirty marks are retained, so the next flush recovers.

use serde_json::Value;

/// Inject a top-level `"warning"` key into `value` when `warning` is `Some`.
///
/// * `warning == None` → `value` is returned unchanged (the common, silent case).
/// * `value` is a JSON object → the `warning` key is inserted into it.
/// * `value` is any other JSON kind (array, string, number, …) → it is wrapped
///   as `{ "warning": <w>, "result": <value> }` so the warning still has a home
///   without discarding the payload.
pub fn with_warning(value: Value, warning: Option<&str>) -> Value {
    match warning {
        None => value,
        Some(w) => match value {
            Value::Object(mut map) => {
                map.insert("warning".to_string(), Value::String(w.to_string()));
                Value::Object(map)
            }
            other => serde_json::json!({ "warning": w, "result": other }),
        },
    }
}

/// Emit a warning to stderr for the human (text/toon) output path.
///
/// Kept separate from `stdout` so a warning never corrupts machine-readable
/// output that a caller is piping.
pub fn warn_stderr(warning: &str) {
    eprintln!("warning: {warning}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn none_returns_value_unchanged() {
        let v = json!({"id": "bf-1", "status": "closed"});
        assert_eq!(with_warning(v.clone(), None), v);
    }

    #[test]
    fn some_injects_top_level_warning_on_object() {
        let v = json!({"id": "bf-1"});
        let out = with_warning(v, Some("auto-flush to JSONL failed: boom"));
        assert_eq!(out.get("id").and_then(|x| x.as_str()), Some("bf-1"));
        assert_eq!(
            out.get("warning").and_then(|x| x.as_str()),
            Some("auto-flush to JSONL failed: boom")
        );
    }

    #[test]
    fn some_overwrites_existing_warning_key() {
        let v = json!({"warning": "old"});
        let out = with_warning(v, Some("new"));
        assert_eq!(out.get("warning").and_then(|x| x.as_str()), Some("new"));
    }

    #[test]
    fn non_object_value_is_wrapped() {
        let v = json!(["a", "b"]);
        let out = with_warning(v, Some("heads up"));
        assert_eq!(
            out.get("warning").and_then(|x| x.as_str()),
            Some("heads up")
        );
        assert_eq!(
            out.get("result")
                .and_then(|x| x.as_array())
                .map(|a| a.len()),
            Some(2)
        );
    }
}
