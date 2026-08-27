use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};
use serde_json::Value;

pub use crate::indent::Indentation;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JsonFormatError {
    #[error("非法 JSON")]
    InvalidJson,
}

pub fn format_json(
    input: &str,
    indent: Indentation,
    sort_properties: bool,
) -> Result<String, JsonFormatError> {
    let mut value: Value =
        serde_json::from_str(input).map_err(|_| JsonFormatError::InvalidJson)?;
    if sort_properties {
        sort_value(&mut value);
    }
    serialize_value(&value, indent)
}

fn sort_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for v in map.values_mut() {
                sort_value(v);
            }
            let mut entries: Vec<_> = std::mem::take(map).into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            *map = entries.into_iter().collect();
        }
        Value::Array(items) => {
            for item in items {
                sort_value(item);
            }
        }
        _ => {}
    }
}

fn serialize_value(value: &Value, indent: Indentation) -> Result<String, JsonFormatError> {
    match indent {
        Indentation::Minified => {
            serde_json::to_string(value).map_err(|_| JsonFormatError::InvalidJson)
        }
        Indentation::TwoSpaces => pretty(value, b"  "),
        Indentation::FourSpaces => pretty(value, b"    "),
        Indentation::OneTab => pretty(value, b"\t"),
    }
}

fn pretty(value: &Value, indent: &[u8]) -> Result<String, JsonFormatError> {
    let mut buf = Vec::new();
    let formatter = PrettyFormatter::with_indent(indent);
    let mut serializer = Serializer::with_formatter(&mut buf, formatter);
    value
        .serialize(&mut serializer)
        .map_err(|_| JsonFormatError::InvalidJson)?;
    String::from_utf8(buf).map_err(|_| JsonFormatError::InvalidJson)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_spaces_preserves_key_order() {
        let got = format_json(r#"{"b":1,"a":2}"#, Indentation::TwoSpaces, false).unwrap();
        assert_eq!(got, "{\n  \"b\": 1,\n  \"a\": 2\n}");
    }

    #[test]
    fn four_spaces() {
        let got = format_json(r#"{"a":1}"#, Indentation::FourSpaces, false).unwrap();
        assert_eq!(got, "{\n    \"a\": 1\n}");
    }

    #[test]
    fn one_tab() {
        let got = format_json(r#"{"a":1}"#, Indentation::OneTab, false).unwrap();
        assert_eq!(got, "{\n\t\"a\": 1\n}");
    }

    #[test]
    fn minified() {
        let got = format_json("{ \"a\": 1 }", Indentation::Minified, false).unwrap();
        assert_eq!(got, "{\"a\":1}");
    }

    #[test]
    fn sort_properties_minified_nested() {
        let got = format_json(
            r#"{"z":{"b":1,"a":2},"a":0}"#,
            Indentation::Minified,
            true,
        )
        .unwrap();
        assert_eq!(got, r#"{"a":0,"z":{"a":2,"b":1}}"#);
    }

    #[test]
    fn invalid_json_is_err_without_input() {
        let err = format_json("{", Indentation::TwoSpaces, false).unwrap_err();
        assert_eq!(err, JsonFormatError::InvalidJson);
        let message = err.to_string();
        assert!(!message.contains('{'), "error must not include user input");
    }
}
