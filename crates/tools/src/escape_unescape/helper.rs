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
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
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
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('u') => {
                let mut hex = String::with_capacity(4);
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
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => {
                out.push('\\');
            }
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
    let mut chars = text.chars().peekable();
    let mut found_supported_escape = false;

    while let Some(c) = chars.next() {
        if c != '\\' {
            continue;
        }
        match chars.next() {
            Some('n' | 'r' | 't' | 'b' | 'f' | '\\' | '"' | '\'') => {
                found_supported_escape = true;
            }
            Some('u') => {
                let mut hex = [0u8; 4];
                for byte in &mut hex {
                    match chars.next() {
                        Some(h) if h.is_ascii_hexdigit() => {
                            *byte = h as u8;
                        }
                        _ => return false,
                    }
                }
                let s = match std::str::from_utf8(&hex) {
                    Ok(s) => s,
                    Err(_) => return false,
                };
                let code = match u32::from_str_radix(s, 16) {
                    Ok(code) => code,
                    Err(_) => return false,
                };
                if char::from_u32(code).is_none() {
                    return false;
                }
                found_supported_escape = true;
            }
            Some(_) => {
                // Unknown escape sequence preserved as-is per issue 09,
                // does not count as supported escape on its own.
            }
            None => {
                // Trailing backslash preserved as-is per issue 09,
                // does not count as supported escape on its own.
            }
        }
    }

    found_supported_escape
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
        Some(DetectedPayload::new(TYPE_ESCAPED, value))
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
    fn decode_backspace_is_char_code_8() {
        let out = decode("\\b").unwrap();
        assert_eq!(out.chars().count(), 1);
        assert_eq!(out.chars().next().unwrap() as u32, 8);
        assert_eq!(out, "\u{8}");
    }

    #[test]
    fn decode_form_feed_is_char_code_12() {
        let out = decode("\\f").unwrap();
        assert_eq!(out.chars().count(), 1);
        assert_eq!(out.chars().next().unwrap() as u32, 12);
        assert_eq!(out, "\u{c}");
    }

    #[test]
    fn decode_mixed_backspace_form_feed_keeps_other_chars() {
        let out = decode("A\\bB\\fC").unwrap();
        assert_eq!(out, "A\u{8}B\u{c}C");
        let codes: Vec<u32> = out.chars().map(|c| c as u32).collect();
        assert_eq!(codes, [0x41, 8, 0x42, 12, 0x43]);
    }

    #[test]
    fn encode_decode_round_trip_backspace_and_form_feed() {
        let original = "A\u{8}B\u{c}C";
        assert_eq!(encode(original), "A\\bB\\fC");
        assert_eq!(decode("A\\bB\\fC").unwrap(), original);
        assert_eq!(decode(&encode(original)).unwrap(), original);
        assert_eq!(encode(&decode("A\\bB\\fC").unwrap()), "A\\bB\\fC");
    }

    #[test]
    fn decode_existing_sequences_unchanged() {
        assert_eq!(decode("\\n").unwrap(), "\n");
        assert_eq!(decode("\\r").unwrap(), "\r");
        assert_eq!(decode("\\t").unwrap(), "\t");
        assert_eq!(decode("\\\\").unwrap(), "\\");
        assert_eq!(decode("\\\"").unwrap(), "\"");
        assert_eq!(decode("\\u0041").unwrap(), "A");
        assert_eq!(decode("\\u0008").unwrap(), "\u{8}");
        assert_eq!(decode("\\u000C").unwrap(), "\u{c}");
    }

    #[test]
    fn decode_unknown_escape_and_trailing_backslash_preserved() {
        assert_eq!(decode("hello\\qworld").unwrap(), "hello\\qworld");
        assert_eq!(decode("hello\\").unwrap(), "hello\\");
        assert_eq!(decode("\\q").unwrap(), "\\q");
        assert_eq!(decode("\\").unwrap(), "\\");
        assert_eq!(decode("ok\\x").unwrap(), "ok\\x");
        assert_eq!(decode("hello\\1world").unwrap(), "hello\\1world");
        assert_eq!(decode("a\\\\\\").unwrap(), "a\\\\");
    }

    #[test]
    fn decode_mixed_known_and_unknown_escapes() {
        assert_eq!(decode("hello\\q\\nworld").unwrap(), "hello\\q\nworld");
        assert_eq!(
            decode("a\\qb\\nc\\td\\be\\ff\\\"g\\'h\\\\i").unwrap(),
            "a\\qb\nc\td\u{8}e\u{c}f\"g'h\\i"
        );
    }

    #[test]
    fn decode_single_quote() {
        assert_eq!(decode("\\'").unwrap(), "'");
    }

    #[test]
    fn decode_invalid_and_incomplete_u_sequences_err() {
        assert_eq!(decode("\\u").unwrap_err(), EscapeError::InvalidSequence);
        assert_eq!(decode("\\u12").unwrap_err(), EscapeError::InvalidSequence);
        assert_eq!(decode("\\uZZZZ").unwrap_err(), EscapeError::InvalidSequence);
        assert_eq!(decode("\\uD800").unwrap_err(), EscapeError::InvalidSequence);
        assert_eq!(
            decode("ok\\n\\uZZ").unwrap_err(),
            EscapeError::InvalidSequence
        );
    }

    #[test]
    fn convert_decode_backspace_form_feed_and_unknown() {
        assert_eq!(
            convert("A\\bB\\fC", Conversion::Decode).unwrap(),
            "A\u{8}B\u{c}C"
        );
        assert_eq!(convert("\\q", Conversion::Decode).unwrap(), "\\q");
        assert_eq!(
            convert("\\uZZ", Conversion::Decode).unwrap_err(),
            EscapeError::InvalidSequence
        );
    }

    #[test]
    fn looks_escaped_detects_all_supported_escapes() {
        assert!(looks_escaped("hello\\nworld"));
        assert!(looks_escaped("hello\\rworld"));
        assert!(looks_escaped("hello\\tworld"));
        assert!(looks_escaped("hello\\bworld"));
        assert!(looks_escaped("hello\\fworld"));
        assert!(looks_escaped("hello\\\"world\\\""));
        assert!(looks_escaped("hello\\\'world\\\'"));
        assert!(looks_escaped("hello\\\\world"));
        assert!(looks_escaped("\\\\"));
        assert!(looks_escaped("hello\\u0041world"));
        assert!(looks_escaped("hello\\q\\nworld"));
    }

    #[test]
    fn looks_escaped_rejects_plain_and_invalid() {
        assert!(!looks_escaped("hello world"));
        assert!(!looks_escaped("12345"));
        assert!(!looks_escaped("C:\\Program Files"));
        assert!(!looks_escaped("hello\\qworld"));
        assert!(!looks_escaped("hello\\"));
        assert!(!looks_escaped("\\"));
        assert!(!looks_escaped("hello\\uZZZZworld"));
        assert!(!looks_escaped("hello\\u12"));
        assert!(!looks_escaped("hello\\u"));
        assert!(!looks_escaped("hello\\uD800"));
        assert!(!looks_escaped("hello\\n\\uZZZZ"));
    }

    #[test]
    fn detector_sees_escaped_sequences() {
        for sample in [
            "hello\\nworld",
            "hello\\rworld",
            "hello\\tworld",
            "hello\\bworld",
            "hello\\fworld",
            "hello\\\"world\\\"",
            "hello\\\'world\\\'",
            "hello\\\\world",
            "hello\\u0041world",
        ] {
            let parent = DetectedPayload::new(TYPE_TEXT, sample);
            let hit = EscapedTextDetector
                .detect(&RawData::text(sample), Some(&parent))
                .expect(sample);
            assert_eq!(hit.type_name, TYPE_ESCAPED);
        }

        for non_sample in ["hello world", "hello\\qworld", "hello\\", "hello\\uZZZZ"] {
            let parent = DetectedPayload::new(TYPE_TEXT, non_sample);
            let hit = EscapedTextDetector.detect(&RawData::text(non_sample), Some(&parent));
            assert!(hit.is_none(), "Should not detect: {non_sample}");
        }
    }
}
