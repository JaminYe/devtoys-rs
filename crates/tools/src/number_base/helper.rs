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

/// Basic-mode signedness. Default `Signed` keeps two's complement (current behavior).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Signedness {
    #[default]
    Signed,
    Unsigned,
}

impl Signedness {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "Signed" => Some(Self::Signed),
            "Unsigned" => Some(Self::Unsigned),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Signed => "Signed",
            Self::Unsigned => "Unsigned",
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

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Base16 => "Base16",
            Self::Base32 => "Base32",
            Self::Base32Hex => "Base32Hex",
            Self::Base64 => "Base64",
            Self::Base64Url => "Base64Url",
        }
    }

    /// Unsigned-long dictionary (DevToys `UnsignedLongNumberBaseDefinition`).
    pub fn dictionary(self) -> &'static str {
        match self {
            Self::Base16 => "0123456789ABCDEF",
            Self::Base32 => "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
            Self::Base32Hex => "0123456789ABCDEFGHIJKLMNOPQRSTUV",
            Self::Base64 => "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
            Self::Base64Url => "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_",
        }
    }

    pub fn case_sensitive(self) -> bool {
        !matches!(self, Self::Base16)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NumberBaseError {
    #[error("非法数字")]
    InvalidNumber,
    #[error("非法字符")]
    InvalidEncoding,
    #[error("非法自定义字符表")]
    InvalidAlphabet,
}

/// Convert `input` from `from` to `to`.
///
/// `Signedness::Signed` (default): signed 64-bit two's complement
/// (`.NET Convert.ToInt64` / `ToString(long, toBase)`).
/// A leading minus, or a hex/oct/bin magnitude in `[2^63, 2^64)`, is an `i64`.
/// Negative hex/oct/bin output has no minus sign; decimal keeps it.
/// Other non-negative integers stay unsigned `u128` (does not shrink that range).
///
/// `Signedness::Unsigned`: unsigned 64-bit (`0..=u64::MAX`). No minus, no
/// two's complement. Values outside that range are invalid.
pub fn convert_base(
    input: &str,
    from: NumberBase,
    to: NumberBase,
    signedness: Signedness,
) -> Result<String, NumberBaseError> {
    let value = parse_integer(input, from, signedness)?;
    Ok(format_integer(value, to))
}

/// Four basic-base fields plus error. GUI view state; testable without egui.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BasicBaseFields {
    pub decimal: String,
    pub hexadecimal: String,
    pub octal: String,
    pub binary: String,
    pub error: Option<String>,
}

impl BasicBaseFields {
    pub fn field(&self, base: NumberBase) -> &str {
        match base {
            NumberBase::Decimal => &self.decimal,
            NumberBase::Hexadecimal => &self.hexadecimal,
            NumberBase::Octal => &self.octal,
            NumberBase::Binary => &self.binary,
        }
    }

    pub fn field_mut(&mut self, base: NumberBase) -> &mut String {
        match base {
            NumberBase::Decimal => &mut self.decimal,
            NumberBase::Hexadecimal => &mut self.hexadecimal,
            NumberBase::Octal => &mut self.octal,
            NumberBase::Binary => &mut self.binary,
        }
    }

    fn clear_others(&mut self, from: NumberBase) {
        for base in [
            NumberBase::Decimal,
            NumberBase::Hexadecimal,
            NumberBase::Octal,
            NumberBase::Binary,
        ] {
            if base != from {
                self.field_mut(base).clear();
            }
        }
    }

