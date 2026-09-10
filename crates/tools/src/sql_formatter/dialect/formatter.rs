use crate::indent::Indentation;

use super::spec::OverrideKind;
use super::token::{Token, TokenType};

const INLINE_MAX_LENGTH: usize = 50;
const LINES_BETWEEN_QUERIES: usize = 2;

pub(crate) fn format(
    query: &str,
    indent: Indentation,
    tokens: &[Token],
    override_kind: OverrideKind,
) -> String {
    if query.is_empty() {
        return String::new();
    }
    let mut fmt = Formatter {
        query,
        tokens,
        indent_unit: indent_unit(indent),
        override_kind,
        indentation: Vec::new(),
        inline_level: 0,
        previous_reserved: None,
        index: 0,
        out: String::new(),
    };
    fmt.emit();
    fmt.out.trim().to_string()
}

fn indent_unit(indent: Indentation) -> &'static str {
    match indent {
        Indentation::TwoSpaces | Indentation::Minified => "  ",
        Indentation::FourSpaces => "    ",
        Indentation::OneTab => "\t",
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IndentKind {
    TopLevel,
    BlockLevel,
}

struct Formatter<'a> {
    query: &'a str,
    tokens: &'a [Token],
    indent_unit: &'a str,
    override_kind: OverrideKind,
    indentation: Vec<IndentKind>,
    inline_level: i32,
    previous_reserved: Option<Token>,
    index: usize,
    out: String,
}

impl<'a> Formatter<'a> {
    fn emit(&mut self) {
        for i in 0..self.tokens.len() {
            self.index = i;
            let token = self.token_override(self.tokens[i]);
            match token.token_type {
                TokenType::LineComment => self.format_line_comment(token),
                TokenType::BlockComment => self.format_block_comment(token),
                TokenType::ReservedTopLevel => {
                    self.format_top_level_reserved(token);
                    self.previous_reserved = Some(token);
                }
                TokenType::ReservedTopLevelNoIndent => {
                    self.format_top_level_reserved_no_indent(token);
                    self.previous_reserved = Some(token);
                }
                TokenType::ReservedNewLine => {
                    self.format_newline_reserved(token);
                    self.previous_reserved = Some(token);
                }
                TokenType::Reserved => {
                    self.format_with_spaces(token);
                    self.previous_reserved = Some(token);
                }
                TokenType::OpenParen => self.format_opening_paren(token),
                TokenType::CloseParen => self.format_closing_paren(token),
                TokenType::PlaceHolder => self.format_placeholder(token),
                _ => {
                    let value = token.value(self.query);
                    match value {
                        "," => self.format_comma(token),
                        ":" => self.format_with_space_after(token),
                        "." => self.format_without_spaces(token),
                        ";" => self.format_query_separator(token),
                        _ => self.format_with_spaces(token),
                    }
                }
            }
        }
    }

    fn token_override(&self, token: Token) -> Token {
        match self.override_kind {
            OverrideKind::None => token,
            OverrideKind::PlSql => {
                if token.is_set(self.query) {
                    if let Some(prev) = self.previous_reserved {
                        if prev.is_by(self.query) {
                            return token.with_type(TokenType::Reserved);
                        }
                    }
                }
                token
            }
            OverrideKind::Spark => {
                if token.is_window(self.query) {
                    if let Some(ahead) = self.look_ahead(1) {
                        if ahead.token_type == TokenType::OpenParen {
                            return token.with_type(TokenType::Reserved);
                        }
                    }
                }
                if token.is_end(self.query) {
                    if let Some(back) = self.look_behind(1) {
                        if back.token_type == TokenType::Operator
                            && back.length == 1
                            && self.query.as_bytes().get(back.index) == Some(&b'.')
                        {
                            return token.with_type(TokenType::Word);
                        }
                    }
                }
                token
            }
        }
    }

    fn look_behind(&self, n: usize) -> Option<Token> {
        self.index
            .checked_sub(n)
            .and_then(|i| self.tokens.get(i).copied())
    }

    fn look_ahead(&self, n: usize) -> Option<Token> {
        self.tokens.get(self.index + n).copied()
    }

    fn format_line_comment(&mut self, token: Token) {
        self.append_token(token);
        self.add_newline();
    }

    fn format_block_comment(&mut self, token: Token) {
        self.add_newline();
        self.out.push_str(&indent_comment(
            token.value(self.query),
            &self.current_indent(),
        ));
        self.add_newline();
    }

    fn format_top_level_reserved_no_indent(&mut self, token: Token) {
        self.decrease_top_level();
        self.add_newline();
        self.append_token(token);
        self.add_newline();
    }

    fn format_top_level_reserved(&mut self, token: Token) {
        self.decrease_top_level();
        self.add_newline();
        self.increase_top_level();
        self.append_token(token);
        self.add_newline();
    }

