use similar::{ChangeTag, TextDiff};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffMode {
    #[default]
    SideBySide,
    Inline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffTag {
    Equal,
    Delete,
    Insert,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffHunk {
    pub tag: DiffTag,
    pub text: String,
}

pub fn diff_lines(left: &str, right: &str) -> Vec<DiffHunk> {
    let diff = TextDiff::from_lines(left, right);
    diff.iter_all_changes()
        .map(|change| {
            let tag = match change.tag() {
                ChangeTag::Equal => DiffTag::Equal,
                ChangeTag::Delete => DiffTag::Delete,
                ChangeTag::Insert => DiffTag::Insert,
            };
            DiffHunk {
                tag,
                text: change.value().to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_a_b_right_a_c_has_delete_b_insert_c() {
        let hunks = diff_lines("a\nb", "a\nc");
        assert!(
            hunks
                .iter()
                .any(|h| h.tag == DiffTag::Delete && h.text.trim() == "b"),
            "expected Delete b, got {hunks:?}"
        );
        assert!(
            hunks
                .iter()
                .any(|h| h.tag == DiffTag::Insert && h.text.trim() == "c"),
            "expected Insert c, got {hunks:?}"
        );
        assert!(hunks
            .iter()
            .any(|h| h.tag == DiffTag::Equal && h.text.trim() == "a"));
    }
}
