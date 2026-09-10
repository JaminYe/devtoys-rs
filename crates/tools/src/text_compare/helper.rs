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

/// Character-level fragment inside a display line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSpan {
    pub tag: DiffTag,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffHunk {
    pub tag: DiffTag,
    pub text: String,
    pub spans: Vec<DiffSpan>,
}

/// One display line on a single side of a side-by-side row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub tag: DiffTag,
    pub text: String,
    pub spans: Vec<DiffSpan>,
}

/// Paired side-by-side display row. `None` is a placeholder so common lines stay aligned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRow {
    pub left: Option<DiffLine>,
    pub right: Option<DiffLine>,
}

pub fn diff_lines(left: &str, right: &str) -> Vec<DiffHunk> {
    let mut hunks: Vec<DiffHunk> = TextDiff::from_lines(left, right)
        .iter_all_changes()
        .map(|change| {
            let tag = match change.tag() {
                ChangeTag::Equal => DiffTag::Equal,
                ChangeTag::Delete => DiffTag::Delete,
                ChangeTag::Insert => DiffTag::Insert,
            };
            DiffHunk {
                tag,
                text: change.value().to_string(),
                spans: Vec::new(),
            }
        })
        .collect();
    attach_inline_spans(&mut hunks);
    hunks
}

/// Build horizontally aligned display rows from line-level diff hunks.
/// Consecutive deletes/inserts are zipped (replace); leftover lines get placeholders.
pub fn diff_rows(left: &str, right: &str) -> Vec<DiffRow> {
    align_hunks(&diff_lines(left, right))
}

fn display_text(raw: &str) -> String {
    raw.trim_end_matches(['\n', '\r']).to_string()
}

fn whole_spans(tag: DiffTag, text: &str) -> Vec<DiffSpan> {
    vec![DiffSpan {
        tag,
        text: text.to_string(),
    }]
}

fn coalesce_spans(spans: Vec<DiffSpan>) -> Vec<DiffSpan> {
    let mut out: Vec<DiffSpan> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        if let Some(last) = out.last_mut() {
            if last.tag == span.tag {
                last.text.push_str(&span.text);
                continue;
            }
        }
        out.push(span);
    }
    out
}

fn spans_or_whole(spans: Vec<DiffSpan>, tag: DiffTag, text: &str) -> Vec<DiffSpan> {
    let spans = coalesce_spans(spans);
    if spans.is_empty() {
        whole_spans(tag, text)
    } else {
        spans
    }
}

/// Character-level inline diff on a zipped replace pair (similar `from_chars`).
fn pair_inline_spans(old: &str, new: &str) -> (Vec<DiffSpan>, Vec<DiffSpan>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for change in TextDiff::from_chars(old, new).iter_all_changes() {
        let text = change.value().to_string();
        match change.tag() {
            ChangeTag::Equal => {
                left.push(DiffSpan {
                    tag: DiffTag::Equal,
                    text: text.clone(),
                });
                right.push(DiffSpan {
                    tag: DiffTag::Equal,
                    text,
                });
            }
            ChangeTag::Delete => left.push(DiffSpan {
                tag: DiffTag::Delete,
                text,
            }),
            ChangeTag::Insert => right.push(DiffSpan {
                tag: DiffTag::Insert,
                text,
            }),
        }
    }
    (
        spans_or_whole(left, DiffTag::Delete, old),
        spans_or_whole(right, DiffTag::Insert, new),
    )
}

fn line_from_hunk(hunk: &DiffHunk) -> DiffLine {
    DiffLine {
        tag: hunk.tag,
        text: display_text(&hunk.text),
        spans: hunk.spans.clone(),
    }
}

