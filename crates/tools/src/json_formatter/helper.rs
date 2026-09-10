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
    let mut value: Value = parse_json_value(input)?;
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

struct Parser<'a> {
    input: &'a str,
    bytes: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            pos: 0,
            depth: 0,
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), JsonFormatError> {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => {
                    self.pos += 1;
                }
                b'/' => {
                    if self.pos + 1 >= self.bytes.len() {
                        return Err(JsonFormatError::InvalidJson);
                    }
                    match self.bytes[self.pos + 1] {
                        b'/' => {
                            self.pos += 2;
                            while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                                self.pos += 1;
                            }
                        }
                        b'*' => {
                            self.pos += 2;
                            let mut closed = false;
                            while self.pos + 1 < self.bytes.len() {
                                if self.bytes[self.pos] == b'*' && self.bytes[self.pos + 1] == b'/'
                                {
                                    self.pos += 2;
                                    closed = true;
                                    break;
                                }
                                self.pos += 1;
                            }
                            if !closed {
                                return Err(JsonFormatError::InvalidJson);
                            }
                        }
                        _ => return Err(JsonFormatError::InvalidJson),
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn parse_value(&mut self) -> Result<Value, JsonFormatError> {
        self.skip_whitespace_and_comments()?;
        if self.pos >= self.bytes.len() {
            return Err(JsonFormatError::InvalidJson);
        }
        match self.bytes[self.pos] {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => self.parse_string().map(Value::String),
            b't' => self.parse_literal("true", Value::Bool(true)),
            b'f' => self.parse_literal("false", Value::Bool(false)),
            b'n' => self.parse_literal("null", Value::Null),
            b'-' | b'0'..=b'9' => self.parse_number().map(Value::Number),
            _ => Err(JsonFormatError::InvalidJson),
        }
    }

    fn parse_literal(&mut self, expected: &str, val: Value) -> Result<Value, JsonFormatError> {
        if self.input[self.pos..].starts_with(expected) {
            self.pos += expected.len();
            Ok(val)
        } else {
            Err(JsonFormatError::InvalidJson)
        }
    }

    fn parse_string(&mut self) -> Result<String, JsonFormatError> {
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'"' {
            return Err(JsonFormatError::InvalidJson);
        }
        let start = self.pos;
        self.pos += 1;
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'\\' => {
                    self.pos += 1;
                    if self.pos >= self.bytes.len() {
                        return Err(JsonFormatError::InvalidJson);
                    }
                    self.pos += 1;
                }
                b'"' => {
                    self.pos += 1;
                    let slice = &self.input[start..self.pos];
                    return serde_json::from_str::<String>(slice)
                        .map_err(|_| JsonFormatError::InvalidJson);
                }
                b'\0'..=b'\x1f' => {
                    return Err(JsonFormatError::InvalidJson);
                }
                _ => {
                    self.pos += 1;
                }
            }
        }
        Err(JsonFormatError::InvalidJson)
    }

    fn parse_number(&mut self) -> Result<serde_json::Number, JsonFormatError> {
        let start = self.pos;
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'-' {
            self.pos += 1;
        }
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-' => {
                    self.pos += 1;
                }
                _ => break,
            }
        }
        let slice = &self.input[start..self.pos];
        serde_json::from_str::<serde_json::Number>(slice).map_err(|_| JsonFormatError::InvalidJson)
    }

    fn parse_array(&mut self) -> Result<Value, JsonFormatError> {
        if self.depth > 128 {
            return Err(JsonFormatError::InvalidJson);
        }
        self.depth += 1;
        self.pos += 1; // consume '['
        self.skip_whitespace_and_comments()?;
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b']' {
            self.pos += 1;
            self.depth -= 1;
            return Ok(Value::Array(Vec::new()));
        }

        let mut items = Vec::new();
        loop {
            let item = self.parse_value()?;
            items.push(item);
            self.skip_whitespace_and_comments()?;
            if self.pos >= self.bytes.len() {
                return Err(JsonFormatError::InvalidJson);
            }
            match self.bytes[self.pos] {
                b',' => {
                    self.pos += 1;
                    self.skip_whitespace_and_comments()?;
                    if self.pos < self.bytes.len() && self.bytes[self.pos] == b']' {
                        self.pos += 1;
                        self.depth -= 1;
                        return Ok(Value::Array(items));
                    }
                }
                b']' => {
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(JsonFormatError::InvalidJson),
            }
        }
    }

    fn parse_object(&mut self) -> Result<Value, JsonFormatError> {
        if self.depth > 128 {
            return Err(JsonFormatError::InvalidJson);
        }
        self.depth += 1;
        self.pos += 1; // consume '{'
        self.skip_whitespace_and_comments()?;
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'}' {
            self.pos += 1;
            self.depth -= 1;
            return Ok(Value::Object(serde_json::Map::new()));
        }

        let mut map = serde_json::Map::new();
        loop {
            self.skip_whitespace_and_comments()?;
            let key = self.parse_string()?;
            self.skip_whitespace_and_comments()?;
            if self.pos >= self.bytes.len() || self.bytes[self.pos] != b':' {
                return Err(JsonFormatError::InvalidJson);
            }
            self.pos += 1; // consume ':'
            let val = self.parse_value()?;

            // Issue 06: first duplicate key wins
            map.entry(key).or_insert(val);

            self.skip_whitespace_and_comments()?;
            if self.pos >= self.bytes.len() {
                return Err(JsonFormatError::InvalidJson);
            }
            match self.bytes[self.pos] {
                b',' => {
                    self.pos += 1;
                    self.skip_whitespace_and_comments()?;
                    if self.pos < self.bytes.len() && self.bytes[self.pos] == b'}' {
                        self.pos += 1;
                        self.depth -= 1;
                        return Ok(Value::Object(map));
                    }
                }
                b'}' => {
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(Value::Object(map));
                }
                _ => return Err(JsonFormatError::InvalidJson),
            }
        }
    }
}

