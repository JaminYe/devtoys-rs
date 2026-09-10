#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokenType {
    Word,
    String,
    Reserved,
    ReservedTopLevel,
    ReservedTopLevelNoIndent,
    ReservedNewLine,
    Operator,
    OpenParen,
    CloseParen,
    LineComment,
    BlockComment,
    Number,
    PlaceHolder,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Token {
    pub index: usize,
    pub length: usize,
    pub token_type: TokenType,
    pub preceding_whitespace_length: usize,
}

impl Token {
    pub fn value<'a>(&self, query: &'a str) -> &'a str {
        &query[self.index..self.index + self.length]
    }

    pub fn with_type(self, token_type: TokenType) -> Self {
        Self { token_type, ..self }
    }

    pub fn is_and(self, query: &str) -> bool {
        self.token_type == TokenType::ReservedNewLine && eq_kw(self.value(query), "AND")
    }

    pub fn is_between(self, query: &str) -> bool {
        self.token_type == TokenType::Reserved && eq_kw(self.value(query), "BETWEEN")
    }

    pub fn is_limit(self, query: &str) -> bool {
        self.token_type == TokenType::ReservedTopLevel && eq_kw(self.value(query), "LIMIT")
    }

    pub fn is_set(self, query: &str) -> bool {
        self.token_type == TokenType::ReservedTopLevel && eq_kw(self.value(query), "SET")
    }

    pub fn is_by(self, query: &str) -> bool {
        self.token_type == TokenType::Reserved && eq_kw(self.value(query), "BY")
    }

    pub fn is_window(self, query: &str) -> bool {
        self.token_type == TokenType::ReservedTopLevel && eq_kw(self.value(query), "WINDOW")
    }

    pub fn is_end(self, query: &str) -> bool {
        self.token_type == TokenType::CloseParen && eq_kw(self.value(query), "END")
    }

    pub fn is_values(self, query: &str) -> bool {
        self.token_type == TokenType::ReservedTopLevel && eq_kw(self.value(query), "VALUES")
    }
}

pub(crate) fn eq_kw(value: &str, keyword: &str) -> bool {
    value.eq_ignore_ascii_case(keyword)
}
