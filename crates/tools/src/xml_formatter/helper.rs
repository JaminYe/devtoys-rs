use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;

use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_TEXT, TYPE_XML};

pub use crate::indent::Indentation;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum XmlFormatError {
    #[error("非法 XML")]
    InvalidXml,
}

pub fn format_xml(
    input: &str,
    indent: Indentation,
    new_line_on_attributes: bool,
) -> Result<String, XmlFormatError> {
    let events = parse_xml_events(input)?;
    render(&events, indent, new_line_on_attributes)
}

pub fn looks_like_xml(input: &str) -> bool {
    input.trim().starts_with('<') && parse_xml_events(input).is_ok()
}

fn parse_xml_events(input: &str) -> Result<Vec<Event<'static>>, XmlFormatError> {
    let mut reader = Reader::from_str(input);
    let mut events = Vec::new();
    let mut depth = 0i32;
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
                depth += 1;
                events.push(Event::Start(e.into_owned()));
            }
            Ok(Event::End(e)) => {
                depth -= 1;
                if depth < 0 {
                    return Err(XmlFormatError::InvalidXml);
                }
                events.push(Event::End(e.into_owned()));
            }
            Ok(event) => events.push(event.into_owned()),
            Err(_) => return Err(XmlFormatError::InvalidXml),
        }
    }
    if depth != 0 {
        return Err(XmlFormatError::InvalidXml);
    }
    if !events
        .iter()
        .any(|event| matches!(event, Event::Start(_) | Event::Empty(_)))
    {
        return Err(XmlFormatError::InvalidXml);
    }
    Ok(events)
}

fn render(
    events: &[Event<'static>],
    indent: Indentation,
    new_line_on_attributes: bool,
) -> Result<String, XmlFormatError> {
    let mut buf = Vec::new();
    let split = indent.as_bytes();
    let mut writer = match split {
        Some(bytes) if !bytes.is_empty() => {
            Writer::new_with_indent(&mut buf, bytes[0], bytes.len())
        }
        _ => Writer::new(&mut buf),
    };

    let pretty = split.is_some();
    let rewrite_attrs = pretty && new_line_on_attributes;
    let unit = split
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .unwrap_or("");
    let mut depth = 0usize;

    for event in events {
        match event {
            Event::Text(text) if is_whitespace_only(text.as_ref()) => continue,
            Event::Start(start) => {
                let start = maybe_rewrite_attrs(start, rewrite_attrs, unit, depth)?;
                writer
                    .write_event(Event::Start(start))
                    .map_err(|_| XmlFormatError::InvalidXml)?;
                depth += 1;
            }
            Event::Empty(start) => {
                let start = maybe_rewrite_attrs(start, rewrite_attrs, unit, depth)?;
                writer
                    .write_event(Event::Empty(start))
                    .map_err(|_| XmlFormatError::InvalidXml)?;
            }
            Event::End(end) => {
                depth = depth.saturating_sub(1);
                writer
                    .write_event(Event::End(end.borrow()))
                    .map_err(|_| XmlFormatError::InvalidXml)?;
            }
            other => {
                writer
                    .write_event(other.borrow())
                    .map_err(|_| XmlFormatError::InvalidXml)?;
            }
        }
    }

    String::from_utf8(buf).map_err(|_| XmlFormatError::InvalidXml)
}

fn is_whitespace_only(bytes: &[u8]) -> bool {
    bytes.iter().all(|b| b.is_ascii_whitespace())
}

fn maybe_rewrite_attrs(
    start: &BytesStart<'_>,
    rewrite: bool,
    unit: &str,
    depth: usize,
) -> Result<BytesStart<'static>, XmlFormatError> {
    if !rewrite {
        return Ok(start.to_owned());
    }
    let qname = start.name();
    let name = std::str::from_utf8(qname.as_ref()).map_err(|_| XmlFormatError::InvalidXml)?;
    let mut content = String::from(name);
    let mut has_attr = false;
    let attr_indent = unit.repeat(depth + 1);
    for attr in start.attributes() {
        let attr = attr.map_err(|_| XmlFormatError::InvalidXml)?;
        has_attr = true;
        let key = std::str::from_utf8(attr.key.as_ref()).map_err(|_| XmlFormatError::InvalidXml)?;
        let value = std::str::from_utf8(&attr.value).map_err(|_| XmlFormatError::InvalidXml)?;
        content.push('\n');
        content.push_str(&attr_indent);
        content.push_str(key);
        content.push_str("=\"");
        content.push_str(value);
        content.push('"');
    }
    if !has_attr {
        return Ok(start.to_owned());
    }
    Ok(BytesStart::from_content(content, name.len()))
}

#[derive(Debug, Default, Clone, Copy)]
pub struct XmlDetector;

impl Detector for XmlDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_XML,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        if !looks_like_xml(&parent.value) {
            return None;
        }
        Some(DetectedPayload {
            type_name: TYPE_XML.to_string(),
            value: parent.value.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_spaces_pretty_literal() {
        let got = format_xml("<root><a>1</a></root>", Indentation::TwoSpaces, false).unwrap();
        assert_eq!(got, "<root>\n  <a>1</a>\n</root>");
    }

    #[test]
    fn minified() {
        let got = format_xml("<root>\n  <a>1</a>\n</root>", Indentation::Minified, false).unwrap();
        assert_eq!(got, "<root><a>1</a></root>");
    }

    #[test]
    fn invalid_xml_is_err_without_half_document() {
        let err = format_xml("<", Indentation::TwoSpaces, false).unwrap_err();
        assert_eq!(err, XmlFormatError::InvalidXml);
        assert_eq!(err.to_string(), "非法 XML");
    }

    #[test]
    fn attributes_on_new_lines() {
        let got = format_xml(r#"<root a="1" b="2"/>"#, Indentation::TwoSpaces, true).unwrap();
        assert_eq!(got, "<root\n  a=\"1\"\n  b=\"2\"/>");
    }

    #[test]
    fn detector_accepts_xml_parent_text() {
        let detector = XmlDetector;
        let parent = DetectedPayload {
            type_name: TYPE_TEXT.to_string(),
            value: "<root><a>1</a></root>".to_string(),
        };
        let hit = detector
            .detect(&RawData::text("<root><a>1</a></root>"), Some(&parent))
            .expect("xml parent text should detect");
        assert_eq!(hit.type_name, TYPE_XML);
    }

    #[test]
    fn detector_rejects_plain_text_and_junk() {
        let detector = XmlDetector;
        let text = DetectedPayload {
            type_name: TYPE_TEXT.to_string(),
            value: "hello".to_string(),
        };
        assert!(detector
            .detect(&RawData::text("hello"), Some(&text))
            .is_none());
        let junk = DetectedPayload {
            type_name: TYPE_TEXT.to_string(),
            value: "<".to_string(),
        };
        assert!(detector.detect(&RawData::text("<"), Some(&junk)).is_none());
    }
}