/// Intra-line spans on zipped replace pairs; unpaired add/delete stay whole-line.
fn attach_inline_spans(hunks: &mut [DiffHunk]) {
    let mut i = 0;
    while i < hunks.len() {
        if hunks[i].tag == DiffTag::Equal {
            let text = display_text(&hunks[i].text);
            hunks[i].spans = whole_spans(DiffTag::Equal, &text);
            i += 1;
            continue;
        }
        let start = i;
        while i < hunks.len() && hunks[i].tag != DiffTag::Equal {
            i += 1;
        }
        let deletes: Vec<usize> = (start..i)
            .filter(|&j| hunks[j].tag == DiffTag::Delete)
            .collect();
        let inserts: Vec<usize> = (start..i)
            .filter(|&j| hunks[j].tag == DiffTag::Insert)
            .collect();
        let paired = deletes.len().min(inserts.len());
        let mut updates = Vec::with_capacity(paired * 2);
        for k in 0..paired {
            let old = display_text(&hunks[deletes[k]].text);
            let new = display_text(&hunks[inserts[k]].text);
            let (left, right) = pair_inline_spans(&old, &new);
            updates.push((deletes[k], left));
            updates.push((inserts[k], right));
        }
        for (idx, spans) in updates {
            hunks[idx].spans = spans;
        }
        for j in start..i {
            if hunks[j].spans.is_empty() {
                let text = display_text(&hunks[j].text);
                let tag = hunks[j].tag;
                hunks[j].spans = whole_spans(tag, &text);
            }
        }
    }
}

