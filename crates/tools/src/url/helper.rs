use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};

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
pub enum UrlError {
    #[error("非法 URL 编码")]
    InvalidPercent,
}

pub fn encode(input: &str) -> String {
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()
}

pub fn decode(input: &str) -> Result<String, UrlError> {
    if has_invalid_percent(input) {
        return Err(UrlError::InvalidPercent);
    }
    percent_decode_str(input)
        .decode_utf8()
        .map(|cow| cow.into_owned())
        .map_err(|_| UrlError::InvalidPercent)
}

pub fn convert(input: &str, conversion: Conversion, multiline: bool) -> Result<String, UrlError> {
    if multiline {
        map_lines(input, |line| match conversion {
            Conversion::Encode => Ok(encode(line)),
            Conversion::Decode => decode(line),
        })
    } else {
        match conversion {
            Conversion::Encode => Ok(encode(input)),
            Conversion::Decode => decode(input),
        }
    }
}

fn map_lines(
    input: &str,
    f: impl Fn(&str) -> Result<String, UrlError>,
) -> Result<String, UrlError> {
    let mut out = String::new();
    for (i, line) in input.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let line = line.strip_suffix('\r').unwrap_or(line);
        out.push_str(&f(line)?);
    }
    Ok(out)
}

fn has_invalid_percent(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        if i + 2 >= bytes.len() || !is_hex(bytes[i + 1]) || !is_hex(bytes[i + 2]) {
            return true;
        }
        i += 3;
    }
    false
}

fn is_hex(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_space_is_percent_20() {
        let got = encode("a b");
        assert!(
            got.contains("%20"),
            "expected percent-encoded space, got {got}"
        );
    }

    #[test]
    fn encode_decode_roundtrip() {
        let source = "a b/c?x=1";
        let encoded = encode(source);
        assert_eq!(decode(&encoded).unwrap(), source);
    }

    #[test]
    fn invalid_percent_is_err() {
        let err = decode("%").unwrap_err();
        assert_eq!(err, UrlError::InvalidPercent);
        assert!(
            !err.to_string().contains('%'),
            "error must not include user input"
        );
        assert_eq!(decode("%2"), Err(UrlError::InvalidPercent));
        assert_eq!(decode("%GG"), Err(UrlError::InvalidPercent));
        assert_eq!(decode("a%2"), Err(UrlError::InvalidPercent));
    }

    #[test]
    fn valid_percent_decodes() {
        assert_eq!(decode("a%20b").unwrap(), "a b");
        assert_eq!(convert("a%20b", Conversion::Decode, false).unwrap(), "a b");
    }

    #[test]
    fn multiline_encodes_each_line() {
        let got = convert("a b\nc d", Conversion::Encode, true).unwrap();
        assert_eq!(got, "a%20b\nc%20d");
    }

    #[test]
    fn multiline_off_encodes_newline() {
        let got = convert("a b\nc", Conversion::Encode, false).unwrap();
        assert!(got.contains("%20"));
        assert!(got.contains("%0A") || got.contains("%0a"));
        assert!(!got.contains('\n'));
    }
}
