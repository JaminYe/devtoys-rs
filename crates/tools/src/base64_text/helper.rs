use base64::engine::general_purpose::STANDARD;
use base64::Engine;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Charset {
    #[default]
    Utf8,
    Ascii,
}

impl Charset {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Utf8" => Some(Self::Utf8),
            "Ascii" => Some(Self::Ascii),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Base64TextError {
    #[error("非法 Base64")]
    InvalidBase64,
    #[error("非 ASCII 文本")]
    NotAscii,
}

pub fn convert(
    input: &str,
    conversion: Conversion,
    charset: Charset,
    multiline: bool,
) -> Result<String, Base64TextError> {
    match conversion {
        Conversion::Encode => encode(input, charset, multiline),
        Conversion::Decode => decode(input, charset, multiline),
    }
}

pub fn encode(input: &str, charset: Charset, multiline: bool) -> Result<String, Base64TextError> {
    if multiline {
        map_lines(input, |line| encode_one(line, charset))
    } else {
        encode_one(input, charset)
    }
}

pub fn decode(input: &str, charset: Charset, multiline: bool) -> Result<String, Base64TextError> {
    if multiline {
        map_lines(input, |line| decode_one(line, charset, true))
    } else {
        decode_one(input, charset, false)
    }
}

/// Host currently passes only the payload string. Treat as Decode when it is
/// valid RFC 4648 Base64 and not a short plain word.
pub fn looks_like_base64(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.len() < 8 {
        return false;
    }
    decode_bytes(trimmed, false).is_ok()
}

fn map_lines(
    input: &str,
    mut each: impl FnMut(&str) -> Result<String, Base64TextError>,
) -> Result<String, Base64TextError> {
    let mut out = String::new();
    for (i, line) in input.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let line = line.strip_suffix('\r').unwrap_or(line);
        out.push_str(&each(line)?);
    }
    Ok(out)
}

fn encode_one(input: &str, charset: Charset) -> Result<String, Base64TextError> {
    let bytes = to_bytes(input, charset)?;
    Ok(STANDARD.encode(bytes))
}

fn decode_one(
    input: &str,
    charset: Charset,
    ignore_whitespace: bool,
) -> Result<String, Base64TextError> {
    let bytes = decode_bytes(input, ignore_whitespace)?;
    from_bytes(&bytes, charset)
}

fn to_bytes(input: &str, charset: Charset) -> Result<Vec<u8>, Base64TextError> {
    match charset {
        Charset::Utf8 => Ok(input.as_bytes().to_vec()),
        Charset::Ascii => {
            if !input.is_ascii() {
                return Err(Base64TextError::NotAscii);
            }
            Ok(input.as_bytes().to_vec())
        }
    }
}

fn from_bytes(bytes: &[u8], charset: Charset) -> Result<String, Base64TextError> {
    match charset {
        Charset::Utf8 => {
            String::from_utf8(bytes.to_vec()).map_err(|_| Base64TextError::InvalidBase64)
        }
        Charset::Ascii => {
            if !bytes.is_ascii() {
                return Err(Base64TextError::NotAscii);
            }
            String::from_utf8(bytes.to_vec()).map_err(|_| Base64TextError::InvalidBase64)
        }
    }
}

fn decode_bytes(input: &str, ignore_whitespace: bool) -> Result<Vec<u8>, Base64TextError> {
    let prepared = if ignore_whitespace {
        input
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    } else {
        input.trim().to_string()
    };
    STANDARD
        .decode(prepared.as_bytes())
        .map_err(|_| Base64TextError::InvalidBase64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_encode_hello() {
        let got = encode("hello", Charset::Utf8, false).unwrap();
        assert_eq!(got, "aGVsbG8=");
    }

    #[test]
    fn utf8_decode_hello() {
        let got = decode("aGVsbG8=", Charset::Utf8, false).unwrap();
        assert_eq!(got, "hello");
    }

    #[test]
    fn invalid_base64_is_err_without_input() {
        let err = decode("not-base64!", Charset::Utf8, false).unwrap_err();
        assert_eq!(err, Base64TextError::InvalidBase64);
        let message = err.to_string();
        assert!(
            !message.contains("not-base64"),
            "error must not include user input"
        );
    }

    #[test]
    fn looks_like_base64_not_short_plain_text() {
        assert!(looks_like_base64("aGVsbG8="));
        assert!(!looks_like_base64("hello"));
    }

    #[test]
    fn multiline_decode_ignores_whitespace() {
        let got = decode("aGVs bG8=", Charset::Utf8, true).unwrap();
        assert_eq!(got, "hello");
    }
}
