use std::collections::HashMap;

use rand::seq::SliceRandom;
use rand::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    LineEndingsLf,
    LineEndingsCrlf,
    Lower,
    Upper,
    Sentence,
    Title,
    Camel,
    Pascal,
    Snake,
    Constant,
    Kebab,
    Cobol,
    Train,
    Alternating,
    Inverse,
    RandomCase,
    SortLines,
    SortLinesDesc,
    SortByLastWord,
    SortByLastWordDesc,
    ReverseLines,
    ShuffleLines,
}

pub const OPERATION_NAMES: &[&str] = &[
    "LineEndingsLf",
    "LineEndingsCrlf",
    "Lower",
    "Upper",
    "Sentence",
    "Title",
    "Camel",
    "Pascal",
    "Snake",
    "Constant",
    "Kebab",
    "Cobol",
    "Train",
    "Alternating",
    "Inverse",
    "RandomCase",
    "SortLines",
    "SortLinesDesc",
    "SortByLastWord",
    "SortByLastWordDesc",
    "ReverseLines",
    "ShuffleLines",
];

impl Operation {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "LineEndingsLf" => Self::LineEndingsLf,
            "LineEndingsCrlf" => Self::LineEndingsCrlf,
            "Lower" => Self::Lower,
            "Upper" => Self::Upper,
            "Sentence" => Self::Sentence,
            "Title" => Self::Title,
            "Camel" => Self::Camel,
            "Pascal" => Self::Pascal,
            "Snake" => Self::Snake,
            "Constant" => Self::Constant,
            "Kebab" => Self::Kebab,
            "Cobol" => Self::Cobol,
            "Train" => Self::Train,
            "Alternating" => Self::Alternating,
            "Inverse" => Self::Inverse,
            "RandomCase" => Self::RandomCase,
            "SortLines" => Self::SortLines,
            "SortLinesDesc" => Self::SortLinesDesc,
            "SortByLastWord" => Self::SortByLastWord,
            "SortByLastWordDesc" => Self::SortByLastWordDesc,
            "ReverseLines" => Self::ReverseLines,
            "ShuffleLines" => Self::ShuffleLines,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LineEndingsLf => "LineEndingsLf",
            Self::LineEndingsCrlf => "LineEndingsCrlf",
            Self::Lower => "Lower",
            Self::Upper => "Upper",
            Self::Sentence => "Sentence",
            Self::Title => "Title",
            Self::Camel => "Camel",
            Self::Pascal => "Pascal",
            Self::Snake => "Snake",
            Self::Constant => "Constant",
            Self::Kebab => "Kebab",
            Self::Cobol => "Cobol",
            Self::Train => "Train",
            Self::Alternating => "Alternating",
            Self::Inverse => "Inverse",
            Self::RandomCase => "RandomCase",
            Self::SortLines => "SortLines",
            Self::SortLinesDesc => "SortLinesDesc",
            Self::SortByLastWord => "SortByLastWord",
            Self::SortByLastWordDesc => "SortByLastWordDesc",
            Self::ReverseLines => "ReverseLines",
            Self::ShuffleLines => "ShuffleLines",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eol {
    Lf,
    Crlf,
    Mixed,
    None,
}

impl Eol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::Crlf => "CRLF",
            Self::Mixed => "混合",
            Self::None => "无",
        }
    }

    fn separator(self) -> &'static str {
        match self {
            Self::Crlf => "\r\n",
            _ => "\n",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextStats {
    pub bytes: usize,
    pub chars: usize,
    pub words: usize,
    pub sentences: usize,
    pub paragraphs: usize,
    pub lines: usize,
    pub eol: Eol,
    pub char_freq: HashMap<char, usize>,
    pub word_freq: HashMap<String, usize>,
}

pub fn stats(text: &str) -> TextStats {
    let bytes = text.len();
    let chars = text.chars().count();
    let mut words = 0usize;
    let mut sentences = 0usize;
    let mut paragraphs = 1usize;
    let mut lines = 1usize;
    let mut char_freq = HashMap::new();
    let mut word_freq = HashMap::new();
    let mut word = String::new();
    let mut is_word_start = true;
    let mut consecutive_breaks = 0usize;
    let mut sentence_has_alnum = false;
    let mut eol = Eol::None;

    let chars_vec: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars_vec.len() {
        let c = chars_vec[i];
        *char_freq.entry(c).or_insert(0) += 1;

        if is_word_separator(c) {
            if !word.is_empty() {
                *word_freq.entry(std::mem::take(&mut word)).or_insert(0) += 1;
            }
            is_word_start = true;
        } else {
            word.push(c);
            if is_word_start {
                words += 1;
                is_word_start = false;
            }
        }

        if c.is_alphanumeric() {
            sentence_has_alnum = true;
        }

        let (is_break, crlf) = is_line_break(&chars_vec, i);
        if is_break {
            lines += 1;
            consecutive_breaks += 1;
            eol = merge_eol(eol, if crlf { Eol::Crlf } else { Eol::Lf });
            if crlf {
                i += 1;
            }
        } else {
            if consecutive_breaks > 1 {
                paragraphs += 1;
            }
            consecutive_breaks = 0;
        }

        if is_sentence_terminator(c) {
            if sentence_has_alnum {
                sentences += 1;
                sentence_has_alnum = false;
            }
        }

        i += 1;
    }

    if !word.is_empty() {
        *word_freq.entry(word).or_insert(0) += 1;
    }
    if sentence_has_alnum {
        sentences += 1;
    }

    TextStats {
        bytes,
        chars,
        words,
        sentences,
        paragraphs,
        lines,
        eol,
        char_freq,
        word_freq,
    }
}

pub fn apply(text: &str, ops: &[Operation], rng: &mut impl Rng) -> String {
    let mut current = text.to_string();
    for op in ops {
        current = apply_one(&current, *op, rng);
    }
    current
}

fn apply_one(text: &str, op: Operation, rng: &mut impl Rng) -> String {
    match op {
        Operation::LineEndingsLf => convert_eol(text, false),
        Operation::LineEndingsCrlf => convert_eol(text, true),
        Operation::Lower => text.to_lowercase(),
        Operation::Upper => text.to_uppercase(),
        Operation::Sentence => to_sentence_case(text),
        Operation::Title => to_title_case(text),
        Operation::Camel => to_camel_or_pascal(text, false),
        Operation::Pascal => to_camel_or_pascal(text, true),
        Operation::Snake => snake_like(text, '_', false),
        Operation::Constant => snake_like(text, '_', true),
        Operation::Kebab => snake_like(text, '-', false),
        Operation::Cobol => snake_like(text, '-', true),
        Operation::Train => to_train_case(text),
        Operation::Alternating => to_alternating(text, true),
        Operation::Inverse => to_alternating(text, false),
        Operation::RandomCase => to_random_case(text, rng),
        Operation::SortLines => sort_lines(text, |a, b| a.cmp(b)),
        Operation::SortLinesDesc => sort_lines(text, |a, b| b.cmp(a)),
        Operation::SortByLastWord => sort_lines(text, |a, b| last_word(a).cmp(last_word(b))),
        Operation::SortByLastWordDesc => sort_lines(text, |a, b| last_word(b).cmp(last_word(a))),
        Operation::ReverseLines => {
            let (mut lines, eol, trailing) = split_lines(text);
            lines.reverse();
            join_lines(&lines, eol, trailing)
        }
        Operation::ShuffleLines => {
            let (mut lines, eol, trailing) = split_lines(text);
            lines.shuffle(rng);
            join_lines(&lines, eol, trailing)
        }
    }
}

fn is_word_separator(c: char) -> bool {
    c.is_whitespace() || c.is_ascii_punctuation()
}

fn is_sentence_terminator(c: char) -> bool {
    matches!(c, '.' | '?' | '!')
}

fn is_line_break(chars: &[char], i: usize) -> (bool, bool) {
    match chars.get(i) {
        Some('\r') if chars.get(i + 1) == Some(&'\n') => (true, true),
        Some('\r' | '\n') => (true, false),
        _ => (false, false),
    }
}

fn merge_eol(current: Eol, found: Eol) -> Eol {
    match current {
        Eol::None => found,
        other if other == found => other,
        _ => Eol::Mixed,
    }
}

fn convert_eol(text: &str, crlf: bool) -> String {
    let repl = if crlf { "\r\n" } else { "\n" };
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let (is_break, crlf_pair) = is_line_break(&chars, i);
        if is_break {
            out.push_str(repl);
            if crlf_pair {
                i += 1;
            }
        } else {
            out.push(chars[i]);
        }
        i += 1;
    }
    out
}