    fn format_newline_reserved(&mut self, token: Token) {
        if token.is_and(self.query) {
            if let Some(t) = self.look_behind(2) {
                if t.is_between(self.query) {
                    self.format_with_spaces(token);
                    return;
                }
            }
        }
        self.add_newline();
        self.append_token(token);
        self.out.push(' ');
    }

    fn format_opening_paren(&mut self, token: Token) {
        if token.preceding_whitespace_length == 0 {
            let behind = self.look_behind(1);
            let keep_space = matches!(
                behind.map(|t| t.token_type),
                Some(TokenType::OpenParen | TokenType::LineComment | TokenType::Operator)
            ) || behind.is_some_and(|t| t.is_values(self.query));
            if !keep_space {
                trim_space_end(&mut self.out);
            }
        }
        self.append_token(token);
        self.begin_inline_if_possible();
        if !self.inline_active() {
            self.increase_block_level();
            self.add_newline();
        }
    }

    fn format_closing_paren(&mut self, token: Token) {
        if self.inline_active() {
            self.inline_level -= 1;
            self.format_with_space_after(token);
        } else {
            self.decrease_block_level();
            self.add_newline();
            self.format_with_spaces(token);
        }
    }

    fn format_placeholder(&mut self, token: Token) {
        self.out.push_str(token.value(self.query));
        self.out.push(' ');
    }

    fn format_comma(&mut self, token: Token) {
        trim_space_end(&mut self.out);
        self.append_token(token);
        self.out.push(' ');
        if self.inline_active() {
            return;
        }
        if self
            .previous_reserved
            .is_some_and(|t| t.is_limit(self.query))
        {
            return;
        }
        self.add_newline();
    }

    fn format_with_space_after(&mut self, token: Token) {
        trim_space_end(&mut self.out);
        self.append_token(token);
        self.out.push(' ');
    }

    fn format_without_spaces(&mut self, token: Token) {
        trim_space_end(&mut self.out);
        self.append_token(token);
    }

    fn format_with_spaces(&mut self, token: Token) {
        self.append_token(token);
        self.out.push(' ');
    }

    fn format_query_separator(&mut self, token: Token) {
        self.indentation.clear();
        trim_space_end(&mut self.out);
        self.append_token(token);
        for _ in 0..LINES_BETWEEN_QUERIES {
            self.out.push('\n');
        }
    }

    fn append_token(&mut self, token: Token) {
        let value = token.value(self.query);
        if matches!(
            token.token_type,
            TokenType::Reserved
                | TokenType::ReservedTopLevel
                | TokenType::ReservedTopLevelNoIndent
                | TokenType::ReservedNewLine
                | TokenType::OpenParen
                | TokenType::CloseParen
        ) {
            for c in value.chars() {
                self.out.push(c.to_ascii_uppercase());
            }
        } else {
            self.out.push_str(value);
        }
    }

    fn add_newline(&mut self) {
        trim_space_end(&mut self.out);
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out.push_str(&self.current_indent());
    }

    fn current_indent(&self) -> String {
        self.indent_unit.repeat(self.indentation.len())
    }

    fn increase_top_level(&mut self) {
        self.indentation.push(IndentKind::TopLevel);
    }

    fn increase_block_level(&mut self) {
        self.indentation.push(IndentKind::BlockLevel);
    }

    fn decrease_top_level(&mut self) {
        if self.indentation.last() == Some(&IndentKind::TopLevel) {
            self.indentation.pop();
        }
    }

    fn decrease_block_level(&mut self) {
        while let Some(kind) = self.indentation.pop() {
            if kind != IndentKind::TopLevel {
                break;
            }
        }
    }

    fn inline_active(&self) -> bool {
        self.inline_level > 0
    }

    fn begin_inline_if_possible(&mut self) {
        if self.inline_level == 0 && is_inline_block(self.tokens, self.index) {
            self.inline_level = 1;
        } else if self.inline_level > 0 {
            self.inline_level += 1;
        } else {
            self.inline_level = 0;
        }
    }
}

fn is_inline_block(tokens: &[Token], index: usize) -> bool {
    let mut length = 0usize;
    let mut level = 0i32;
    for token in tokens.iter().skip(index) {
        length += token.length;
        if length > INLINE_MAX_LENGTH {
            return false;
        }
        match token.token_type {
            TokenType::OpenParen => level += 1,
            TokenType::CloseParen => {
                level -= 1;
                if level == 0 {
                    return true;
                }
            }
            TokenType::ReservedTopLevel | TokenType::ReservedNewLine | TokenType::BlockComment => {
                return false;
            }
            _ => {}
        }
    }
    false
}

fn indent_comment(comment: &str, indent: &str) -> String {
    let mut out = String::with_capacity(comment.len() + indent.len());
    let mut chars = comment.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            out.push('\n');
            while matches!(chars.peek(), Some(' ' | '\t')) {
                chars.next();
            }
            out.push_str(indent);
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

fn trim_space_end(s: &mut String) {
    let trimmed = s.trim_end_matches(' ').len();
    s.truncate(trimmed);
}
