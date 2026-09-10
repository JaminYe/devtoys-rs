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
    Cr,
    Crlf,
    Mixed,
    None,
}

impl Eol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::Cr => "CR",
            Self::Crlf => "CRLF",
            Self::Mixed => "混合",
            Self::None => "无",
        }
    }

    fn separator(self) -> &'static str {
        match self {
            Self::Crlf => "\r\n",
            Self::Cr => "\r",
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
            let found = if crlf {
                Eol::Crlf
            } else if c == '\r' {
                Eol::Cr
            } else {
                Eol::Lf
            };
            eol = merge_eol(eol, found);
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
    c.is_whitespace() || is_punctuation(c)
}

fn is_punctuation(c: char) -> bool {
    let cp = c as u32;
    PUNCTUATION_RANGES
        .binary_search_by(|&(lo, hi)| {
            if hi < cp {
                std::cmp::Ordering::Less
            } else if lo > cp {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// Unicode categories Pc/Pd/Ps/Pe/Pi/Pf/Po, matching .NET `char.IsPunctuation`.
#[rustfmt::skip]
const PUNCTUATION_RANGES: &[(u32, u32)] = &[
    (0x0021, 0x0023), (0x0025, 0x002A), (0x002C, 0x002F), (0x003A, 0x003B),
    (0x003F, 0x0040), (0x005B, 0x005D), (0x005F, 0x005F), (0x007B, 0x007B),
    (0x007D, 0x007D), (0x00A1, 0x00A1), (0x00A7, 0x00A7), (0x00AB, 0x00AB),
    (0x00B6, 0x00B7), (0x00BB, 0x00BB), (0x00BF, 0x00BF), (0x037E, 0x037E),
    (0x0387, 0x0387), (0x055A, 0x055F), (0x0589, 0x058A), (0x05BE, 0x05BE),
    (0x05C0, 0x05C0), (0x05C3, 0x05C3), (0x05C6, 0x05C6), (0x05F3, 0x05F4),
    (0x0609, 0x060A), (0x060C, 0x060D), (0x061B, 0x061B), (0x061D, 0x061F),
    (0x066A, 0x066D), (0x06D4, 0x06D4), (0x0700, 0x070D), (0x07F7, 0x07F9),
    (0x0830, 0x083E), (0x085E, 0x085E), (0x0964, 0x0965), (0x0970, 0x0970),
    (0x09FD, 0x09FD), (0x0A76, 0x0A76), (0x0AF0, 0x0AF0), (0x0C77, 0x0C77),
    (0x0C84, 0x0C84), (0x0DF4, 0x0DF4), (0x0E4F, 0x0E4F), (0x0E5A, 0x0E5B),
    (0x0F04, 0x0F12), (0x0F14, 0x0F14), (0x0F3A, 0x0F3D), (0x0F85, 0x0F85),
    (0x0FD0, 0x0FD4), (0x0FD9, 0x0FDA), (0x104A, 0x104F), (0x10FB, 0x10FB),
    (0x1360, 0x1368), (0x1400, 0x1400), (0x166E, 0x166E), (0x169B, 0x169C),
    (0x16EB, 0x16ED), (0x1735, 0x1736), (0x17D4, 0x17D6), (0x17D8, 0x17DA),
    (0x1800, 0x180A), (0x1944, 0x1945), (0x1A1E, 0x1A1F), (0x1AA0, 0x1AA6),
    (0x1AA8, 0x1AAD), (0x1B5A, 0x1B60), (0x1B7D, 0x1B7E), (0x1BFC, 0x1BFF),
    (0x1C3B, 0x1C3F), (0x1C7E, 0x1C7F), (0x1CC0, 0x1CC7), (0x1CD3, 0x1CD3),
    (0x2010, 0x2027), (0x2030, 0x2043), (0x2045, 0x2051), (0x2053, 0x205E),
    (0x207D, 0x207E), (0x208D, 0x208E), (0x2308, 0x230B), (0x2329, 0x232A),
    (0x2768, 0x2775), (0x27C5, 0x27C6), (0x27E6, 0x27EF), (0x2983, 0x2998),
    (0x29D8, 0x29DB), (0x29FC, 0x29FD), (0x2CF9, 0x2CFC), (0x2CFE, 0x2CFF),
    (0x2D70, 0x2D70), (0x2E00, 0x2E2E), (0x2E30, 0x2E4F), (0x2E52, 0x2E5D),
    (0x3001, 0x3003), (0x3008, 0x3011), (0x3014, 0x301F), (0x3030, 0x3030),
    (0x303D, 0x303D), (0x30A0, 0x30A0), (0x30FB, 0x30FB), (0xA4FE, 0xA4FF),
    (0xA60D, 0xA60F), (0xA673, 0xA673), (0xA67E, 0xA67E), (0xA6F2, 0xA6F7),
    (0xA874, 0xA877), (0xA8CE, 0xA8CF), (0xA8F8, 0xA8FA), (0xA8FC, 0xA8FC),
    (0xA92E, 0xA92F), (0xA95F, 0xA95F), (0xA9C1, 0xA9CD), (0xA9DE, 0xA9DF),
    (0xAA5C, 0xAA5F), (0xAADE, 0xAADF), (0xAAF0, 0xAAF1), (0xABEB, 0xABEB),
    (0xFD3E, 0xFD3F), (0xFE10, 0xFE19), (0xFE30, 0xFE52), (0xFE54, 0xFE61),
    (0xFE63, 0xFE63), (0xFE68, 0xFE68), (0xFE6A, 0xFE6B), (0xFF01, 0xFF03),
    (0xFF05, 0xFF0A), (0xFF0C, 0xFF0F), (0xFF1A, 0xFF1B), (0xFF1F, 0xFF20),
    (0xFF3B, 0xFF3D), (0xFF3F, 0xFF3F), (0xFF5B, 0xFF5B), (0xFF5D, 0xFF5D),
    (0xFF5F, 0xFF65), (0x10100, 0x10102), (0x1039F, 0x1039F), (0x103D0, 0x103D0),
    (0x1056F, 0x1056F), (0x10857, 0x10857), (0x1091F, 0x1091F), (0x1093F, 0x1093F),
    (0x10A50, 0x10A58), (0x10A7F, 0x10A7F), (0x10AF0, 0x10AF6), (0x10B39, 0x10B3F),
    (0x10B99, 0x10B9C), (0x10EAD, 0x10EAD), (0x10F55, 0x10F59), (0x10F86, 0x10F89),
    (0x11047, 0x1104D), (0x110BB, 0x110BC), (0x110BE, 0x110C1), (0x11140, 0x11143),
    (0x11174, 0x11175), (0x111C5, 0x111C8), (0x111CD, 0x111CD), (0x111DB, 0x111DB),
    (0x111DD, 0x111DF), (0x11238, 0x1123D), (0x112A9, 0x112A9), (0x1144B, 0x1144F),
    (0x1145A, 0x1145B), (0x1145D, 0x1145D), (0x114C6, 0x114C6), (0x115C1, 0x115D7),
    (0x11641, 0x11643), (0x11660, 0x1166C), (0x116B9, 0x116B9), (0x1173C, 0x1173E),
    (0x1183B, 0x1183B), (0x11944, 0x11946), (0x119E2, 0x119E2), (0x11A3F, 0x11A46),
    (0x11A9A, 0x11A9C), (0x11A9E, 0x11AA2), (0x11B00, 0x11B09), (0x11C41, 0x11C45),
    (0x11C70, 0x11C71), (0x11EF7, 0x11EF8), (0x11F43, 0x11F4F), (0x11FFF, 0x11FFF),
    (0x12470, 0x12474), (0x12FF1, 0x12FF2), (0x16A6E, 0x16A6F), (0x16AF5, 0x16AF5),
    (0x16B37, 0x16B3B), (0x16B44, 0x16B44), (0x16E97, 0x16E9A), (0x16FE2, 0x16FE2),
    (0x1BC9F, 0x1BC9F), (0x1DA87, 0x1DA8B), (0x1E95E, 0x1E95F),
];

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
    if text.is_empty() {
        return (Vec::new(), eol, false);
    }
    let trailing = text.ends_with('\n') || text.ends_with('\r');
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            lines.push(&text[start..i]);
            i += 2;
            start = i;
        } else if bytes[i] == b'\r' || bytes[i] == b'\n' {
            lines.push(&text[start..i]);
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }
    if !trailing {
        lines.push(&text[start..]);
    }
    (lines, eol, trailing)
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
    let mut start = 0;
    let mut end = 0;
    let mut in_word = false;
    for (i, c) in line.char_indices() {
        if is_word_separator(c) {
            in_word = false;
        } else {
            if !in_word {
                start = i;
                in_word = true;
            }
            end = i + c.len_utf8();
        }
    }
    if end > start {
        &line[start..end]
    } else {
        ""
    }
}

/// One-step restore snapshot. Each transform overwrites the target; not an undo stack.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RestoreBuffer {
    original: Option<String>,
    dirty: bool,
}

impl RestoreBuffer {
    pub fn on_user_edit(&mut self, new_input: &str) {
        self.original = Some(new_input.to_string());
        self.dirty = false;
    }

    /// Snapshot `current_input` as the restore target. Caller then applies the transform.
    pub fn on_transform(&mut self, current_input: &str) {
        self.original = Some(current_input.to_string());
        self.dirty = true;
    }

    pub fn restore(&mut self) -> Option<String> {
        self.dirty = false;
        self.original.clone()
    }

    pub fn can_restore(&self) -> bool {
        self.dirty
    }
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
        assert_eq!(
            apply("Hello World", &[Operation::Snake], &mut rng()),
            "hello_world"
        );
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

    #[test]
    fn empty_text_stats_and_sort_stay_empty() {
        let s = stats("");
        assert_eq!(s.bytes, 0);
        assert_eq!(s.chars, 0);
        assert_eq!(s.words, 0);
        assert_eq!(s.sentences, 0);
        assert_eq!(s.paragraphs, 1);
        assert_eq!(s.lines, 1);
        assert_eq!(s.eol, Eol::None);
        assert!(s.word_freq.is_empty());
        assert_eq!(apply("", &[Operation::SortLines], &mut rng()), "");
        assert_eq!(apply("", &[Operation::SortByLastWord], &mut rng()), "");
        assert_eq!(apply("", &[Operation::ReverseLines], &mut rng()), "");
    }

    #[test]
    fn cr_lf_crlf_split_and_rejoin() {
        let lf = stats("a\nb");
        assert_eq!(lf.lines, 2);
        assert_eq!(lf.eol, Eol::Lf);
        let cr = stats("a\rb");
        assert_eq!(cr.lines, 2);
        assert_eq!(cr.eol, Eol::Cr);
        let crlf = stats("a\r\nb");
        assert_eq!(crlf.lines, 2);
        assert_eq!(crlf.eol, Eol::Crlf);

        assert_eq!(apply("b\na", &[Operation::SortLines], &mut rng()), "a\nb");
        assert_eq!(apply("b\ra", &[Operation::SortLines], &mut rng()), "a\rb");
        assert_eq!(
            apply("b\r\na", &[Operation::SortLines], &mut rng()),
            "a\r\nb"
        );
        assert_eq!(
            apply("b\na\n", &[Operation::SortLines], &mut rng()),
            "a\nb\n"
        );
        assert_eq!(
            apply("b\ra\r", &[Operation::SortLines], &mut rng()),
            "a\rb\r"
        );
        assert_eq!(
            apply("b\r\na\r\n", &[Operation::SortLines], &mut rng()),
            "a\r\nb\r\n"
        );
        assert_eq!(
            apply("c\rb\ra", &[Operation::ReverseLines], &mut rng()),
            "a\rb\rc"
        );
        assert_eq!(
            apply("c\r\nb\r\na", &[Operation::ReverseLines], &mut rng()),
            "a\r\nb\r\nc"
        );
    }

    #[test]
    fn unicode_punctuation_splits_words() {
        let chinese = stats("甲，乙。");
        assert_eq!(chinese.words, 2);
        assert_eq!(chinese.word_freq.get("甲"), Some(&1));
        assert_eq!(chinese.word_freq.get("乙"), Some(&1));
        assert_eq!(chinese.word_freq.len(), 2);

        let no_space = stats("aa,zz");
        assert_eq!(no_space.words, 2);
        assert_eq!(no_space.word_freq.get("aa"), Some(&1));
        assert_eq!(no_space.word_freq.get("zz"), Some(&1));
    }

    #[test]
    fn last_word_sort_uses_punctuation_boundary() {
        assert_eq!(
            apply("aa,zz\nbb,aa", &[Operation::SortByLastWord], &mut rng()),
            "bb,aa\naa,zz"
        );
        assert_eq!(
            apply("aa,zz\nbb,aa", &[Operation::SortByLastWordDesc], &mut rng()),
            "aa,zz\nbb,aa"
        );
        assert_eq!(
            apply("aa，zz\nbb，aa", &[Operation::SortByLastWord], &mut rng()),
            "bb，aa\naa，zz"
        );
    }

    #[test]
    fn empty_lines_are_kept_when_sorting() {
        assert_eq!(
            apply("c\n\na", &[Operation::SortLines], &mut rng()),
            "\na\nc"
        );
        assert_eq!(
            apply("c\r\ra", &[Operation::SortLines], &mut rng()),
            "\ra\rc"
        );
        assert_eq!(
            apply("c\r\n\r\na", &[Operation::SortLines], &mut rng()),
            "\r\na\r\nc"
        );
    }

    #[test]
    fn restore_after_upper_returns_pre_transform_text() {
        let mut buf = RestoreBuffer::default();
        buf.on_transform("ab\ncd");
        assert_eq!(apply("ab\ncd", &[Operation::Upper], &mut rng()), "AB\nCD");
        assert!(buf.can_restore());
        assert_eq!(buf.restore().as_deref(), Some("ab\ncd"));
    }

    #[test]
    fn restore_after_two_transforms_returns_previous_step() {
        let mut buf = RestoreBuffer::default();
        buf.on_transform("ab");
        let after_upper = apply("ab", &[Operation::Upper], &mut rng());
        assert_eq!(after_upper, "AB");
        buf.on_transform(&after_upper);
        assert_eq!(apply(&after_upper, &[Operation::Lower], &mut rng()), "ab");
        assert_eq!(buf.restore().as_deref(), Some("AB"));
    }

    #[test]
    fn user_edit_after_transform_replaces_restore_target() {
        let mut buf = RestoreBuffer::default();
        buf.on_transform("ab");
        assert_eq!(apply("ab", &[Operation::Upper], &mut rng()), "AB");
        buf.on_user_edit("zz");
        assert!(!buf.can_restore());
        assert_eq!(buf.restore().as_deref(), Some("zz"));
    }
}
