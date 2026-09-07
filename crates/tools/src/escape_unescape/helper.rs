use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_TEXT};

pub const TYPE_ESCAPED: &str = "TextWithEscapedCharacters";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Conversion {
    #[default]
    Encode,
    Decode,
}

impl Conversion {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "Encode" => Some(Self::Encode),
            "Decode" => Some(Self::Decode),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EscapeError {
    #[error("非法转义序列")]
    InvalidSequence,
}

pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

pub fn decode(text: &str) -> Result<String, EscapeError> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('u') => {
                let mut hex = String::new();
                for _ in 0..4 {
                    match chars.next() {
                        Some(h) if h.is_ascii_hexdigit() => hex.push(h),
                        _ => return Err(EscapeError::InvalidSequence),
                    }
                }
                let code =
                    u32::from_str_radix(&hex, 16).map_err(|_| EscapeError::InvalidSequence)?;
                let ch = char::from_u32(code).ok_or(EscapeError::InvalidSequence)?;
                out.push(ch);
            }
            _ => return Err(EscapeError::InvalidSequence),
        }
    }
    Ok(out)
}

pub fn convert(text: &str, conversion: Conversion) -> Result<String, EscapeError> {
    match conversion {
        Conversion::Encode => Ok(encode(text)),
        Conversion::Decode => decode(text),
    }
}

pub fn looks_escaped(text: &str) -> bool {
    text.contains("\\n") || text.contains("\\t") || text.contains("\\u")
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EscapedTextDetector;

impl Detector for EscapedTextDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_ESCAPED,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        let value = parent.value.as_str();
        if !looks_escaped(value) {
            return None;
        }
        Some(DetectedPayload {
            type_name: TYPE_ESCAPED.to_string(),
            value: value.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_newline_is_backslash_n() {
        assert_eq!(encode("\n"), "\\n");
    }

    #[test]
    fn decode_reverses_newline() {
        assert_eq!(decode("\\n").unwrap(), "\n");
        assert_eq!(decode(&encode("hello\nworld")).unwrap(), "hello\nworld");
    }

    #[test]
    fn decode_invalid_u_is_err() {
        assert_eq!(decode("\\uZZ").unwrap_err(), EscapeError::InvalidSequence);
    }

    #[test]
    fn detector_sees_escaped_sequences() {
        let parent = DetectedPayload {
            type_name: TYPE_TEXT.to_string(),
            value: "hello\\nworld".to_string(),
        };
        let hit = EscapedTextDetector
            .detect(&RawData::text("hello\\nworld"), Some(&parent))
            .unwrap();
        assert_eq!(hit.type_name, TYPE_ESCAPED);
    }
}