fn to_sentence_case(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut new_sentence = true;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let (is_break, crlf) = is_line_break(&chars, i);
        if is_sentence_terminator(c) || is_break {
            out.push(c);
            new_sentence = true;
            if crlf {
                i += 1;
                out.push('\n');
            }
        } else if c.is_alphanumeric() {
            if new_sentence {
                out.extend(c.to_uppercase());
                new_sentence = false;
            } else {
                out.extend(c.to_lowercase());
            }
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

fn to_title_case(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, c) in chars.iter().enumerate() {
        if i == 0 || !chars[i - 1].is_alphanumeric() {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn to_camel_or_pascal(text: &str, pascal: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut next_upper = pascal;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphanumeric() {
            if next_upper {
                out.extend(c.to_uppercase());
                next_upper = false;
            } else {
                out.extend(c.to_lowercase());
            }
        } else {
            let (is_break, crlf) = is_line_break(&chars, i);
            if is_break {
                next_upper = pascal;
                out.push(c);
                if crlf {
                    i += 1;
                    out.push('\n');
                }
            } else {
                next_upper = true;
            }
        }
        i += 1;
    }
    out
}

fn snake_like(text: &str, sep: char, upper: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut ignore_sep = true;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphanumeric() {
            ignore_sep = false;
            if upper {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
        } else {
            let (is_break, crlf) = is_line_break(&chars, i);
            if is_break {
                ignore_sep = true;
                out.push(c);
                if crlf {
                    i += 1;
                    out.push('\n');
                }
            } else if !ignore_sep && i + 1 < chars.len() && chars[i + 1].is_alphanumeric() {
                ignore_sep = true;
                out.push(sep);
            }
        }
        i += 1;
    }
    out
}

fn to_train_case(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut next_upper = true;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_alphanumeric() {
            if next_upper {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            next_upper = false;
        } else {
            let (is_break, crlf) = is_line_break(&chars, i);
            if is_break {
                next_upper = true;
                out.push(c);
                if crlf {
                    i += 1;
                    out.push('\n');
                }
            } else if !next_upper && i + 1 < chars.len() && chars[i + 1].is_alphanumeric() {
                next_upper = true;
                out.push('-');
            }
        }
        i += 1;
    }
    out
}

fn to_alternating(text: &str, start_lower: bool) -> String {
    let mut lower = start_lower;
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if lower {
            out.extend(c.to_lowercase());
        } else {
            out.extend(c.to_uppercase());
        }
        lower = !lower;
    }
    out
}

fn to_random_case(text: &str, rng: &mut impl Rng) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if rng.gen_bool(0.5) {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn split_lines(text: &str) -> (Vec<&str>, Eol, bool) {
    let eol = stats(text).eol;
    let trailing = text.ends_with('\n') || text.ends_with('\r');
    (text.lines().collect(), eol, trailing)
}

fn join_lines(lines: &[&str], eol: Eol, trailing: bool) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let sep = eol.separator();
    let mut out = lines.join(sep);
    if trailing {
        out.push_str(sep);
    }
    out
}

fn sort_lines(text: &str, cmp: impl Fn(&str, &str) -> std::cmp::Ordering) -> String {
    let (mut lines, eol, trailing) = split_lines(text);
    lines.sort_by(|a, b| cmp(a, b));
    join_lines(&lines, eol, trailing)
}

fn last_word(line: &str) -> &str {
    line.split_whitespace().last().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(1)
    }

    #[test]
    fn hello_world_words_and_chars() {
        let s = stats("Hello world");
        assert_eq!(s.words, 2);
        assert_eq!(s.chars, 11);
    }

    #[test]
    fn snake_of_hello_world() {
        assert_eq!(apply("Hello World", &[Operation::Snake], &mut rng()), "hello_world");
    }

    #[test]
    fn lf_conversion_of_crlf() {
        assert_eq!(
            apply("a\r\nb", &[Operation::LineEndingsLf], &mut rng()),
            "a\nb"
        );
    }

    #[test]
    fn sort_lines_literal() {
        assert_eq!(
            apply("c\nb\na", &[Operation::SortLines], &mut rng()),
            "a\nb\nc"
        );
    }

    #[test]
    fn sequential_ops_and_eol_detect() {
        let s = stats("a\r\nb");
        assert_eq!(s.eol, Eol::Crlf);
        assert_eq!(s.lines, 2);
        let mixed = stats("a\nb\r\nc");
        assert_eq!(mixed.eol, Eol::Mixed);
        assert_eq!(stats("plain").eol, Eol::None);
        assert_eq!(
            apply("Hello World", &[Operation::Pascal], &mut rng()),
            "HelloWorld"
        );
        assert_eq!(
            apply("Hello World", &[Operation::Camel], &mut rng()),
            "helloWorld"
        );
    }
}
