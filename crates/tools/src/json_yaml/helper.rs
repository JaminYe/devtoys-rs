use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

use crate::indent::Indentation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Conversion {
    #[default]
    JsonToYaml,
    YamlToJson,
}

impl Conversion {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "JsonToYaml" => Some(Self::JsonToYaml),
            "YamlToJson" => Some(Self::YamlToJson),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::JsonToYaml => "JsonToYaml",
            Self::YamlToJson => "YamlToJson",
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum JsonYamlError {
    #[error("非法 JSON")]
    InvalidJson,
    #[error("非法 YAML")]
    InvalidYaml,
    #[error("JSON→YAML 仅支持两空格或四空格缩进")]
    UnsupportedYamlIndent,
}

pub fn yaml_output_indent_ok(direction: Conversion, indent: Indentation) -> bool {
    match direction {
        Conversion::JsonToYaml => {
            matches!(indent, Indentation::TwoSpaces | Indentation::FourSpaces)
        }
        Conversion::YamlToJson => true,
    }
}

pub fn coerce_json_yaml_indent(direction: Conversion, indent: Indentation) -> Indentation {
    if yaml_output_indent_ok(direction, indent) {
        indent
    } else {
        Indentation::TwoSpaces
    }
}

pub fn convert_json_yaml(
    input: &str,
    direction: Conversion,
    indent: Indentation,
) -> Result<String, JsonYamlError> {
    match direction {
        Conversion::JsonToYaml => json_to_yaml(input, indent),
        Conversion::YamlToJson => yaml_to_json(input, indent),
    }
}

pub fn looks_like_json(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.parse::<i64>().is_ok() {
        return false;
    }
    serde_json::from_str::<JsonValue>(t).is_ok()
}

pub fn looks_like_yaml(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    let is_json = serde_json::from_str::<JsonValue>(t).is_ok();
    let is_yaml_doc = t.starts_with("---");
    if is_json && !is_yaml_doc {
        return false;
    }
    match serde_yaml::from_str::<YamlValue>(t) {
        Ok(YamlValue::Mapping(_) | YamlValue::Sequence(_)) => true,
        Ok(_) if is_yaml_doc => true,
        _ => false,
    }
}

fn json_to_yaml(input: &str, indent: Indentation) -> Result<String, JsonYamlError> {
    let value: JsonValue = serde_json::from_str(input).map_err(|_| JsonYamlError::InvalidJson)?;
    let step = match indent {
        Indentation::TwoSpaces => 2,
        Indentation::FourSpaces => 4,
        Indentation::OneTab | Indentation::Minified => {
            return Err(JsonYamlError::UnsupportedYamlIndent);
        }
    };
    Ok(emit_json(&value, step))
}

fn yaml_to_json(input: &str, indent: Indentation) -> Result<String, JsonYamlError> {
    let value: YamlValue = serde_yaml::from_str(input).map_err(|_| JsonYamlError::InvalidYaml)?;
    serialize_json(&value, indent)
}

struct YamlEmitter {
    out: String,
    step: usize,
}

fn emit_json(value: &JsonValue, step: usize) -> String {
    let mut emitter = YamlEmitter {
        out: String::new(),
        step,
    };
    match value {
        JsonValue::Object(map) => emitter.emit_json_map(map, 0, false),
        JsonValue::Array(seq) => emitter.emit_json_seq(seq, 0, false),
        other => emitter.emit_json(other, 0, true),
    }
    emitter.out.trim_end().to_string()
}

impl YamlEmitter {
    fn pad(&mut self, col: usize) {
        for _ in 0..col {
            self.out.push(' ');
        }
    }

    fn break_and_pad(&mut self, col: usize) {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.pad(col);
    }

    fn emit_json(&mut self, value: &JsonValue, col: usize, inline: bool) {
        match value {
            JsonValue::Null => self.out.push_str("null"),
            JsonValue::Bool(true) => self.out.push_str("true"),
            JsonValue::Bool(false) => self.out.push_str("false"),
            JsonValue::Number(n) => self.out.push_str(&n.to_string()),
            JsonValue::String(s) => write_yaml_string(&mut self.out, s),
            JsonValue::Array(seq) => self.emit_json_seq(seq, col, inline),
            JsonValue::Object(map) => self.emit_json_map(map, col, inline),
        }
    }