fn align_hunks(hunks: &[DiffHunk]) -> Vec<DiffRow> {
    let mut rows = Vec::new();
    let mut i = 0;
    while i < hunks.len() {
        match hunks[i].tag {
            DiffTag::Equal => {
                let line = line_from_hunk(&hunks[i]);
                rows.push(DiffRow {
                    left: Some(line.clone()),
                    right: Some(line),
                });
                i += 1;
            }
            DiffTag::Delete | DiffTag::Insert => {
                let mut deletes = Vec::new();
                let mut inserts = Vec::new();
                while i < hunks.len() {
                    match hunks[i].tag {
                        DiffTag::Delete => deletes.push(line_from_hunk(&hunks[i])),
                        DiffTag::Insert => inserts.push(line_from_hunk(&hunks[i])),
                        DiffTag::Equal => break,
                    }
                    i += 1;
                }
                let n = deletes.len().max(inserts.len());
                for j in 0..n {
                    rows.push(DiffRow {
                        left: deletes.get(j).cloned(),
                        right: inserts.get(j).cloned(),
                    });
                }
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side_text(cell: &Option<DiffLine>) -> &str {
        cell.as_ref().map(|line| line.text.as_str()).unwrap_or("")
    }

    fn row_texts(rows: &[DiffRow]) -> (Vec<String>, Vec<String>) {
        (
            rows.iter()
                .map(|row| side_text(&row.left).to_string())
                .collect(),
            rows.iter()
                .map(|row| side_text(&row.right).to_string())
                .collect(),
        )
    }

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

    #[test]
    fn side_by_side_middle_insert_keeps_common_b_aligned() {
        let rows = diff_rows("a\nb", "a\nx\nb");
        let (left, right) = row_texts(&rows);
        assert_eq!(left, ["a", "", "b"]);
        assert_eq!(right, ["a", "x", "b"]);
        assert_eq!(rows[0].left.as_ref().unwrap().tag, DiffTag::Equal);
        assert!(rows[1].left.is_none());
        assert_eq!(rows[1].right.as_ref().unwrap().tag, DiffTag::Insert);
        assert_eq!(rows[2].left.as_ref().unwrap().tag, DiffTag::Equal);
    }

    #[test]
    fn single_line_insert() {
        let (left, right) = row_texts(&diff_rows("a\nb", "a\nx\nb"));
        assert_eq!(left, ["a", "", "b"]);
        assert_eq!(right, ["a", "x", "b"]);
    }

    #[test]
    fn single_line_delete() {
        let rows = diff_rows("a\nx\nb", "a\nb");
        let (left, right) = row_texts(&rows);
        assert_eq!(left, ["a", "x", "b"]);
        assert_eq!(right, ["a", "", "b"]);
        assert_eq!(rows[1].left.as_ref().unwrap().tag, DiffTag::Delete);
        assert!(rows[1].right.is_none());
    }

    #[test]
    fn single_line_replace_pairs_on_same_row() {
        let rows = diff_rows("a\nb", "a\nc");
        let (left, right) = row_texts(&rows);
        assert_eq!(left, ["a", "b"]);
        assert_eq!(right, ["a", "c"]);
        assert_eq!(rows[1].left.as_ref().unwrap().tag, DiffTag::Delete);
        assert_eq!(rows[1].right.as_ref().unwrap().tag, DiffTag::Insert);
    }

    #[test]
    fn multi_line_insert() {
        let (left, right) = row_texts(&diff_rows("a\nb", "a\nx\ny\nb"));
        assert_eq!(left, ["a", "", "", "b"]);
        assert_eq!(right, ["a", "x", "y", "b"]);
    }

    #[test]
    fn multi_line_delete() {
        let (left, right) = row_texts(&diff_rows("a\nx\ny\nb", "a\nb"));
        assert_eq!(left, ["a", "x", "y", "b"]);
        assert_eq!(right, ["a", "", "", "b"]);
    }

    #[test]
    fn multi_line_replace_zips_then_placeholder() {
        let rows = diff_rows("a\nb\nc", "a\nx\ny\nz");
        let (left, right) = row_texts(&rows);
        assert_eq!(left, ["a", "b", "c", ""]);
        assert_eq!(right, ["a", "x", "y", "z"]);
        assert!(rows[3].left.is_none());
        assert_eq!(rows[3].right.as_ref().unwrap().tag, DiffTag::Insert);
    }

    #[test]
    fn alternating_changes_keep_common_lines_aligned() {
        let rows = diff_rows("a\nb\nc\nd", "a\nx\nc\ny");
        let (left, right) = row_texts(&rows);
        assert_eq!(left, ["a", "b", "c", "d"]);
        assert_eq!(right, ["a", "x", "c", "y"]);
        assert_eq!(rows[2].left.as_ref().unwrap().tag, DiffTag::Equal);
        assert_eq!(side_text(&rows[2].left), "c");
        assert_eq!(side_text(&rows[2].right), "c");
    }

    #[test]
    fn empty_files_yield_no_rows() {
        assert!(diff_rows("", "").is_empty());
    }

    #[test]
    fn empty_left_inserts_all_right_lines() {
        let (left, right) = row_texts(&diff_rows("", "a\nb"));
        assert_eq!(left, ["", ""]);
        assert_eq!(right, ["a", "b"]);
    }

    #[test]
    fn empty_right_deletes_all_left_lines() {
        let (left, right) = row_texts(&diff_rows("a\nb", ""));
        assert_eq!(left, ["a", "b"]);
        assert_eq!(right, ["", ""]);
    }

    #[test]
    fn trailing_add() {
        let (left, right) = row_texts(&diff_rows("a", "a\nb"));
        assert_eq!(left, ["a", ""]);
        assert_eq!(right, ["a", "b"]);
    }

    #[test]
    fn trailing_delete() {
        let (left, right) = row_texts(&diff_rows("a\nb", "a"));
        assert_eq!(left, ["a", "b"]);
        assert_eq!(right, ["a", ""]);
    }

    #[test]
    fn preserves_blank_content_line_distinct_from_placeholder() {
        let rows = diff_rows("a\n\nb", "a\nb");
        let (left, right) = row_texts(&rows);
        assert_eq!(left, ["a", "", "b"]);
        assert_eq!(right, ["a", "", "b"]);
        assert_eq!(rows[1].left.as_ref().unwrap().tag, DiffTag::Delete);
        assert_eq!(rows[1].left.as_ref().unwrap().text, "");
        assert!(rows[1].right.is_none());
    }

    fn span_pairs(spans: &[DiffSpan]) -> Vec<(DiffTag, &str)> {
        spans.iter().map(|s| (s.tag, s.text.as_str())).collect()
    }

    fn concat_spans(spans: &[DiffSpan]) -> String {
        spans.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn hello_world_vs_hello_there_splits_changed_words() {
        let left = "hello world";
        let right = "hello there";
        let rows = diff_rows(left, right);
        assert_eq!(rows.len(), 1);
        let left_line = rows[0].left.as_ref().unwrap();
        let right_line = rows[0].right.as_ref().unwrap();
        assert_eq!(
            span_pairs(&left_line.spans),
            [
                (DiffTag::Equal, "hello "),
                (DiffTag::Delete, "wo"),
                (DiffTag::Equal, "r"),
                (DiffTag::Delete, "ld"),
            ]
        );
        assert_eq!(
            span_pairs(&right_line.spans),
            [
                (DiffTag::Equal, "hello "),
                (DiffTag::Insert, "the"),
                (DiffTag::Equal, "r"),
                (DiffTag::Insert, "e"),
            ]
        );
        assert_eq!(concat_spans(&right_line.spans), right_line.text);

        let hunks = diff_lines(left, right);
        let del = hunks.iter().find(|h| h.tag == DiffTag::Delete).unwrap();
        let ins = hunks.iter().find(|h| h.tag == DiffTag::Insert).unwrap();
        assert_eq!(span_pairs(&del.spans), span_pairs(&left_line.spans));
        assert_eq!(span_pairs(&ins.spans), span_pairs(&right_line.spans));
    }

    #[test]
    fn side_by_side_and_inline_both_produce_intra_line_spans() {
        let left = "keep\nhello world\nkeep";
        let right = "keep\nhello there\nkeep";
        let rows = diff_rows(left, right);
        assert_eq!(rows.len(), 3);
        assert_eq!(
            span_pairs(&rows[1].left.as_ref().unwrap().spans),
            [
                (DiffTag::Equal, "hello "),
                (DiffTag::Delete, "wo"),
                (DiffTag::Equal, "r"),
                (DiffTag::Delete, "ld"),
            ]
        );
        assert_eq!(
            span_pairs(&rows[1].right.as_ref().unwrap().spans),
            [
                (DiffTag::Equal, "hello "),
                (DiffTag::Insert, "the"),
                (DiffTag::Equal, "r"),
                (DiffTag::Insert, "e"),
            ]
        );
        let hunks = diff_lines(left, right);
        let del = hunks.iter().find(|h| h.tag == DiffTag::Delete).unwrap();
        let ins = hunks.iter().find(|h| h.tag == DiffTag::Insert).unwrap();
        assert!(
            del.spans.iter().any(|s| s.tag == DiffTag::Equal)
                && del.spans.iter().any(|s| s.tag == DiffTag::Delete),
            "inline delete hunk should mix equal + delete spans, got {:?}",
            del.spans
        );
        assert!(
            ins.spans.iter().any(|s| s.tag == DiffTag::Equal)
                && ins.spans.iter().any(|s| s.tag == DiffTag::Insert),
            "inline insert hunk should mix equal + insert spans, got {:?}",
            ins.spans
        );
    }

    #[test]
    fn whole_line_insert_and_delete_keep_single_line_tag() {
        let insert_rows = diff_rows("a\nb", "a\nx\nb");
        assert!(insert_rows[1].left.is_none());
        let inserted = insert_rows[1].right.as_ref().unwrap();
        assert_eq!(inserted.tag, DiffTag::Insert);
        assert_eq!(span_pairs(&inserted.spans), [(DiffTag::Insert, "x")]);

        let delete_rows = diff_rows("a\nx\nb", "a\nb");
        assert!(delete_rows[1].right.is_none());
        let deleted = delete_rows[1].left.as_ref().unwrap();
        assert_eq!(deleted.tag, DiffTag::Delete);
        assert_eq!(span_pairs(&deleted.spans), [(DiffTag::Delete, "x")]);

        let hunks = diff_lines("a\nb", "a\nx\nb");
        let ins = hunks.iter().find(|h| h.tag == DiffTag::Insert).unwrap();
        assert_eq!(ins.spans.len(), 1);
        assert_eq!(ins.spans[0].tag, DiffTag::Insert);
        assert_eq!(ins.spans[0].text, "x");
    }

    #[test]
    fn empty_and_identical_have_no_false_highlights() {
        assert!(diff_rows("", "").is_empty());
        assert!(diff_lines("", "").is_empty());

        let rows = diff_rows("hello world", "hello world");
        assert_eq!(rows.len(), 1);
        let line = rows[0].left.as_ref().unwrap();
        assert_eq!(line.tag, DiffTag::Equal);
        assert!(
            line.spans.iter().all(|s| s.tag == DiffTag::Equal),
            "identical text must not highlight, got {:?}",
            line.spans
        );
        assert!(!line.spans.iter().any(|s| s.tag != DiffTag::Equal));

        let hunks = diff_lines("a\nb", "a\nb");
        assert!(hunks.iter().all(|h| h.tag == DiffTag::Equal));
        assert!(hunks
            .iter()
            .all(|h| h.spans.iter().all(|s| s.tag == DiffTag::Equal)));
        assert!(hunks.iter().all(|h| !h
            .spans
            .iter()
            .any(|s| matches!(s.tag, DiffTag::Delete | DiffTag::Insert))));
    }

    #[test]
    fn character_level_diff_abc_vs_axc() {
        let left = "abc";
        let right = "axc";

        // Side-by-side (diff_rows)
        let rows = diff_rows(left, right);
        assert_eq!(rows.len(), 1);
        let left_line = rows[0].left.as_ref().unwrap();
        let right_line = rows[0].right.as_ref().unwrap();
        assert_eq!(
            span_pairs(&left_line.spans),
            [
                (DiffTag::Equal, "a"),
                (DiffTag::Delete, "b"),
                (DiffTag::Equal, "c"),
            ]
        );
        assert_eq!(
            span_pairs(&right_line.spans),
            [
                (DiffTag::Equal, "a"),
                (DiffTag::Insert, "x"),
                (DiffTag::Equal, "c"),
            ]
        );
        assert_eq!(concat_spans(&left_line.spans), "abc");
        assert_eq!(concat_spans(&right_line.spans), "axc");

        // Inline (diff_lines)
        let hunks = diff_lines(left, right);
        let del = hunks.iter().find(|h| h.tag == DiffTag::Delete).unwrap();
        let ins = hunks.iter().find(|h| h.tag == DiffTag::Insert).unwrap();
        assert_eq!(span_pairs(&del.spans), span_pairs(&left_line.spans));
        assert_eq!(span_pairs(&ins.spans), span_pairs(&right_line.spans));
    }

    #[test]
    fn chinese_unicode_character_level_diff() {
        let left = "测试文本";
        let right = "测试样例";
        let rows = diff_rows(left, right);
        assert_eq!(rows.len(), 1);
        let left_line = rows[0].left.as_ref().unwrap();
        let right_line = rows[0].right.as_ref().unwrap();
        assert_eq!(
            span_pairs(&left_line.spans),
            [(DiffTag::Equal, "测试"), (DiffTag::Delete, "文本"),]
        );
        assert_eq!(
            span_pairs(&right_line.spans),
            [(DiffTag::Equal, "测试"), (DiffTag::Insert, "样例"),]
        );
        assert_eq!(concat_spans(&left_line.spans), "测试文本");
        assert_eq!(concat_spans(&right_line.spans), "测试样例");

        let left_mid = "这是一个完整测试";
        let right_mid = "这是一个重写测试";
        let rows_mid = diff_rows(left_mid, right_mid);
        let l_mid = rows_mid[0].left.as_ref().unwrap();
        let r_mid = rows_mid[0].right.as_ref().unwrap();
        assert_eq!(
            span_pairs(&l_mid.spans),
            [
                (DiffTag::Equal, "这是一个"),
                (DiffTag::Delete, "完整"),
                (DiffTag::Equal, "测试"),
            ]
        );
        assert_eq!(
            span_pairs(&r_mid.spans),
            [
                (DiffTag::Equal, "这是一个"),
                (DiffTag::Insert, "重写"),
                (DiffTag::Equal, "测试"),
            ]
        );
    }

    #[test]
    fn intra_word_insertion_and_deletion() {
        // Deletion inside word: "world" -> "word"
        let rows_del = diff_rows("world", "word");
        assert_eq!(rows_del.len(), 1);
        let l_del = rows_del[0].left.as_ref().unwrap();
        let r_del = rows_del[0].right.as_ref().unwrap();
        assert_eq!(
            span_pairs(&l_del.spans),
            [
                (DiffTag::Equal, "wor"),
                (DiffTag::Delete, "l"),
                (DiffTag::Equal, "d"),
            ]
        );
        assert_eq!(span_pairs(&r_del.spans), [(DiffTag::Equal, "word")]);

        // Insertion inside word: "word" -> "world"
        let rows_ins = diff_rows("word", "world");
        assert_eq!(rows_ins.len(), 1);
        let l_ins = rows_ins[0].left.as_ref().unwrap();
        let r_ins = rows_ins[0].right.as_ref().unwrap();
        assert_eq!(span_pairs(&l_ins.spans), [(DiffTag::Equal, "word")]);
        assert_eq!(
            span_pairs(&r_ins.spans),
            [
                (DiffTag::Equal, "wor"),
                (DiffTag::Insert, "l"),
                (DiffTag::Equal, "d"),
            ]
        );

        // Multi-char replacement inside word: "abcdef" -> "abxyzdef"
        let rows_sub = diff_rows("abcdef", "abxyzdef");
        assert_eq!(rows_sub.len(), 1);
        let l_sub = rows_sub[0].left.as_ref().unwrap();
        let r_sub = rows_sub[0].right.as_ref().unwrap();
        assert_eq!(
            span_pairs(&l_sub.spans),
            [
                (DiffTag::Equal, "ab"),
                (DiffTag::Delete, "c"),
                (DiffTag::Equal, "def"),
            ]
        );
        assert_eq!(
            span_pairs(&r_sub.spans),
            [
                (DiffTag::Equal, "ab"),
                (DiffTag::Insert, "xyz"),
                (DiffTag::Equal, "def"),
            ]
        );
    }

    #[test]
    fn character_diff_preserves_line_alignment_and_placeholders() {
        let left = "a\nabc\nb";
        let right = "a\naxc\nx\nb";
        let rows = diff_rows(left, right);
        assert_eq!(rows.len(), 4);

        // Row 0: Equal "a"
        assert_eq!(rows[0].left.as_ref().unwrap().tag, DiffTag::Equal);
        assert_eq!(rows[0].left.as_ref().unwrap().text, "a");
        assert_eq!(rows[0].right.as_ref().unwrap().tag, DiffTag::Equal);
        assert_eq!(rows[0].right.as_ref().unwrap().text, "a");

        // Row 1: Paired inline diff "abc" vs "axc"
        let r1_l = rows[1].left.as_ref().unwrap();
        let r1_r = rows[1].right.as_ref().unwrap();
        assert_eq!(r1_l.tag, DiffTag::Delete);
        assert_eq!(r1_r.tag, DiffTag::Insert);
        assert_eq!(
            span_pairs(&r1_l.spans),
            [
                (DiffTag::Equal, "a"),
                (DiffTag::Delete, "b"),
                (DiffTag::Equal, "c"),
            ]
        );
        assert_eq!(
            span_pairs(&r1_r.spans),
            [
                (DiffTag::Equal, "a"),
                (DiffTag::Insert, "x"),
                (DiffTag::Equal, "c"),
            ]
        );

        // Row 2: Placeholder on left, Insert on right
        assert!(rows[2].left.is_none());
        let r2_r = rows[2].right.as_ref().unwrap();
        assert_eq!(r2_r.tag, DiffTag::Insert);
        assert_eq!(r2_r.text, "x");
        assert_eq!(span_pairs(&r2_r.spans), [(DiffTag::Insert, "x")]);

        // Row 3: Equal "b"
        assert_eq!(rows[3].left.as_ref().unwrap().tag, DiffTag::Equal);
        assert_eq!(rows[3].left.as_ref().unwrap().text, "b");
        assert_eq!(rows[3].right.as_ref().unwrap().tag, DiffTag::Equal);
        assert_eq!(rows[3].right.as_ref().unwrap().text, "b");
    }
}
