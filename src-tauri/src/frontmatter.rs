//! YAML frontmatter split + typed field extraction + round-trip serialize.
//!
//! A file is `---\n<yaml>\n---\n<body>`. We split on the fences, parse the
//! YAML into an ordered field list for the generic frontmatter editor, and can
//! re-join edited fields with an untouched body.

use serde::{Deserialize, Serialize};

/// One frontmatter field, typed just enough for the UI to pick a widget.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Field {
    /// A single string/number/bool -> text input.
    Scalar { key: String, value: String },
    /// A list of strings (e.g. `allowed-tools`) -> chip input.
    List { key: String, value: Vec<String> },
    /// Nested/complex -> read-only, edited via the body/raw editor.
    Raw { key: String, value: String },
}

/// Split raw file content into (frontmatter_yaml, body).
/// Returns `(None, whole)` when there is no frontmatter fence.
pub fn split(content: &str) -> (Option<&str>, &str) {
    let rest = match content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n")) {
        Some(r) => r,
        None => return (None, content),
    };
    // Find the closing fence at the start of a line.
    for pat in ["\n---\n", "\n---\r\n"] {
        if let Some(idx) = rest.find(pat) {
            let fm = &rest[..idx];
            let body = &rest[idx + pat.len()..];
            return (Some(fm), body);
        }
    }
    // Fence opened but never closed: treat whole thing as body.
    (None, content)
}

/// Parse frontmatter YAML into ordered typed fields for the editor.
pub fn parse_fields(yaml: &str) -> Vec<Field> {
    let val: serde_yml::Value = match serde_yml::from_str(yaml) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let map = match val {
        serde_yml::Value::Mapping(m) => m,
        _ => return vec![],
    };
    map.into_iter().map(|(k, v)| field_from(key_str(&k), v)).collect()
}

fn key_str(k: &serde_yml::Value) -> String {
    match k {
        serde_yml::Value::String(s) => s.clone(),
        other => serde_yml::to_string(other).unwrap_or_default().trim().to_string(),
    }
}

fn field_from(key: String, v: serde_yml::Value) -> Field {
    use serde_yml::Value::*;
    match v {
        String(s) => Field::Scalar { key, value: s },
        Bool(b) => Field::Scalar { key, value: b.to_string() },
        Number(n) => Field::Scalar { key, value: n.to_string() },
        Null => Field::Scalar { key, value: std::string::String::new() },
        Sequence(seq) if seq.iter().all(|e| matches!(e, String(_))) => {
            let value = seq
                .into_iter()
                .map(|e| if let String(s) = e { s } else { unreachable!() })
                .collect();
            Field::List { key, value }
        }
        other => Field::Raw {
            key,
            value: serde_yml::to_string(&other).unwrap_or_default().trim_end().to_string(),
        },
    }
}

/// Re-serialize edited fields back to a YAML frontmatter block (no fences).
pub fn fields_to_yaml(fields: &[Field]) -> Result<String, String> {
    let mut map = serde_yml::Mapping::new();
    for f in fields {
        let (k, v) = match f {
            Field::Scalar { key, value } => (key, serde_yml::Value::String(value.clone())),
            Field::List { key, value } => (
                key,
                serde_yml::Value::Sequence(
                    value.iter().cloned().map(serde_yml::Value::String).collect(),
                ),
            ),
            Field::Raw { key, value } => {
                let parsed: serde_yml::Value =
                    serde_yml::from_str(value).map_err(|e| format!("raw field `{key}`: {e}"))?;
                (key, parsed)
            }
        };
        map.insert(serde_yml::Value::String(k.clone()), v);
    }
    serde_yml::to_string(&serde_yml::Value::Mapping(map)).map_err(|e| e.to_string())
}

/// Join a frontmatter YAML block and body back into a file. If `yaml` is empty,
/// the body is returned unchanged (no frontmatter).
pub fn join(yaml: &str, body: &str) -> String {
    let y = yaml.trim_end();
    if y.is_empty() {
        return body.to_string();
    }
    format!("---\n{y}\n---\n{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\nname: backend\ndescription: does things\nallowed-tools:\n  - Read\n  - Write\n---\n# Body\n\nkeep me\n";

    #[test]
    fn split_extracts_fm_and_body() {
        let (fm, body) = split(SAMPLE);
        assert!(fm.unwrap().contains("name: backend"));
        assert_eq!(body, "# Body\n\nkeep me\n");
    }

    #[test]
    fn no_frontmatter() {
        let (fm, body) = split("# just body\n");
        assert!(fm.is_none());
        assert_eq!(body, "# just body\n");
    }

    #[test]
    fn fields_typed() {
        let (fm, _) = split(SAMPLE);
        let fields = parse_fields(fm.unwrap());
        assert_eq!(fields[0], Field::Scalar { key: "name".into(), value: "backend".into() });
        assert_eq!(
            fields[2],
            Field::List { key: "allowed-tools".into(), value: vec!["Read".into(), "Write".into()] }
        );
    }

    #[test]
    fn body_preserved_byte_for_byte_on_roundtrip() {
        let (fm, body) = split(SAMPLE);
        let fields = parse_fields(fm.unwrap());
        let yaml = fields_to_yaml(&fields).unwrap();
        let rejoined = join(&yaml, body);
        // Body region must be identical after a round trip.
        let (_, body2) = split(&rejoined);
        assert_eq!(body, body2);
    }
}