    fn emit_json_map(
        &mut self,
        map: &serde_json::Map<String, JsonValue>,
        col: usize,
        inline: bool,
    ) {
        if map.is_empty() {
            self.out.push_str("{}");
            return;
        }
        for (i, (key, value)) in map.iter().enumerate() {
            if i > 0 || !inline {
                self.break_and_pad(col);
            }
            write_yaml_string(&mut self.out, key);
            match value {
                JsonValue::Object(nested) if !nested.is_empty() => {
                    self.out.push(':');
                    self.emit_json_map(nested, col + self.step, false);
                }
                JsonValue::Array(nested) if !nested.is_empty() => {
                    self.out.push(':');
                    self.out.push('\n');
                    self.emit_json_seq(nested, col + self.step, false);
                }
                other => {
                    self.out.push_str(": ");
                    self.emit_json(other, col + self.step, true);
                }
            }
        }
    }

    fn emit_json_seq(&mut self, seq: &[JsonValue], col: usize, inline: bool) {
        if seq.is_empty() {
            self.out.push_str("[]");
            return;
        }
        for (i, item) in seq.iter().enumerate() {
            if i > 0 || !inline {
                self.break_and_pad(col);
            }
            self.out.push_str("- ");
            match item {
                JsonValue::Object(map) if !map.is_empty() => {
                    self.emit_json_map(map, col + 2, true);
                }
                JsonValue::Array(nested) if !nested.is_empty() => {
                    self.out.push('\n');
                    self.emit_json_seq(nested, col + self.step, false);
                }
                other => self.emit_json(other, col + self.step, true),
            }
        }
    }
}

fn write_yaml_string(out: &mut String, s: &str) {
    if needs_yaml_quotes(s) {
        write_double_quoted(out, s);
    } else {
        out.push_str(s);
    }
}

fn needs_yaml_quotes(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    let first = s.chars().next().unwrap();
    let last = s.chars().next_back().unwrap();
    if first.is_whitespace() || last.is_whitespace() {
        return true;
    }
    if matches!(
        first,
        '0'..='9'
            | '+'
            | '-'
            | '.'
            | '?'
            | ':'
            | '{'
            | '}'
            | '['
            | ']'
            | ','
            | '&'
            | '*'
            | '!'
            | '|'
            | '>'
            | '\''
            | '"'
            | '%'
            | '@'
            | '`'
            | '#'
            | '~'
    ) {
        return true;
    }
    if s.chars().any(|c| {
        c.is_control()
            || matches!(
                c,
                ':' | '#' | '{' | '}' | '[' | ']' | ',' | '&' | '*' | '!' | '|' | '>' | '\'' | '"'
            )
    }) {
        return true;
    }
    is_yaml_reserved_plain(s)
}

fn is_yaml_reserved_plain(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "y" | "n"
            | "yes"
            | "no"
            | "true"
            | "false"
            | "on"
            | "off"
            | "null"
            | "~"
            | ".inf"
            | "-.inf"
            | ".nan"
    ) || s.parse::<f64>().is_ok()
}

