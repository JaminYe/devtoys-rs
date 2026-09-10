mod formatter;
mod spec;
mod token;
mod tokenizer;

use std::sync::OnceLock;

use crate::indent::Indentation;

use super::helper::SqlLanguage;
use formatter::format as format_tokens;
use spec::dialect;
use tokenizer::Tokenizer;

static TOKENIZERS: [OnceLock<Tokenizer>; 10] = [const { OnceLock::new() }; 10];

pub(super) fn format(query: &str, indent: Indentation, language: SqlLanguage) -> String {
    let spec = dialect(language);
    let tokenizer = TOKENIZERS[language as usize].get_or_init(|| Tokenizer::new(spec));
    let tokens = tokenizer.tokenize(query);
    format_tokens(query, indent, &tokens, spec.override_kind)
}