    /// Keep the source field text. Empty input clears others with no error;
    /// illegal or out-of-range input also clears others and sets the error.
    pub fn apply_input(&mut self, from: NumberBase, thousands: bool, signedness: Signedness) {
        let source = self.field(from).to_string();
        if source.trim().is_empty() {
            self.error = None;
            self.clear_others(from);
            return;
        }
        let mut converted = Vec::new();
        for to in [
            NumberBase::Decimal,
            NumberBase::Hexadecimal,
            NumberBase::Octal,
            NumberBase::Binary,
        ] {
            if to == from {
                continue;
            }
            match convert_base(&source, from, to, signedness) {
                Ok(mut text) => {
                    if thousands {
                        text = add_thousands_separators(&text);
                    }
                    converted.push((to, text));
                }
                Err(err) => {
                    self.error = Some(err.to_string());
                    self.clear_others(from);
                    return;
                }
            }
        }
        self.error = None;
        for (to, text) in converted {
            *self.field_mut(to) = text;
        }
    }
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

/// Convert `input` between RFC4648 dictionaries as an unsigned 64-bit integer.
/// This is dictionary radix conversion, not a byte codec (`FF` → Base64 is `D/`, not `/w==`).
pub fn convert_rfc4648(
    input: &str,
    from: Rfc4648Encoding,
    to: Rfc4648Encoding,
) -> Result<String, NumberBaseError> {
    let value = parse_dictionary_ulong(input, from.dictionary(), from.case_sensitive())?;
    format_dictionary_ulong(value, to.dictionary())
}

/// Format `value` with a case-sensitive custom dictionary (ulong radix).
pub fn encode_custom(value: u64, alphabet: &str) -> Result<String, NumberBaseError> {
    format_dictionary_ulong(value, alphabet)
}

/// Parse `input` with a case-sensitive custom dictionary (ulong radix).
pub fn decode_custom(input: &str, alphabet: &str) -> Result<u64, NumberBaseError> {
    parse_dictionary_ulong(input, alphabet, true)
}

/// Convert `input` between two case-sensitive custom dictionaries via ulong.
pub fn convert_custom(
    input: &str,
    from_alphabet: &str,
    to_alphabet: &str,
) -> Result<String, NumberBaseError> {
    let value = decode_custom(input, from_alphabet)?;
    encode_custom(value, to_alphabet)
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
    if t.starts_with('-') {
        return t.parse::<i64>().is_ok();
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

/// Magnitude in `[2^63, 2^64)`: 64-bit two's complement with the sign bit set.
const SIGNED_64_MIN: u128 = 1u128 << 63;
const UNSIGNED_64_MAX: u128 = u64::MAX as u128;

enum ParsedInt {
    /// `i64::MIN..=-1`, from a leading minus or 64-bit sign-bit hex/oct/bin.
    Signed(i64),
    /// `0..=u128::MAX`, including positives that exceed `i64::MAX`.
    Unsigned(u128),
}

fn parse_integer(
    input: &str,
    from: NumberBase,
    signedness: Signedness,
) -> Result<ParsedInt, NumberBaseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(NumberBaseError::InvalidNumber);
    }
    let stripped: String = trimmed
        .chars()
        .filter(|c| *c != ',' && *c != '_' && *c != ' ')
        .collect();
    let (negative, after_sign) = match stripped.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, stripped.as_str()),
    };
    if after_sign.is_empty() {
        return Err(NumberBaseError::InvalidNumber);
    }
    let body = match from {
        NumberBase::Hexadecimal => strip_ignore_case(after_sign, "0x").unwrap_or(after_sign),
        NumberBase::Binary => strip_ignore_case(after_sign, "0b").unwrap_or(after_sign),
        NumberBase::Octal => strip_ignore_case(after_sign, "0o").unwrap_or(after_sign),
        NumberBase::Decimal => after_sign,
    };
    if body.is_empty() {
        return Err(NumberBaseError::InvalidNumber);
    }
    if signedness == Signedness::Unsigned {
        if negative {
            return Err(NumberBaseError::InvalidNumber);
        }
        let value =
            u128::from_str_radix(body, from.radix()).map_err(|_| NumberBaseError::InvalidNumber)?;
        if value > UNSIGNED_64_MAX {
            return Err(NumberBaseError::InvalidNumber);
        }
        return Ok(ParsedInt::Unsigned(value));
    }
    if negative {
        let mut signed = String::with_capacity(body.len() + 1);
        signed.push('-');
        signed.push_str(body);
        let value = i64::from_str_radix(&signed, from.radix())
            .map_err(|_| NumberBaseError::InvalidNumber)?;
        if value < 0 {
            Ok(ParsedInt::Signed(value))
        } else {
            Ok(ParsedInt::Unsigned(0))
        }
    } else {
        let value =
            u128::from_str_radix(body, from.radix()).map_err(|_| NumberBaseError::InvalidNumber)?;
        if from != NumberBase::Decimal && (SIGNED_64_MIN..=UNSIGNED_64_MAX).contains(&value) {
            Ok(ParsedInt::Signed(value as u64 as i64))
        } else {
            Ok(ParsedInt::Unsigned(value))
        }
    }
}