fn write_double_quoted(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn serialize_json<T: Serialize>(value: &T, indent: Indentation) -> Result<String, JsonYamlError> {
    match indent {
        Indentation::Minified => {
            serde_json::to_string(value).map_err(|_| JsonYamlError::InvalidYaml)
        }
        Indentation::TwoSpaces => pretty_json(value, b"  "),
        Indentation::FourSpaces => pretty_json(value, b"    "),
        Indentation::OneTab => pretty_json(value, b"\t"),
    }
}

fn pretty_json<T: Serialize>(value: &T, indent: &[u8]) -> Result<String, JsonYamlError> {
    let mut buf = Vec::new();
    let formatter = PrettyFormatter::with_indent(indent);
    let mut serializer = Serializer::with_formatter(&mut buf, formatter);
    value
        .serialize(&mut serializer)
        .map_err(|_| JsonYamlError::InvalidYaml)?;
    String::from_utf8(buf).map_err(|_| JsonYamlError::InvalidYaml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_object_to_yaml_contains_key_value() {
        let got = convert_json_yaml(r#"{"a":1}"#, Conversion::JsonToYaml, Indentation::TwoSpaces)
            .unwrap();
        assert!(got.contains("a: 1"), "{got}");
    }

    #[test]
    fn yaml_to_json_minified() {
        let got = convert_json_yaml("a: 1", Conversion::YamlToJson, Indentation::Minified).unwrap();
        assert_eq!(got, r#"{"a":1}"#);
    }

    #[test]
    fn invalid_json_is_err_without_garbage() {
        let input = "{not-json";
        let err =
            convert_json_yaml(input, Conversion::JsonToYaml, Indentation::TwoSpaces).unwrap_err();
        assert_eq!(err, JsonYamlError::InvalidJson);
        let message = err.to_string();
        assert!(!message.contains('{'));
        assert!(!message.contains("not-json"));
    }

    #[test]
    fn invalid_yaml_is_err_without_garbage() {
        let input = ":\n  - [";
        let err =
            convert_json_yaml(input, Conversion::YamlToJson, Indentation::TwoSpaces).unwrap_err();
        assert_eq!(err, JsonYamlError::InvalidYaml);
        let message = err.to_string();
        assert!(!message.contains('['));
        assert!(!message.contains(':'));
    }

    fn json_yaml_semantic_roundtrip(input: &str, indent: Indentation) {
        let yaml = convert_json_yaml(input, Conversion::JsonToYaml, indent).expect(input);
        assert!(
            !yaml.contains('\t'),
            "YAML structure must not use tab indent: {yaml}"
        );
        let parsed: YamlValue = serde_yaml::from_str(&yaml).unwrap_or_else(|err| {
            panic!("output must be legal YAML ({err}): {yaml}");
        });
        let original: JsonValue = serde_json::from_str(input).unwrap();
        let from_yaml: JsonValue = serde_json::to_value(&parsed).expect("yaml value to json");
        assert_eq!(from_yaml, original, "semantic mismatch, yaml={yaml}");
    }

    #[test]
    fn json_to_yaml_two_and_four_spaces_roundtrip_nested_values() {
        let samples = [
            r#"{"a":{"b":1}}"#,
            r#"{"arr":[1,null,{"c":"x y"},true]}"#,
            r##"{"s":"a: b","t":"#hi","u":"true","v":"*star","w":"","x":"  spaces  "}"##,
            r#"{"n":null,"z":{},"e":[]}"#,
            r#"[{"k":1},{"k":[2,{"m":3}]}]"#,
        ];
        for input in samples {
            json_yaml_semantic_roundtrip(input, Indentation::TwoSpaces);
            json_yaml_semantic_roundtrip(input, Indentation::FourSpaces);
        }
    }

    #[test]
    fn json_to_yaml_preserves_multiline_string_inner_spaces() {
        let input = r#"{"s":"a\n  b"}"#;
        for indent in [Indentation::TwoSpaces, Indentation::FourSpaces] {
            let yaml = convert_json_yaml(input, Conversion::JsonToYaml, indent).unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&yaml).expect(&yaml);
            assert_eq!(
                parsed["s"].as_str(),
                Some("a\n  b"),
                "indent={indent:?} yaml={yaml}"
            );
            json_yaml_semantic_roundtrip(input, indent);
        }
        let nested = r#"{"outer":{"s":"a\n  b\n    c"}}"#;
        for indent in [Indentation::TwoSpaces, Indentation::FourSpaces] {
            json_yaml_semantic_roundtrip(nested, indent);
            let yaml = convert_json_yaml(nested, Conversion::JsonToYaml, indent).unwrap();
            let parsed: YamlValue = serde_yaml::from_str(&yaml).expect(&yaml);
            assert_eq!(parsed["outer"]["s"].as_str(), Some("a\n  b\n    c"));
        }
    }

    #[test]
    fn json_to_yaml_four_spaces_changes_structure_not_values() {
        let input = r#"{"a":{"b":1}}"#;
        let two = convert_json_yaml(input, Conversion::JsonToYaml, Indentation::TwoSpaces).unwrap();
        let four =
            convert_json_yaml(input, Conversion::JsonToYaml, Indentation::FourSpaces).unwrap();
        assert!(two.contains("\n  b:"), "{two}");
        assert!(!two.contains("\n    b:"), "{two}");
        assert!(four.contains("\n    b:"), "{four}");
        assert!(!four.contains('\t'), "{four}");
        json_yaml_semantic_roundtrip(input, Indentation::TwoSpaces);
        json_yaml_semantic_roundtrip(input, Indentation::FourSpaces);
    }

    #[test]
    fn json_to_yaml_keeps_integers_beyond_u64() {
        let input = r#"{"n":18446744073709551617,"neg":-9223372036854775809}"#;
        for indent in [Indentation::TwoSpaces, Indentation::FourSpaces] {
            let yaml = convert_json_yaml(input, Conversion::JsonToYaml, indent).unwrap();
            assert!(
                yaml.contains("18446744073709551617"),
                "indent={indent:?} yaml={yaml}"
            );
            assert!(
                yaml.contains("-9223372036854775809"),
                "indent={indent:?} yaml={yaml}"
            );
        }
    }

    #[test]
    fn json_to_yaml_tab_and_minified_are_unsupported() {
        let input = r#"{"a":{"b":1}}"#;
        for indent in [Indentation::OneTab, Indentation::Minified] {
            let err = convert_json_yaml(input, Conversion::JsonToYaml, indent).unwrap_err();
            assert_eq!(err, JsonYamlError::UnsupportedYamlIndent, "{indent:?}");
        }
    }

    #[test]
    fn yaml_to_json_keeps_existing_indent_options() {
        let yaml = "a:\n  b: 1";
        let minified =
            convert_json_yaml(yaml, Conversion::YamlToJson, Indentation::Minified).unwrap();
        assert_eq!(minified, r#"{"a":{"b":1}}"#);
        let two = convert_json_yaml(yaml, Conversion::YamlToJson, Indentation::TwoSpaces).unwrap();
        assert!(two.contains("\n  "), "{two}");
        let four =
            convert_json_yaml(yaml, Conversion::YamlToJson, Indentation::FourSpaces).unwrap();
        assert!(four.contains("\n    "), "{four}");
        let tab = convert_json_yaml(yaml, Conversion::YamlToJson, Indentation::OneTab).unwrap();
        assert!(tab.contains('\t'), "{tab}");
        let original: JsonValue = serde_json::from_str(r#"{"a":{"b":1}}"#).unwrap();
        for indent in [
            Indentation::TwoSpaces,
            Indentation::FourSpaces,
            Indentation::OneTab,
            Indentation::Minified,
        ] {
            let got = convert_json_yaml(yaml, Conversion::YamlToJson, indent).unwrap();
            let parsed: JsonValue = serde_json::from_str(&got).unwrap();
            assert_eq!(parsed, original, "{indent:?} {got}");
        }
    }

    #[test]
    fn json_to_yaml_gui_hides_unsupported_indent() {
        assert!(yaml_output_indent_ok(
            Conversion::JsonToYaml,
            Indentation::TwoSpaces
        ));
        assert!(yaml_output_indent_ok(
            Conversion::JsonToYaml,
            Indentation::FourSpaces
        ));
        assert!(!yaml_output_indent_ok(
            Conversion::JsonToYaml,
            Indentation::OneTab
        ));
        assert!(!yaml_output_indent_ok(
            Conversion::JsonToYaml,
            Indentation::Minified
        ));
        for indent in [
            Indentation::TwoSpaces,
            Indentation::FourSpaces,
            Indentation::OneTab,
            Indentation::Minified,
        ] {
            assert!(yaml_output_indent_ok(Conversion::YamlToJson, indent));
        }
        assert_eq!(
            coerce_json_yaml_indent(Conversion::JsonToYaml, Indentation::OneTab),
            Indentation::TwoSpaces
        );
        assert_eq!(
            coerce_json_yaml_indent(Conversion::JsonToYaml, Indentation::Minified),
            Indentation::TwoSpaces
        );
        assert_eq!(
            coerce_json_yaml_indent(Conversion::YamlToJson, Indentation::OneTab),
            Indentation::OneTab
        );
        assert_eq!(
            coerce_json_yaml_indent(Conversion::JsonToYaml, Indentation::FourSpaces),
            Indentation::FourSpaces
        );
    }
}
