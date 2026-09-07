use data_encoding::{Specification, BASE32, BASE32HEX, BASE64, BASE64URL, HEXUPPER};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NumberBase {
    #[default]
    Decimal,
    Octal,
    Hexadecimal,
    Binary,
}

impl NumberBase {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Decimal" => Some(Self::Decimal),
            "Octal" => Some(Self::Octal),
            "Hexadecimal" => Some(Self::Hexadecimal),
            "Binary" => Some(Self::Binary),
            _ => None,
        }
    }

    pub fn radix(self) -> u32 {
        match self {
            Self::Decimal => 10,
            Self::Octal => 8,
            Self::Hexadecimal => 16,
            Self::Binary => 2,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Decimal => "Decimal",
            Self::Octal => "Octal",
            Self::Hexadecimal => "Hexadecimal",
            Self::Binary => "Binary",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rfc4648Encoding {
    Base16,
    Base32,
    Base32Hex,
    Base64,
    Base64Url,
}

impl Rfc4648Encoding {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Base16" => Some(Self::Base16),
            "Base32" => Some(Self::Base32),
            "Base32Hex" => Some(Self::Base32Hex),
            "Base64" => Some(Self::Base64),
            "Base64Url" => Some(Self::Base64Url),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NumberBaseError {
    #[error("非法数字")]
    InvalidNumber,
    #[error("非法编码")]
    InvalidEncoding,
    #[error("非法自定义字符表")]
    InvalidAlphabet,
    #[error("无法解码为文本")]
    InvalidUtf8,
}

pub fn convert_base(
    input: &str,
    from: NumberBase,
    to: NumberBase,
) -> Result<String, NumberBaseError> {
    let value = parse_integer(input, from)?;
    Ok(format_integer(value, to))
}

pub fn add_thousands_separators(digits: &str) -> String {
    let (sign, rest) = match digits.strip_prefix('-') {
        Some(stripped) => ("-", stripped),
        None => ("", digits),
    };
    if rest.is_empty() {
        return digits.to_string();
    }
    let mut grouped = String::new();
    for (i, ch) in rest.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let grouped: String = grouped.chars().rev().collect();
    format!("{sign}{grouped}")
}

pub fn encode_rfc4648(input: &str, encoding: Rfc4648Encoding) -> Result<String, NumberBaseError> {
    Ok(encoding_of(encoding).encode(input.as_bytes()))
}

pub fn decode_rfc4648(input: &str, encoding: Rfc4648Encoding) -> Result<String, NumberBaseError> {
    let bytes = encoding_of(encoding)
        .decode(input.trim().as_bytes())
        .map_err(|_| NumberBaseError::InvalidEncoding)?;
    String::from_utf8(bytes).map_err(|_| NumberBaseError::InvalidUtf8)
}

pub fn convert_rfc4648(
    input: &str,
    from: Rfc4648Encoding,
    to: Rfc4648Encoding,
) -> Result<String, NumberBaseError> {
    let bytes = encoding_of(from)
        .decode(input.trim().as_bytes())
        .map_err(|_| NumberBaseError::InvalidEncoding)?;
    Ok(encoding_of(to).encode(&bytes))
}

pub fn encode_custom(input: &str, alphabet: &str) -> Result<String, NumberBaseError> {
    let encoding = custom_encoding(alphabet)?;
    Ok(encoding.encode(input.as_bytes()))
}

pub fn decode_custom(input: &str, alphabet: &str) -> Result<String, NumberBaseError> {
    let encoding = custom_encoding(alphabet)?;
    let bytes = encoding
        .decode(input.trim().as_bytes())
        .map_err(|_| NumberBaseError::InvalidEncoding)?;
    String::from_utf8(bytes).map_err(|_| NumberBaseError::InvalidUtf8)
}

pub fn looks_like_number_base(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    if let Some(rest) = strip_ignore_case(t, "0x") {
        return !rest.is_empty() && u128::from_str_radix(rest, 16).is_ok();
    }
    if let Some(rest) = strip_ignore_case(t, "0b") {
        return !rest.is_empty() && u128::from_str_radix(rest, 2).is_ok();
    }
    if let Some(rest) = strip_ignore_case(t, "0o") {
        return !rest.is_empty() && u128::from_str_radix(rest, 8).is_ok();
    }
    t.chars().all(|c| c.is_ascii_digit()) && t.parse::<u128>().is_ok()
}

fn strip_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    if head.eq_ignore_ascii_case(prefix) {
        text.get(prefix.len()..)
    } else {
        None
    }
}

fn parse_integer(input: &str, from: NumberBase) -> Result<u128, NumberBaseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(NumberBaseError::InvalidNumber);
    }
    let stripped: String = trimmed
        .chars()
        .filter(|c| *c != ',' && *c != '_' && *c != ' ')
        .collect();
    let body = match from {
        NumberBase::Hexadecimal => strip_ignore_case(&stripped, "0x").unwrap_or(&stripped),
        NumberBase::Binary => strip_ignore_case(&stripped, "0b").unwrap_or(&stripped),
        NumberBase::Octal => strip_ignore_case(&stripped, "0o").unwrap_or(&stripped),
        NumberBase::Decimal => stripped.as_str(),
    };
    u128::from_str_radix(body, from.radix()).map_err(|_| NumberBaseError::InvalidNumber)
}

