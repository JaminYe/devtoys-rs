use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_TEXT};
use pulldown_cmark::{html, Options, Parser};

pub const TYPE_MARKDOWN: &str = "Markdown";

pub fn markdown_to_html(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

pub fn looks_like_markdown(text: &str) -> bool {
    text.lines().any(|line| {
        let t = line.trim_start();
        t.starts_with("# ") || t.starts_with("```") || t.starts_with("- ")
    })
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownDetector;

impl Detector for MarkdownDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_MARKDOWN,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        let value = parent.value.as_str();
        if !looks_like_markdown(value) {
            return None;
        }
        Some(DetectedPayload {
            type_name: TYPE_MARKDOWN.to_string(),
            value: value.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent(text: &str) -> DetectedPayload {
        DetectedPayload {
            type_name: TYPE_TEXT.to_string(),
            value: text.to_string(),
        }
    }

    #[test]
    fn heading_html_contains_h1() {
        let html = markdown_to_html("# Hi");
        assert!(html.contains("<h1"), "html was {html}");
    }

    #[test]
    fn detector_accepts_title_rejects_json() {
        assert!(MarkdownDetector
            .detect(&RawData::text("# Title"), Some(&parent("# Title")))
            .is_some());
        assert!(MarkdownDetector
            .detect(&RawData::text(r#"{"a":1}"#), Some(&parent(r#"{"a":1}"#)))
            .is_none());
    }
}
