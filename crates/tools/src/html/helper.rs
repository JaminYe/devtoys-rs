use html_escape::{decode_html_entities, encode_safe, encode_text};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Conversion {
    #[default]
    Encode,
    Decode,
}

impl Conversion {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Encode" => Some(Self::Encode),
            "Decode" => Some(Self::Decode),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HtmlError {
    #[error("非法或不完整的 HTML 实体")]
    IncompleteEntity,
}

/// Encode with `encode_text`; `safe` uses `encode_safe`.
pub fn encode(input: &str, safe: bool) -> String {
    if safe {
        encode_safe(input).into_owned()
    } else {
        encode_text(input).into_owned()
    }
}

pub fn decode(input: &str) -> Result<String, HtmlError> {
    if has_incomplete_entity(input) {
        return Err(HtmlError::IncompleteEntity);
    }
    Ok(decode_html_entities(input).into_owned())
}

pub fn convert(input: &str, conversion: Conversion) -> Result<String, HtmlError> {
    match conversion {
        Conversion::Encode => Ok(encode(input, false)),
        Conversion::Decode => decode(input),
    }
}

fn has_incomplete_entity(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            i += 1;
            continue;
        }
        let rest = &bytes[i + 1..];
        if rest.first() == Some(&b'#') {
            let hex = matches!(rest.get(1), Some(b'x' | b'X'));
            let mut j = if hex { 2 } else { 1 };
            let digits_start = j;
            while j < rest.len()
                && if hex {
                    rest[j].is_ascii_hexdigit()
                } else {
                    rest[j].is_ascii_digit()
                }
            {
                j += 1;
            }
            if j == digits_start || j >= rest.len() || rest[j] != b';' {
                return true;
            }
            i += 1 + j + 1;
        } else if rest.first().is_some_and(|c| c.is_ascii_alphabetic()) {
            let mut j = 1;
            while j < rest.len() && rest[j].is_ascii_alphanumeric() {
                j += 1;
            }
            if j >= rest.len() || rest[j] != b';' {
                return true;
            }
            i += 1 + j + 1;
        } else {
            i += 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_lt_contains_entity() {
        let got = encode("<a>", false);
        assert!(got.contains("&lt;"), "encoded form should contain &lt;");
        assert!(!got.contains('<'));
    }

    #[test]
    fn encode_decode_roundtrip() {
        let source = "<a href=\"x\">&</a>";
        let encoded = encode(source, false);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, source);
    }

    #[test]
    fn encode_safe_escapes_slash() {
        assert_eq!(encode("/", false), "/");
        let safe = encode("/", true);
        assert!(safe.contains("&#x2F;") || safe.contains("&#47;"));
    }

    #[test]
    fn convert_decode() {
        assert_eq!(convert("&lt;a&gt;", Conversion::Decode).unwrap(), "<a>");
    }

    #[test]
    fn incomplete_entity_is_err() {
        let err = decode("&lt").unwrap_err();
        assert_eq!(err, HtmlError::IncompleteEntity);
        let message = err.to_string();
        assert!(
            !message.contains("&lt"),
            "error must not include user input: {message}"
        );
    }

    #[test]
    fn incomplete_numeric_entity_is_err() {
        assert_eq!(decode("&#12"), Err(HtmlError::IncompleteEntity));
        assert_eq!(decode("&#x1"), Err(HtmlError::IncompleteEntity));
    }

    #[test]
    fn complete_entity_decodes() {
        assert_eq!(decode("&lt;a&gt;").unwrap(), "<a>");
    }

    #[test]
    fn convert_encode_default() {
        let got = convert("<a>", Conversion::Encode).unwrap();
        assert!(got.contains("&lt;"));
    }
}
