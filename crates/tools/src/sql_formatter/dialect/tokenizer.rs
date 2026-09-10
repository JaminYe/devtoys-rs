use super::spec::{Dialect, StringType};
use super::token::{Token, TokenType};

const DEFAULT_OPERATORS: &[&str] = &["<>", "<=", ">=", "&&", "||", "!="];

pub(crate) struct Tokenizer {
    reserved_top_level: Vec<&'static str>,
    reserved_newline: Vec<&'static str>,
    reserved_top_level_no_indent: Vec<&'static str>,
    reserved_plain: Vec<&'static str>,
    string_types: &'static [StringType],
    open_parens: Vec<&'static str>,
    close_parens: Vec<&'static str>,
    indexed_placeholders: &'static [char],
    named_placeholders: &'static [char],
    line_comments: Vec<&'static str>,
    special_word_chars: &'static [char],
    operators: Vec<&'static str>,
}

impl Tokenizer {
    pub(crate) fn new(dialect: &Dialect) -> Self {
        Self {
            reserved_top_level: sort_by_len(dialect.reserved_top_level),
            reserved_newline: sort_by_len(dialect.reserved_newline),
            reserved_top_level_no_indent: sort_by_len(dialect.reserved_top_level_no_indent),
            reserved_plain: sort_by_len(dialect.reserved_words),
            string_types: dialect.string_types,
            open_parens: sort_by_len(dialect.open_parens),
            close_parens: sort_by_len(dialect.close_parens),
            indexed_placeholders: dialect.indexed_placeholders,
            named_placeholders: dialect.named_placeholders,
            line_comments: sort_by_len(dialect.line_comments),
            special_word_chars: dialect.special_word_chars,
            operators: merge_operators(dialect.extra_operators),
        }
    }