fn format_integer(value: u128, to: NumberBase) -> String {
    match to {
        NumberBase::Decimal => value.to_string(),
        NumberBase::Octal => format!("{value:o}"),
        NumberBase::Hexadecimal => format!("{value:X}"),
        NumberBase::Binary => format!("{value:b}"),
    }
}

fn encoding_of(encoding: Rfc4648Encoding) -> data_encoding::Encoding {
    match encoding {
        Rfc4648Encoding::Base16 => HEXUPPER,
        Rfc4648Encoding::Base32 => BASE32,
        Rfc4648Encoding::Base32Hex => BASE32HEX,
        Rfc4648Encoding::Base64 => BASE64,
        Rfc4648Encoding::Base64Url => BASE64URL,
    }
}

fn custom_encoding(alphabet: &str) -> Result<data_encoding::Encoding, NumberBaseError> {
    let symbols = alphabet.trim();
    if symbols.is_empty() {
        return Err(NumberBaseError::InvalidAlphabet);
    }
    let mut spec = Specification::new();
    spec.symbols = symbols.to_string();
    if symbols.len() == 64 || symbols.len() == 32 {
        spec.padding = Some('=');
    }
    spec.encoding()
        .map_err(|_| NumberBaseError::InvalidAlphabet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_255_to_hex_and_binary() {
        assert_eq!(
            convert_base("255", NumberBase::Decimal, NumberBase::Hexadecimal).unwrap(),
            "FF"
        );
        assert_eq!(
            convert_base("255", NumberBase::Decimal, NumberBase::Binary).unwrap(),
            "11111111"
        );
    }

    #[test]
    fn invalid_number_is_err_without_echo() {
        let input = "xyz-not-a-number";
        let err = convert_base(input, NumberBase::Decimal, NumberBase::Hexadecimal).unwrap_err();
        assert_eq!(err, NumberBaseError::InvalidNumber);
        let message = err.to_string();
        assert!(!message.contains(input));
        assert!(!message.contains("xyz"));
    }

    #[test]
    fn base64_foo_is_literal() {
        assert_eq!(
            encode_rfc4648("foo", Rfc4648Encoding::Base64).unwrap(),
            "Zm9v"
        );
        assert_eq!(
            convert_rfc4648("Zm9v", Rfc4648Encoding::Base64, Rfc4648Encoding::Base16).unwrap(),
            "666F6F"
        );
    }

    #[test]
    fn unicode_text_is_not_number_base() {
        assert!(!looks_like_number_base("用户文本"));
    }
}