fn parse_json_value(input: &str) -> Result<Value, JsonFormatError> {
    let mut parser = Parser::new(input);
    let val = parser.parse_value()?;
    parser.skip_whitespace_and_comments()?;
    if parser.pos != parser.bytes.len() {
        return Err(JsonFormatError::InvalidJson);
    }
    Ok(val)
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
        let got = format_json(r#"{"z":{"b":1,"a":2},"a":0}"#, Indentation::Minified, true).unwrap();
        assert_eq!(got, r#"{"a":0,"z":{"a":2,"b":1}}"#);
    }

    #[test]
    fn invalid_json_is_err_without_input() {
        let err = format_json("{", Indentation::TwoSpaces, false).unwrap_err();
        assert_eq!(err, JsonFormatError::InvalidJson);
        let message = err.to_string();
        assert!(!message.contains('{'), "error must not include user input");
    }

    #[test]
    fn unquoted_key_is_invalid_json() {
        let err = format_json("{n:1}", Indentation::Minified, false).unwrap_err();
        assert_eq!(err, JsonFormatError::InvalidJson);
        assert_eq!(err.to_string(), "非法 JSON");
    }

    #[test]
    fn ordinary_integers_stay_decimal() {
        let got = format_json(
            r#"{"n":42,"m":-7,"z":0,"u":18446744073709551615,"i":-9223372036854775808}"#,
            Indentation::Minified,
            false,
        )
        .unwrap();
        assert_eq!(
            got,
            r#"{"n":42,"m":-7,"z":0,"u":18446744073709551615,"i":-9223372036854775808}"#
        );
    }

    #[test]
    fn ordinary_float_stays_decimal() {
        let got = format_json(r#"{"n":1.5,"m":-3.14}"#, Indentation::Minified, false).unwrap();
        assert_eq!(got, r#"{"n":1.5,"m":-3.14}"#);
    }

    #[test]
    fn exponent_form_stays_explicit() {
        let got = format_json(r#"{"n":1e10,"m":1.23e-4}"#, Indentation::Minified, false).unwrap();
        assert_eq!(got, r#"{"n":1e+10,"m":1.23e-4}"#);
    }

    #[test]
    fn positive_integer_beyond_u64_minified() {
        let got = format_json(
            r#"{"n":18446744073709551617}"#,
            Indentation::Minified,
            false,
        )
        .unwrap();
        assert_eq!(got, r#"{"n":18446744073709551617}"#);
        assert!(!got.contains('.'), "{got}");
        assert!(!got.contains('e'), "{got}");
        assert!(!got.contains("$serde_json::private::Number"), "{got}");
    }

    #[test]
    fn negative_integer_below_i64_minified() {
        let got = format_json(
            r#"{"n":-9223372036854775809}"#,
            Indentation::Minified,
            false,
        )
        .unwrap();
        assert_eq!(got, r#"{"n":-9223372036854775809}"#);
        assert!(!got.contains('.'), "{got}");
        assert!(!got.contains("$serde_json::private::Number"), "{got}");
    }

    #[test]
    fn nested_object_and_array_keep_large_integers() {
        let got = format_json(
            r#"{"z":{"b":18446744073709551617,"a":[-9223372036854775809]},"m":42}"#,
            Indentation::Minified,
            false,
        )
        .unwrap();
        assert_eq!(
            got,
            r#"{"z":{"b":18446744073709551617,"a":[-9223372036854775809]},"m":42}"#
        );
    }

    #[test]
    fn large_integers_two_spaces_unsorted() {
        let got = format_json(
            r#"{"z":{"b":18446744073709551617,"a":[-9223372036854775809]},"m":42}"#,
            Indentation::TwoSpaces,
            false,
        )
        .unwrap();
        assert_eq!(
            got,
            "{\n  \"z\": {\n    \"b\": 18446744073709551617,\n    \"a\": [\n      -9223372036854775809\n    ]\n  },\n  \"m\": 42\n}"
        );
    }

    #[test]
    fn large_integers_four_spaces_unsorted() {
        let got = format_json(
            r#"{"z":{"b":18446744073709551617,"a":[-9223372036854775809]},"m":42}"#,
            Indentation::FourSpaces,
            false,
        )
        .unwrap();
        assert_eq!(
            got,
            "{\n    \"z\": {\n        \"b\": 18446744073709551617,\n        \"a\": [\n            -9223372036854775809\n        ]\n    },\n    \"m\": 42\n}"
        );
    }

    #[test]
    fn large_integers_one_tab_unsorted() {
        let got = format_json(
            r#"{"z":{"b":18446744073709551617,"a":[-9223372036854775809]},"m":42}"#,
            Indentation::OneTab,
            false,
        )
        .unwrap();
        assert_eq!(
            got,
            "{\n\t\"z\": {\n\t\t\"b\": 18446744073709551617,\n\t\t\"a\": [\n\t\t\t-9223372036854775809\n\t\t]\n\t},\n\t\"m\": 42\n}"
        );
    }

    #[test]
    fn large_integers_minified_sorted() {
        let got = format_json(
            r#"{"z":{"b":18446744073709551617,"a":[-9223372036854775809]},"m":42}"#,
            Indentation::Minified,
            true,
        )
        .unwrap();
        assert_eq!(
            got,
            r#"{"m":42,"z":{"a":[-9223372036854775809],"b":18446744073709551617}}"#
        );
    }

    #[test]
    fn large_integers_two_spaces_sorted() {
        let got = format_json(
            r#"{"z":{"b":18446744073709551617,"a":[-9223372036854775809]},"m":42}"#,
            Indentation::TwoSpaces,
            true,
        )
        .unwrap();
        assert_eq!(
            got,
            "{\n  \"m\": 42,\n  \"z\": {\n    \"a\": [\n      -9223372036854775809\n    ],\n    \"b\": 18446744073709551617\n  }\n}"
        );
    }

    #[test]
    fn large_integers_four_spaces_sorted() {
        let got = format_json(
            r#"{"z":{"b":18446744073709551617,"a":[-9223372036854775809]},"m":42}"#,
            Indentation::FourSpaces,
            true,
        )
        .unwrap();
        assert_eq!(
            got,
            "{\n    \"m\": 42,\n    \"z\": {\n        \"a\": [\n            -9223372036854775809\n        ],\n        \"b\": 18446744073709551617\n    }\n}"
        );
    }

    #[test]
    fn large_integers_one_tab_sorted() {
        let got = format_json(
            r#"{"z":{"b":18446744073709551617,"a":[-9223372036854775809]},"m":42}"#,
            Indentation::OneTab,
            true,
        )
        .unwrap();
        assert_eq!(
            got,
            "{\n\t\"m\": 42,\n\t\"z\": {\n\t\t\"a\": [\n\t\t\t-9223372036854775809\n\t\t],\n\t\t\"b\": 18446744073709551617\n\t}\n}"
        );
    }

    #[test]
    fn duplicate_key_root_keeps_first_value() {
        let got = format_json(r#"{"a":1,"a":2}"#, Indentation::Minified, false).unwrap();
        assert_eq!(got, r#"{"a":1}"#);
    }

    #[test]
    fn duplicate_key_nested_and_array() {
        let input = r#"{"nested":{"a":1,"a":2},"arr":[{"a":1,"a":2},{"x":10,"x":20}]}"#;
        let got = format_json(input, Indentation::Minified, false).unwrap();
        assert_eq!(got, r#"{"nested":{"a":1},"arr":[{"a":1},{"x":10}]}"#);
    }

    #[test]
    fn duplicate_key_preserves_other_properties() {
        let input = r#"{"a":1,"b":2,"a":3,"c":4,"b":5}"#;
        let got = format_json(input, Indentation::Minified, false).unwrap();
        assert_eq!(got, r#"{"a":1,"b":2,"c":4}"#);
    }

    #[test]
    fn duplicate_key_with_sort_properties() {
        let input = r#"{"b":1,"a":2,"b":3,"a":4}"#;
        let got = format_json(input, Indentation::Minified, true).unwrap();
        assert_eq!(got, r#"{"a":2,"b":1}"#);
    }

    #[test]
    fn duplicate_key_all_indentations() {
        let input = r#"{"a":1,"a":2}"#;
        let two_spaces = format_json(input, Indentation::TwoSpaces, false).unwrap();
        assert_eq!(two_spaces, "{\n  \"a\": 1\n}");

        let four_spaces = format_json(input, Indentation::FourSpaces, false).unwrap();
        assert_eq!(four_spaces, "{\n    \"a\": 1\n}");

        let one_tab = format_json(input, Indentation::OneTab, false).unwrap();
        assert_eq!(one_tab, "{\n\t\"a\": 1\n}");

        let minified = format_json(input, Indentation::Minified, false).unwrap();
        assert_eq!(minified, r#"{"a":1}"#);
    }

    #[test]
    fn duplicate_key_preserves_large_integers() {
        let input = r#"{"n":18446744073709551617,"n":42}"#;
        let got = format_json(input, Indentation::Minified, false).unwrap();
        assert_eq!(got, r#"{"n":18446744073709551617}"#);

        let input_neg = r#"{"n":-9223372036854775809,"n":-10}"#;
        let got_neg = format_json(input_neg, Indentation::Minified, false).unwrap();
        assert_eq!(got_neg, r#"{"n":-9223372036854775809}"#);
    }

    #[test]
    fn duplicate_key_complex_values() {
        let input = r#"{"a":{"inner":1},"a":[2,3]}"#;
        let got = format_json(input, Indentation::Minified, false).unwrap();
        assert_eq!(got, r#"{"a":{"inner":1}}"#);
    }

    #[test]
    fn accepts_block_comment_and_trailing_comma() {
        let got = format_json(r#"{/*c*/"a":1,}"#, Indentation::Minified, false).unwrap();
        assert_eq!(got, r#"{"a":1}"#);
    }

    #[test]
    fn accepts_line_comments_and_trailing_commas_in_objects_and_arrays() {
        let input = "// top line comment\n{\n  // key comment\n  \"a\": 1, // value comment\n  \"b\": [\n    // array item comment\n    1,\n    2, // trailing comma in array\n  ], // trailing comma in object\n}\n// bottom line comment";
        let got = format_json(input, Indentation::Minified, false).unwrap();
        assert_eq!(got, r#"{"a":1,"b":[1,2]}"#);
    }

    #[test]
    fn accepts_block_comments_in_arbitrary_positions() {
        let input = "/*1*/{/*2*/\"a\"/*3*/:/*4*/[/*5*/1/*6*/,/*7*/2/*8*/,/*9*/]/*10*/,/*11*/}";
        let got = format_json(input, Indentation::Minified, false).unwrap();
        assert_eq!(got, r#"{"a":[1,2]}"#);
    }

    #[test]
    fn string_with_comment_markers_and_escapes_preserved() {
        let input = r#"{"url":"http://example.com/*not_a_comment*/","code":"// not a comment","escaped":"quote \" and slash \\","comma":"a,b,c"}"#;
        let got = format_json(input, Indentation::Minified, false).unwrap();
        assert_eq!(got, input);
    }

    #[test]
    fn unclosed_block_comment_is_error() {
        let err = format_json(r#"{"a": 1 /* unclosed}"#, Indentation::Minified, false).unwrap_err();
        assert_eq!(err, JsonFormatError::InvalidJson);
    }

    #[test]
    fn unclosed_string_is_error() {
        let err = format_json(r#"{"a": "unclosed}"#, Indentation::Minified, false).unwrap_err();
        assert_eq!(err, JsonFormatError::InvalidJson);
    }

    #[test]
    fn unclosed_object_or_array_is_error() {
        assert_eq!(
            format_json(r#"{"a": 1"#, Indentation::Minified, false).unwrap_err(),
            JsonFormatError::InvalidJson
        );
        assert_eq!(
            format_json(r#"[1, 2"#, Indentation::Minified, false).unwrap_err(),
            JsonFormatError::InvalidJson
        );
    }

    #[test]
    fn empty_elements_or_multiple_commas_are_error() {
        assert_eq!(
            format_json(r#"[,]"#, Indentation::Minified, false).unwrap_err(),
            JsonFormatError::InvalidJson
        );
        assert_eq!(
            format_json(r#"{,}"#, Indentation::Minified, false).unwrap_err(),
            JsonFormatError::InvalidJson
        );
        assert_eq!(
            format_json(r#"[1,,2]"#, Indentation::Minified, false).unwrap_err(),
            JsonFormatError::InvalidJson
        );
        assert_eq!(
            format_json(r#"{"a": 1,,}"#, Indentation::Minified, false).unwrap_err(),
            JsonFormatError::InvalidJson
        );
    }

    #[test]
    fn stray_slash_is_error() {
        let err = format_json(r#"{"a": 1 / 2}"#, Indentation::Minified, false).unwrap_err();
        assert_eq!(err, JsonFormatError::InvalidJson);
    }

    #[test]
    fn only_comments_is_error() {
        assert_eq!(
            format_json("// just a line comment\n", Indentation::Minified, false).unwrap_err(),
            JsonFormatError::InvalidJson
        );
        assert_eq!(
            format_json("/* just a block comment */", Indentation::Minified, false).unwrap_err(),
            JsonFormatError::InvalidJson
        );
    }

    #[test]
    fn trailing_characters_after_valid_json_is_error() {
        let err = format_json(r#"{"a": 1} trailing"#, Indentation::Minified, false).unwrap_err();
        assert_eq!(err, JsonFormatError::InvalidJson);
    }

    #[test]
    fn trailing_unclosed_block_comment_is_error() {
        let err = format_json(r#"{"a": 1} /* unclosed"#, Indentation::Minified, false).unwrap_err();
        assert_eq!(err, JsonFormatError::InvalidJson);
    }

    #[test]
    fn line_comment_at_eof_without_trailing_newline_is_ok() {
        let got = format_json(
            r#"{"a": 1} // comment at EOF without newline"#,
            Indentation::Minified,
            false,
        )
        .unwrap();
        assert_eq!(got, r#"{"a":1}"#);
    }

    #[test]
    fn deeply_nested_json_recursion_limit_is_error() {
        let deep = "[".repeat(200) + &"]".repeat(200);
        let err = format_json(&deep, Indentation::Minified, false).unwrap_err();
        assert_eq!(err, JsonFormatError::InvalidJson);
    }
}