    pub(crate) fn tokenize(&self, input: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut pointer = 0;
        let mut previous: Option<Token> = None;
        while pointer < input.len() {
            let preceding = preceding_whitespace_len(&input[pointer..]);
            pointer += preceding;
            if pointer >= input.len() {
                break;
            }
            let Some((length, token_type)) = self.next_token(&input[pointer..], previous, input)
            else {
                pointer += input[pointer..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(1);
                continue;
            };
            if length == 0 {
                pointer += 1;
                continue;
            }
            let token = Token {
                index: pointer,
                length,
                token_type,
                preceding_whitespace_length: preceding,
            };
            pointer += length;
            previous = Some(token);
            tokens.push(token);
        }
        tokens
    }

    fn next_token(
        &self,
        rest: &str,
        previous: Option<Token>,
        query: &str,
    ) -> Option<(usize, TokenType)> {
        self.comment_token(rest)
            .or_else(|| string_token(rest, self.string_types))
            .or_else(|| paren_token(rest, &self.open_parens, TokenType::OpenParen))
            .or_else(|| paren_token(rest, &self.close_parens, TokenType::CloseParen))
            .or_else(|| {
                placeholder_token(rest, self.named_placeholders, self.indexed_placeholders)
            })
            .or_else(|| number_token(rest))
            .or_else(|| self.reserved_token(rest, previous, query))
            .or_else(|| word_token(rest, self.special_word_chars))
            .or_else(|| operator_token(rest, &self.operators))
    }

    fn comment_token(&self, rest: &str) -> Option<(usize, TokenType)> {
        line_comment_token(rest, &self.line_comments).or_else(|| block_comment_token(rest))
    }

    fn reserved_token(
        &self,
        rest: &str,
        previous: Option<Token>,
        query: &str,
    ) -> Option<(usize, TokenType)> {
        if let Some(prev) = previous {
            if prev.length == 1 && query.as_bytes().get(prev.index) == Some(&b'.') {
                return None;
            }
        }
        match_word_list(rest, &self.reserved_top_level, TokenType::ReservedTopLevel)
            .or_else(|| match_word_list(rest, &self.reserved_newline, TokenType::ReservedNewLine))
            .or_else(|| {
                match_word_list(
                    rest,
                    &self.reserved_top_level_no_indent,
                    TokenType::ReservedTopLevelNoIndent,
                )
            })
            .or_else(|| match_word_list(rest, &self.reserved_plain, TokenType::Reserved))
    }
}

fn sort_by_len(words: &[&'static str]) -> Vec<&'static str> {
    let mut out = words.to_vec();
    out.sort_by_key(|w| std::cmp::Reverse(w.len()));
    out
}

fn merge_operators(extra: &[&'static str]) -> Vec<&'static str> {
    let mut ops: Vec<&'static str> = DEFAULT_OPERATORS
        .iter()
        .copied()
        .chain(extra.iter().copied())
        .collect();
    ops.sort_by_key(|w| std::cmp::Reverse(w.len()));
    ops.dedup();
    ops
}

fn preceding_whitespace_len(input: &str) -> usize {
    input
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(i, _)| i)
        .unwrap_or(input.len())
}

fn line_comment_token(rest: &str, prefixes: &[&str]) -> Option<(usize, TokenType)> {
    for prefix in prefixes {
        if rest.starts_with(prefix) {
            let bytes = rest.as_bytes();
            let mut i = prefix.len();
            while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
                i += 1;
            }
            if i < bytes.len() {
                if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            return Some((i, TokenType::LineComment));
        }
    }
    None
}

fn block_comment_token(rest: &str) -> Option<(usize, TokenType)> {
    if !rest.starts_with("/*") {
        return None;
    }
    let end = rest.find("*/").map(|i| i + 2).unwrap_or(rest.len());
    Some((end, TokenType::BlockComment))
}

fn string_token(rest: &str, types: &[StringType]) -> Option<(usize, TokenType)> {
    for ty in types {
        if let Some(len) = match_string(rest, *ty) {
            return Some((len, TokenType::String));
        }
    }
    None
}

fn match_string(rest: &str, ty: StringType) -> Option<usize> {
    match ty {
        StringType::SingleQuote => match_slash_quoted(rest, "'", b'\''),
        StringType::DoubleQuote => match_slash_quoted(rest, "\"", b'"'),
        StringType::NSingle => match_slash_quoted(rest, "N'", b'\''),
        StringType::UAndSingle => match_slash_quoted(rest, "U&'", b'\''),
        StringType::UAndDouble => match_slash_quoted(rest, "U&\"", b'"'),
        StringType::Backtick => match_simple_quoted(rest, b'`', b'`'),
        StringType::Brace => match_simple_quoted(rest, b'{', b'}'),
        StringType::Brackets => match_brackets(rest),
        StringType::Dollar => match_dollar(rest),
    }
}

fn match_slash_quoted(input: &str, prefix: &str, quote: u8) -> Option<usize> {
    if !input.starts_with(prefix) {
        return None;
    }
    let bytes = input.as_bytes();
    let mut i = prefix.len();
    loop {
        loop {
            if i >= bytes.len() {
                return Some(bytes.len());
            }
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if bytes[i] == quote {
                i += 1;
                break;
            }
            i += 1;
        }
        if input.get(i..).is_some_and(|s| s.starts_with(prefix)) {
            i += prefix.len();
            continue;
        }
        return Some(i);
    }
}

fn match_simple_quoted(input: &str, open: u8, close: u8) -> Option<usize> {
    let bytes = input.as_bytes();
    if bytes.first() != Some(&open) {
        return None;
    }
    let mut i = 1;
    loop {
        while i < bytes.len() && bytes[i] != close {
            i += 1;
        }
        if i >= bytes.len() {
            return Some(bytes.len());
        }
        i += 1;
        if bytes.get(i) == Some(&open) {
            i += 1;
            continue;
        }
        return Some(i);
    }
}

fn match_brackets(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    if bytes.first() != Some(&b'[') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && bytes[i] != b']' {
        i += 1;
    }
    if i >= bytes.len() {
        return Some(bytes.len());
    }
    i += 1;
    while bytes.get(i) == Some(&b']') {
        i += 1;
        while i < bytes.len() && bytes[i] != b']' {
            i += 1;
        }
        if i >= bytes.len() {
            return Some(bytes.len());
        }
        i += 1;
    }
    Some(i)
}

fn match_dollar(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    if bytes.first() != Some(&b'$') {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    if bytes.get(i) != Some(&b'$') {
        return None;
    }
    i += 1;
    let tag = &input[..i];
    Some(
        input[i..]
            .find(tag)
            .map(|rel| i + rel + tag.len())
            .unwrap_or(input.len()),
    )
}

fn paren_token(rest: &str, parens: &[&str], token_type: TokenType) -> Option<(usize, TokenType)> {
    for paren in parens {
        if paren.chars().count() == 1 {
            if rest.starts_with(paren) {
                return Some((paren.len(), token_type));
            }
        } else if let Some(len) = match_keyword(rest, paren) {
            return Some((len, token_type));
        }
    }
    None
}

fn placeholder_token(
    rest: &str,
    named: &[char],
    indexed: &[char],
) -> Option<(usize, TokenType)> {
    named_placeholder(rest, named)
        .or_else(|| indexed_placeholder(rest, indexed))
        .map(|len| (len, TokenType::PlaceHolder))
}

fn named_placeholder(rest: &str, types: &[char]) -> Option<usize> {
    if types.is_empty() {
        return None;
    }
    let mut chars = rest.chars();
    let first = chars.next()?;
    if !types.contains(&first) {
        return None;
    }
    let mut len = first.len_utf8();
    let mut n = 0;
    for c in chars {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '$') {
            len += c.len_utf8();
            n += 1;
        } else {
            break;
        }
    }
    (n > 0).then_some(len)
}

fn indexed_placeholder(rest: &str, types: &[char]) -> Option<usize> {
    if types.is_empty() {
        return None;
    }
    let mut chars = rest.chars();
    let first = chars.next()?;
    if !types.contains(&first) {
        return None;
    }
    let mut len = first.len_utf8();
    for c in chars {
        if c.is_ascii_digit() {
            len += 1;
        } else {
            break;
        }
    }
    Some(len)
}

fn number_token(rest: &str) -> Option<(usize, TokenType)> {
    match_number(rest).map(|len| (len, TokenType::Number))
}

fn match_number(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'0' && matches!(bytes[1], b'x' | b'X') {
        let mut i = 2;
        while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
            i += 1;
        }
        if i > 2 && is_word_boundary_at(rest, i) {
            return Some(i);
        }
    }
    if bytes.len() >= 3 && bytes[0] == b'0' && matches!(bytes[1], b'b' | b'B') {
        let mut i = 2;
        while i < bytes.len() && matches!(bytes[i], b'0' | b'1') {
            i += 1;
        }
        if i > 2 && is_word_boundary_at(rest, i) {
            return Some(i);
        }
    }
    let mut i = 0;
    if bytes.first() == Some(&b'-') {
        i = 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
    }
    let digits = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits {
        return None;
    }
    if bytes.get(i) == Some(&b'.') && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
    }
    if matches!(bytes.get(i), Some(&b'e' | &b'E')) {
        let mut j = i + 1;
        if bytes.get(j) == Some(&b'-') {
            j += 1;
        }
        let exp = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp {
            if bytes.get(j) == Some(&b'.') && bytes.get(j + 1).is_some_and(u8::is_ascii_digit) {
                j += 1;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
            }
            i = j;
        }
    }
    is_word_boundary_at(rest, i).then_some(i)
}

fn word_token(rest: &str, special: &[char]) -> Option<(usize, TokenType)> {
    let mut end = None;
    for (i, c) in rest.char_indices() {
        if is_ident_char(c, special) {
            end = Some(i + c.len_utf8());
        } else {
            break;
        }
    }
    end.map(|len| (len, TokenType::Word))
}

fn is_ident_char(c: char, special: &[char]) -> bool {
    special.contains(&c) || c.is_alphabetic() || c.is_numeric() || c == '_'
}

fn operator_token(rest: &str, operators: &[&str]) -> Option<(usize, TokenType)> {
    for op in operators {
        if rest.starts_with(op) {
            return Some((op.len(), TokenType::Operator));
        }
    }
    rest.chars().next().map(|c| (c.len_utf8(), TokenType::Operator))
}

fn match_word_list(rest: &str, words: &[&str], token_type: TokenType) -> Option<(usize, TokenType)> {
    for word in words {
        if let Some(len) = match_keyword(rest, word) {
            return Some((len, token_type));
        }
    }
    None
}

fn match_keyword(input: &str, keyword: &str) -> Option<usize> {
    let mut pos = 0;
    let mut first = true;
    for part in keyword.split(' ') {
        if !first {
            let rest = input.get(pos..)?;
            let ws = rest
                .char_indices()
                .find(|(_, c)| !c.is_whitespace())
                .map(|(i, _)| i)
                .unwrap_or(rest.len());
            if ws == 0 {
                return None;
            }
            pos += ws;
        }
        first = false;
        let rest = input.get(pos..)?;
        let part_len = part.len();
        if !rest
            .get(..part_len)
            .is_some_and(|s| s.eq_ignore_ascii_case(part))
        {
            return None;
        }
        pos += part_len;
    }
    is_word_boundary_at(input, pos).then_some(pos)
}

fn is_word_boundary_at(input: &str, pos: usize) -> bool {
    let before = input.get(..pos).and_then(|s| s.chars().next_back());
    let after = input.get(pos..).and_then(|s| s.chars().next());
    is_word_char(before) != is_word_char(after)
}

fn is_word_char(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}