fn format_integer(value: ParsedInt, to: NumberBase) -> String {
    match value {
        ParsedInt::Signed(signed) => match to {
            NumberBase::Decimal => signed.to_string(),
            NumberBase::Octal => format!("{:o}", signed as u64),
            NumberBase::Hexadecimal => format!("{:X}", signed as u64),
            NumberBase::Binary => format!("{:b}", signed as u64),
        },
        ParsedInt::Unsigned(unsigned) => match to {
            NumberBase::Decimal => unsigned.to_string(),
            NumberBase::Octal => format!("{unsigned:o}"),
            NumberBase::Hexadecimal => format!("{unsigned:X}"),
            NumberBase::Binary => format!("{unsigned:b}"),
        },
    }
}

fn parse_dictionary_ulong(
    input: &str,
    dictionary: &str,
    case_sensitive: bool,
) -> Result<u64, NumberBaseError> {
    let symbols: Vec<char> = dictionary.chars().collect();
    if symbols.len() < 2 {
        return Err(NumberBaseError::InvalidAlphabet);
    }
    let base = symbols.len() as u64;
    let max_value = u64::MAX / base;
    let mut value: u64 = 0;
    let mut saw_digit = false;
    for ch in input.chars() {
        if ch == ' ' {
            continue;
        }
        saw_digit = true;
        let index = dictionary_index(ch, &symbols, case_sensitive)
            .ok_or(NumberBaseError::InvalidEncoding)?;
        if value > max_value {
            return Err(NumberBaseError::InvalidNumber);
        }
        let new_value = value.wrapping_mul(base).wrapping_add(index);
        if new_value < value {
            return Err(NumberBaseError::InvalidNumber);
        }
        value = new_value;
    }
    if !saw_digit {
        return Err(NumberBaseError::InvalidNumber);
    }
    Ok(value)
}

fn format_dictionary_ulong(value: u64, dictionary: &str) -> Result<String, NumberBaseError> {
    let symbols: Vec<char> = dictionary.chars().collect();
    if symbols.len() < 2 {
        return Err(NumberBaseError::InvalidAlphabet);
    }
    if value == 0 {
        return Ok(symbols[0].to_string());
    }
    let base = symbols.len() as u64;
    let mut n = value;
    let mut digits = Vec::new();
    while n > 0 {
        digits.push(symbols[(n % base) as usize]);
        n /= base;
    }
    Ok(digits.into_iter().rev().collect())
}

