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
    if indent == Indentation::Minified {
        return serde_json::to_string(&value).map_err(|_| JsonYamlError::InvalidJson);
    }
    let yaml = serde_yaml::to_string(&value).map_err(|_| JsonYamlError::InvalidYaml)?;
    Ok(apply_yaml_indent(yaml.trim_end(), indent))
}

fn yaml_to_json(input: &str, indent: Indentation) -> Result<String, JsonYamlError> {
    let value: YamlValue = serde_yaml::from_str(input).map_err(|_| JsonYamlError::InvalidYaml)?;
    serialize_json(&value, indent)
}

fn apply_yaml_indent(text: &str, indent: Indentation) -> String {
    let replacement = match indent {
        Indentation::TwoSpaces => return text.to_string(),
        Indentation::FourSpaces => "    ",
        Indentation::OneTab => "\t",
        Indentation::Minified => return text.to_string(),
    };
    text.lines()
        .map(|line| {
            let spaces = line.chars().take_while(|c| *c == ' ').count();
            if spaces > 0 && spaces % 2 == 0 {
                format!("{}{}", replacement.repeat(spaces / 2), &line[spaces..])
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
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
}