fn dictionary_index(ch: char, symbols: &[char], case_sensitive: bool) -> Option<u64> {
    let target = if case_sensitive {
        ch
    } else {
        ch.to_ascii_lowercase()
    };
    for (i, &symbol) in symbols.iter().enumerate() {
        let candidate = if case_sensitive {
            symbol
        } else {
            symbol.to_ascii_lowercase()
        };
        if candidate == target {
            return Some(i as u64);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Existing tests keep signed (default) via this 3-arg wrapper.
    fn convert_base(
        input: &str,
        from: NumberBase,
        to: NumberBase,
    ) -> Result<String, NumberBaseError> {
        super::convert_base(input, from, to, Signedness::Signed)
    }

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
    fn rfc4648_is_ulong_dictionary_radix_not_byte_codec() {
        assert_eq!(
            convert_rfc4648("FF", Rfc4648Encoding::Base16, Rfc4648Encoding::Base64).unwrap(),
            "D/"
        );
        assert_eq!(
            convert_rfc4648("F F", Rfc4648Encoding::Base16, Rfc4648Encoding::Base64).unwrap(),
            "D/"
        );
        assert_eq!(
            convert_rfc4648("ff", Rfc4648Encoding::Base16, Rfc4648Encoding::Base64).unwrap(),
            "D/"
        );
        assert_eq!(
            convert_rfc4648("D/", Rfc4648Encoding::Base64, Rfc4648Encoding::Base16).unwrap(),
            "FF"
        );
        assert_eq!(
            convert_rfc4648("FF", Rfc4648Encoding::Base16, Rfc4648Encoding::Base32).unwrap(),
            "H7"
        );
        assert_eq!(
            convert_rfc4648("H7", Rfc4648Encoding::Base32, Rfc4648Encoding::Base16).unwrap(),
            "FF"
        );
        assert_eq!(
            convert_rfc4648("FF", Rfc4648Encoding::Base16, Rfc4648Encoding::Base64Url).unwrap(),
            "D_"
        );
        assert_eq!(
            convert_rfc4648("D_", Rfc4648Encoding::Base64Url, Rfc4648Encoding::Base16).unwrap(),
            "FF"
        );
        assert_eq!(
            convert_rfc4648("FF", Rfc4648Encoding::Base16, Rfc4648Encoding::Base32Hex).unwrap(),
            "7V"
        );
        assert_eq!(
            convert_custom("1111", "01", "0123456789ABCDEF").unwrap(),
            "F"
        );
        assert_eq!(decode_custom("1111", "01").unwrap(), 15);
        assert_eq!(encode_custom(15, "01").unwrap(), "1111");
        assert_eq!(
            convert_rfc4648("", Rfc4648Encoding::Base16, Rfc4648Encoding::Base64).unwrap_err(),
            NumberBaseError::InvalidNumber
        );
        assert_eq!(
            convert_rfc4648("   ", Rfc4648Encoding::Base16, Rfc4648Encoding::Base64).unwrap_err(),
            NumberBaseError::InvalidNumber
        );
        assert_eq!(
            convert_rfc4648("GG", Rfc4648Encoding::Base16, Rfc4648Encoding::Base64).unwrap_err(),
            NumberBaseError::InvalidEncoding
        );
        assert_eq!(
            convert_rfc4648("h7", Rfc4648Encoding::Base32, Rfc4648Encoding::Base16).unwrap_err(),
            NumberBaseError::InvalidEncoding
        );
        assert_eq!(
            convert_custom("2", "01", "0123456789ABCDEF").unwrap_err(),
            NumberBaseError::InvalidEncoding
        );
        assert_eq!(
            convert_custom("1111", "0", "01").unwrap_err(),
            NumberBaseError::InvalidAlphabet
        );
        assert_eq!(
            convert_base("-1", NumberBase::Decimal, NumberBase::Hexadecimal).unwrap(),
            "FFFFFFFFFFFFFFFF"
        );
    }

    #[test]
    fn unicode_text_is_not_number_base() {
        assert!(!looks_like_number_base("用户文本"));
    }

    #[test]
    fn looks_like_number_base_accepts_decimal_minus_one() {
        assert!(looks_like_number_base("-1"));
        assert!(looks_like_number_base(" -1 "));
        assert!(looks_like_number_base("-9223372036854775808"));
        assert!(!looks_like_number_base("-"));
        assert!(!looks_like_number_base("-9223372036854775809"));
    }

    #[test]
    fn decimal_minus_one_is_64bit_twos_complement() {
        // Convert.ToString(-1L, 16/8/2)
        assert_eq!(
            convert_base("-1", NumberBase::Decimal, NumberBase::Hexadecimal).unwrap(),
            "FFFFFFFFFFFFFFFF"
        );
        assert_eq!(
            convert_base("-1", NumberBase::Decimal, NumberBase::Octal).unwrap(),
            "1777777777777777777777"
        );
        assert_eq!(
            convert_base("-1", NumberBase::Decimal, NumberBase::Binary).unwrap(),
            "1111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(
            convert_base("-1", NumberBase::Decimal, NumberBase::Decimal).unwrap(),
            "-1"
        );
    }

    #[test]
    fn hex_fffffffffffffffff6_is_signed_minus_ten() {
        // Upstream NumberBaseHelperTests ConvertFromHexadecimalWithFormatting
        assert_eq!(
            convert_base(
                "FFFFFFFFFFFFFFF6",
                NumberBase::Hexadecimal,
                NumberBase::Decimal
            )
            .unwrap(),
            "-10"
        );
        assert_eq!(
            convert_base(
                "FFFF FFFF FFFF FFF6",
                NumberBase::Hexadecimal,
                NumberBase::Decimal
            )
            .unwrap(),
            "-10"
        );
        assert_eq!(
            convert_base(
                "FFFFFFFFFFFFFFF6",
                NumberBase::Hexadecimal,
                NumberBase::Octal
            )
            .unwrap(),
            "1777777777777777777766"
        );
        assert_eq!(
            convert_base(
                "FFFFFFFFFFFFFFF6",
                NumberBase::Hexadecimal,
                NumberBase::Binary
            )
            .unwrap(),
            "1111111111111111111111111111111111111111111111111111111111110110"
        );
    }

    #[test]
    fn zero_is_zero_in_all_bases() {
        for from in [
            NumberBase::Decimal,
            NumberBase::Hexadecimal,
            NumberBase::Octal,
            NumberBase::Binary,
        ] {
            for to in [
                NumberBase::Decimal,
                NumberBase::Hexadecimal,
                NumberBase::Octal,
                NumberBase::Binary,
            ] {
                assert_eq!(convert_base("0", from, to).unwrap(), "0");
            }
        }
    }

    #[test]
    fn i64_min_and_max_round_trip() {
        // Convert.ToString(long.MinValue / MaxValue, 16/8/2) and ToInt64
        assert_eq!(
            convert_base(
                "-9223372036854775808",
                NumberBase::Decimal,
                NumberBase::Hexadecimal
            )
            .unwrap(),
            "8000000000000000"
        );
        assert_eq!(
            convert_base(
                "-9223372036854775808",
                NumberBase::Decimal,
                NumberBase::Octal
            )
            .unwrap(),
            "1000000000000000000000"
        );
        assert_eq!(
            convert_base(
                "-9223372036854775808",
                NumberBase::Decimal,
                NumberBase::Binary
            )
            .unwrap(),
            "1000000000000000000000000000000000000000000000000000000000000000"
        );
        assert_eq!(
            convert_base(
                "8000000000000000",
                NumberBase::Hexadecimal,
                NumberBase::Decimal
            )
            .unwrap(),
            "-9223372036854775808"
        );
        assert_eq!(
            convert_base(
                "9223372036854775807",
                NumberBase::Decimal,
                NumberBase::Hexadecimal
            )
            .unwrap(),
            "7FFFFFFFFFFFFFFF"
        );
        assert_eq!(
            convert_base(
                "9223372036854775807",
                NumberBase::Decimal,
                NumberBase::Octal
            )
            .unwrap(),
            "777777777777777777777"
        );
        assert_eq!(
            convert_base(
                "9223372036854775807",
                NumberBase::Decimal,
                NumberBase::Binary
            )
            .unwrap(),
            "111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(
            convert_base(
                "7FFFFFFFFFFFFFFF",
                NumberBase::Hexadecimal,
                NumberBase::Decimal
            )
            .unwrap(),
            "9223372036854775807"
        );
    }

    #[test]
    fn unsigned_u128_range_is_preserved() {
        // 2^64 — exceeds i64, still unsigned u128
        assert_eq!(
            convert_base(
                "18446744073709551616",
                NumberBase::Decimal,
                NumberBase::Hexadecimal
            )
            .unwrap(),
            "10000000000000000"
        );
        assert_eq!(
            convert_base(
                "10000000000000000",
                NumberBase::Hexadecimal,
                NumberBase::Decimal
            )
            .unwrap(),
            "18446744073709551616"
        );
        assert_eq!(
            convert_base("FF", NumberBase::Hexadecimal, NumberBase::Decimal).unwrap(),
            "255"
        );
    }

    #[test]
    fn prefixes_and_separators_still_work() {
        assert_eq!(
            convert_base("0xFF", NumberBase::Hexadecimal, NumberBase::Decimal).unwrap(),
            "255"
        );
        assert_eq!(
            convert_base("0b11111111", NumberBase::Binary, NumberBase::Decimal).unwrap(),
            "255"
        );
        assert_eq!(
            convert_base("0o377", NumberBase::Octal, NumberBase::Decimal).unwrap(),
            "255"
        );
        assert_eq!(
            convert_base("1,000", NumberBase::Decimal, NumberBase::Hexadecimal).unwrap(),
            "3E8"
        );
        assert_eq!(
            convert_base("-1,000", NumberBase::Decimal, NumberBase::Decimal).unwrap(),
            "-1000"
        );
        assert_eq!(add_thousands_separators("-18006427676"), "-18,006,427,676");
    }

    #[test]
    fn out_of_range_is_invalid_without_echo() {
        let too_big = "340282366920938463463374607431768211456";
        let err = convert_base(too_big, NumberBase::Decimal, NumberBase::Hexadecimal).unwrap_err();
        assert_eq!(err, NumberBaseError::InvalidNumber);
        assert!(!err.to_string().contains(too_big));

        let too_small = "-9223372036854775809";
        let err =
            convert_base(too_small, NumberBase::Decimal, NumberBase::Hexadecimal).unwrap_err();
        assert_eq!(err, NumberBaseError::InvalidNumber);
        assert!(!err.to_string().contains(too_small));
    }

    #[test]
    fn upstream_negative_decimal_and_octal_literals() {
        // ConvertFromDecimalWithFormatting / ConvertFromOctalWithFormatting (unformatted)
        assert_eq!(
            convert_base("-18006427676", NumberBase::Decimal, NumberBase::Hexadecimal).unwrap(),
            "FFFFFFFBCEBBB7E4"
        );
        assert_eq!(
            convert_base(
                "1777777777747124257253",
                NumberBase::Octal,
                NumberBase::Decimal
            )
            .unwrap(),
            "-3333333333"
        );
    }

    const ALL_BASES: [NumberBase; 4] = [
        NumberBase::Decimal,
        NumberBase::Hexadecimal,
        NumberBase::Octal,
        NumberBase::Binary,
    ];

    fn valid_255(base: NumberBase) -> &'static str {
        match base {
            NumberBase::Decimal => "255",
            NumberBase::Hexadecimal => "FF",
            NumberBase::Octal => "377",
            NumberBase::Binary => "11111111",
        }
    }

    fn illegal_chars(base: NumberBase) -> &'static str {
        match base {
            NumberBase::Decimal => "abc",
            NumberBase::Hexadecimal => "GG",
            NumberBase::Octal => "8",
            NumberBase::Binary => "2",
        }
    }

    fn overflow_pos(base: NumberBase) -> String {
        match base {
            NumberBase::Decimal => "340282366920938463463374607431768211456".into(),
            NumberBase::Hexadecimal => format!("1{}", "0".repeat(32)),
            NumberBase::Octal => format!("4{}", "0".repeat(42)),
            NumberBase::Binary => format!("1{}", "0".repeat(128)),
        }
    }

    fn overflow_neg(base: NumberBase) -> String {
        match base {
            NumberBase::Decimal => "-9223372036854775809".into(),
            NumberBase::Hexadecimal => "-8000000000000001".into(),
            NumberBase::Octal => format!("-1{}1", "0".repeat(20)),
            NumberBase::Binary => format!("-1{}1", "0".repeat(62)),
        }
    }

    fn assert_fields_are_255(fields: &BasicBaseFields) {
        assert_eq!(fields.decimal, "255");
        assert_eq!(fields.hexadecimal, "FF");
        assert_eq!(fields.octal, "377");
        assert_eq!(fields.binary, "11111111");
        assert_eq!(fields.error, None);
    }

    fn assert_others_cleared(fields: &BasicBaseFields, from: NumberBase) {
        for base in ALL_BASES {
            if base != from {
                assert!(
                    fields.field(base).is_empty(),
                    "{from:?} as source should clear {base:?}, got {:?}",
                    fields.field(base)
                );
            }
        }
    }

    fn apply_from(fields: &mut BasicBaseFields, from: NumberBase, text: &str) {
        apply_from_with(fields, from, text, false, Signedness::Signed);
    }

    fn apply_from_with(
        fields: &mut BasicBaseFields,
        from: NumberBase,
        text: &str,
        thousands: bool,
        signedness: Signedness,
    ) {
        *fields.field_mut(from) = text.to_string();
        fields.apply_input(from, thousands, signedness);
    }

    #[test]
    fn apply_input_each_base_clears_on_illegal_overflow_empty_and_recovers() {
        for from in ALL_BASES {
            let mut fields = BasicBaseFields::default();
            apply_from(&mut fields, from, valid_255(from));
            assert_fields_are_255(&fields);

            let illegal = illegal_chars(from);
            apply_from(&mut fields, from, illegal);
            assert_eq!(fields.field(from), illegal);
            assert_others_cleared(&fields, from);
            assert_eq!(fields.error.as_deref(), Some("非法数字"));

            apply_from(&mut fields, from, valid_255(from));
            assert_fields_are_255(&fields);

            let too_big = overflow_pos(from);
            apply_from(&mut fields, from, &too_big);
            assert_eq!(fields.field(from), too_big);
            assert_others_cleared(&fields, from);
            assert_eq!(fields.error.as_deref(), Some("非法数字"));

            apply_from(&mut fields, from, valid_255(from));
            assert_fields_are_255(&fields);

            let too_small = overflow_neg(from);
            apply_from(&mut fields, from, &too_small);
            assert_eq!(fields.field(from), too_small.as_str());
            assert_others_cleared(&fields, from);
            assert_eq!(fields.error.as_deref(), Some("非法数字"));

            apply_from(&mut fields, from, valid_255(from));
            assert_fields_are_255(&fields);

            apply_from(&mut fields, from, "");
            assert_eq!(fields.field(from), "");
            assert_others_cleared(&fields, from);
            assert_eq!(fields.error, None);

            apply_from(&mut fields, from, valid_255(from));
            apply_from(&mut fields, from, "  ");
            assert_eq!(fields.field(from), "  ");
            assert_others_cleared(&fields, from);
            assert_eq!(fields.error, None);
        }
    }

    #[test]
    fn apply_input_negative_one_still_twos_complement_then_clears_and_recovers() {
        let mut fields = BasicBaseFields::default();
        apply_from(&mut fields, NumberBase::Decimal, "-1");
        assert_eq!(fields.decimal, "-1");
        assert_eq!(fields.hexadecimal, "FFFFFFFFFFFFFFFF");
        assert_eq!(fields.octal, "1777777777777777777777");
        assert_eq!(
            fields.binary,
            "1111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(fields.error, None);

        apply_from(&mut fields, NumberBase::Decimal, "abc");
        assert_eq!(fields.decimal, "abc");
        assert_others_cleared(&fields, NumberBase::Decimal);
        assert_eq!(fields.error.as_deref(), Some("非法数字"));

        apply_from(&mut fields, NumberBase::Decimal, "-1");
        assert_eq!(fields.hexadecimal, "FFFFFFFFFFFFFFFF");
        assert_eq!(fields.error, None);
    }

    #[test]
    fn unsigned_hex_all_f_is_u64_max_round_trip() {
        assert_eq!(
            super::convert_base(
                "FFFFFFFFFFFFFFFF",
                NumberBase::Hexadecimal,
                NumberBase::Decimal,
                Signedness::Unsigned
            )
            .unwrap(),
            "18446744073709551615"
        );
        assert_eq!(
            super::convert_base(
                "18446744073709551615",
                NumberBase::Decimal,
                NumberBase::Hexadecimal,
                Signedness::Unsigned
            )
            .unwrap(),
            "FFFFFFFFFFFFFFFF"
        );
        let hex = super::convert_base(
            "18446744073709551615",
            NumberBase::Decimal,
            NumberBase::Hexadecimal,
            Signedness::Unsigned,
        )
        .unwrap();
        assert!(!hex.contains('-'), "{hex}");
        assert_eq!(
            super::convert_base(
                "FFFFFFFFFFFFFFFF",
                NumberBase::Hexadecimal,
                NumberBase::Octal,
                Signedness::Unsigned
            )
            .unwrap(),
            "1777777777777777777777"
        );
        assert_eq!(
            super::convert_base(
                "FFFFFFFFFFFFFFFF",
                NumberBase::Hexadecimal,
                NumberBase::Binary,
                Signedness::Unsigned
            )
            .unwrap(),
            "1111111111111111111111111111111111111111111111111111111111111111"
        );
    }

    #[test]
    fn signed_decimal_minus_one_is_still_twos_complement() {
        assert_eq!(
            super::convert_base(
                "-1",
                NumberBase::Decimal,
                NumberBase::Hexadecimal,
                Signedness::Signed
            )
            .unwrap(),
            "FFFFFFFFFFFFFFFF"
        );
        assert_eq!(
            super::convert_base(
                "-1",
                NumberBase::Decimal,
                NumberBase::Hexadecimal,
                Signedness::default()
            )
            .unwrap(),
            "FFFFFFFFFFFFFFFF"
        );
    }

    #[test]
    fn unsigned_rejects_minus_and_overflow() {
        assert_eq!(
            super::convert_base(
                "-1",
                NumberBase::Decimal,
                NumberBase::Hexadecimal,
                Signedness::Unsigned
            )
            .unwrap_err(),
            NumberBaseError::InvalidNumber
        );
        assert_eq!(
            super::convert_base(
                "18446744073709551616",
                NumberBase::Decimal,
                NumberBase::Hexadecimal,
                Signedness::Unsigned
            )
            .unwrap_err(),
            NumberBaseError::InvalidNumber
        );
        assert_eq!(
            super::convert_base(
                "10000000000000000",
                NumberBase::Hexadecimal,
                NumberBase::Decimal,
                Signedness::Unsigned
            )
            .unwrap_err(),
            NumberBaseError::InvalidNumber
        );
        let too_big = "FFFFFFFFFFFFFFFFF";
        let err = super::convert_base(
            too_big,
            NumberBase::Hexadecimal,
            NumberBase::Decimal,
            Signedness::Unsigned,
        )
        .unwrap_err();
        assert_eq!(err, NumberBaseError::InvalidNumber);
        assert!(!err.to_string().contains(too_big));
    }

    #[test]
    fn apply_input_unsigned_all_f_round_trip_overflow_and_thousands() {
        let mut fields = BasicBaseFields::default();
        apply_from_with(
            &mut fields,
            NumberBase::Hexadecimal,
            "FFFFFFFFFFFFFFFF",
            false,
            Signedness::Unsigned,
        );
        assert_eq!(fields.decimal, "18446744073709551615");
        assert_eq!(fields.hexadecimal, "FFFFFFFFFFFFFFFF");
        assert_eq!(fields.octal, "1777777777777777777777");
        assert_eq!(
            fields.binary,
            "1111111111111111111111111111111111111111111111111111111111111111"
        );
        assert!(!fields.decimal.contains('-'));
        assert!(!fields.hexadecimal.contains('-'));
        assert_eq!(fields.error, None);

        apply_from_with(
            &mut fields,
            NumberBase::Decimal,
            "18446744073709551615",
            false,
            Signedness::Unsigned,
        );
        assert_eq!(fields.hexadecimal, "FFFFFFFFFFFFFFFF");
        assert!(!fields.hexadecimal.contains('-'));
        assert_eq!(fields.error, None);

        apply_from_with(
            &mut fields,
            NumberBase::Hexadecimal,
            "FFFFFFFFFFFFFFFF",
            true,
            Signedness::Unsigned,
        );
        assert_eq!(fields.decimal, "18,446,744,073,709,551,615");
        assert_eq!(fields.error, None);

        apply_from_with(
            &mut fields,
            NumberBase::Hexadecimal,
            "FFFFFFFFFFFFFFFFF",
            false,
            Signedness::Unsigned,
        );
        assert_eq!(fields.hexadecimal, "FFFFFFFFFFFFFFFFF");
        assert_others_cleared(&fields, NumberBase::Hexadecimal);
        assert_eq!(fields.error.as_deref(), Some("非法数字"));

        apply_from_with(
            &mut fields,
            NumberBase::Hexadecimal,
            "FFFFFFFFFFFFFFFF",
            false,
            Signedness::Unsigned,
        );
        assert_eq!(fields.decimal, "18446744073709551615");
        assert_eq!(fields.error, None);

        apply_from_with(
            &mut fields,
            NumberBase::Decimal,
            "-1",
            false,
            Signedness::Unsigned,
        );
        assert_eq!(fields.decimal, "-1");
        assert_others_cleared(&fields, NumberBase::Decimal);
        assert_eq!(fields.error.as_deref(), Some("非法数字"));

        apply_from_with(
            &mut fields,
            NumberBase::Decimal,
            "abc",
            false,
            Signedness::Unsigned,
        );
        assert_eq!(fields.decimal, "abc");
        assert_others_cleared(&fields, NumberBase::Decimal);
        assert_eq!(fields.error.as_deref(), Some("非法数字"));
    }
}
