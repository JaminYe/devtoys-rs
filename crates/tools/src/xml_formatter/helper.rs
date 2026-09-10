use quick_xml::escape::escape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

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
    let nodes = parse_tree(input)?;
    render(&nodes, indent, new_line_on_attributes)
}

pub fn looks_like_xml(input: &str) -> bool {
    input.trim().starts_with('<') && parse_tree(input).is_ok()
}

struct Element {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
    self_closing: bool,
}

enum Node {
    Element(Element),
    Text(String),
    CData(String),
    Comment(String),
    Decl(String),
    PI(String),
    DocType(String),
}

fn parse_tree(input: &str) -> Result<Vec<Node>, XmlFormatError> {
    let mut reader = Reader::from_str(input);
    let mut stack = Vec::new();
    let mut roots = Vec::new();
    let mut root_elements = 0usize;

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Start(start)) => stack.push(element_from_start(&start, false)?),
            Ok(Event::Empty(start)) => {
                push_node(
                    &mut stack,
                    &mut roots,
                    &mut root_elements,
                    Node::Element(element_from_start(&start, true)?),
                )?;
            }
            Ok(Event::End(end)) => {
                let element = stack.pop().ok_or(XmlFormatError::InvalidXml)?;
                let name = utf8(end.name().as_ref())?.to_string();
                if name != element.name {
                    return Err(XmlFormatError::InvalidXml);
                }
                push_node(
                    &mut stack,
                    &mut roots,
                    &mut root_elements,
                    Node::Element(element),
                )?;
            }
            Ok(Event::Text(text)) => {
                let value = text
                    .unescape()
                    .map_err(|_| XmlFormatError::InvalidXml)?
                    .into_owned();
                if stack.is_empty() && !is_whitespace_only(value.as_bytes()) {
                    return Err(XmlFormatError::InvalidXml);
                }
                push_child(&mut stack, &mut roots, Node::Text(value));
            }
            Ok(Event::CData(cdata)) => {
                if stack.is_empty() {
                    return Err(XmlFormatError::InvalidXml);
                }
                push_child(
                    &mut stack,
                    &mut roots,
                    Node::CData(utf8(cdata.as_ref())?.to_string()),
                );
            }
            Ok(Event::Comment(comment)) => {
                push_child(
                    &mut stack,
                    &mut roots,
                    Node::Comment(utf8(comment.as_ref())?.to_string()),
                );
            }
            Ok(Event::Decl(decl)) => {
                push_child(
                    &mut stack,
                    &mut roots,
                    Node::Decl(utf8(decl.as_ref())?.to_string()),
                );
            }
            Ok(Event::PI(pi)) => {
                push_child(
                    &mut stack,
                    &mut roots,
                    Node::PI(utf8(pi.as_ref())?.to_string()),
                );
            }
            Ok(Event::DocType(doctype)) => {
                push_child(
                    &mut stack,
                    &mut roots,
                    Node::DocType(utf8(doctype.as_ref())?.to_string()),
                );
            }
            Err(_) => return Err(XmlFormatError::InvalidXml),
        }
    }

    if !stack.is_empty() || root_elements != 1 {
        return Err(XmlFormatError::InvalidXml);
    }
    Ok(roots)
}

fn element_from_start(
    start: &BytesStart<'_>,
    self_closing: bool,
) -> Result<Element, XmlFormatError> {
    Ok(Element {
        name: utf8(start.name().as_ref())?.to_string(),
        attrs: parse_attrs(start)?,
        children: Vec::new(),
        self_closing,
    })
}

fn parse_attrs(start: &BytesStart<'_>) -> Result<Vec<(String, String)>, XmlFormatError> {
    let mut attrs = Vec::new();
    for attr in start.attributes() {
        let attr = attr.map_err(|_| XmlFormatError::InvalidXml)?;
        let key = utf8(attr.key.as_ref())?.to_string();
        let value = attr
            .unescape_value()
            .map_err(|_| XmlFormatError::InvalidXml)?
            .into_owned();
        attrs.push((key, value));
    }
    Ok(attrs)
}

fn push_node(
    stack: &mut Vec<Element>,
    roots: &mut Vec<Node>,
    root_elements: &mut usize,
    node: Node,
) -> Result<(), XmlFormatError> {
    if stack.is_empty() {
        *root_elements += 1;
        if *root_elements > 1 {
            return Err(XmlFormatError::InvalidXml);
        }
    }
    push_child(stack, roots, node);
    Ok(())
}

fn push_child(stack: &mut Vec<Element>, roots: &mut Vec<Node>, node: Node) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else {
        roots.push(node);
    }
}

fn utf8(bytes: &[u8]) -> Result<&str, XmlFormatError> {
    std::str::from_utf8(bytes).map_err(|_| XmlFormatError::InvalidXml)
}

fn render(
    nodes: &[Node],
    indent: Indentation,
    new_line_on_attributes: bool,
) -> Result<String, XmlFormatError> {
    let pretty = indent.as_bytes().is_some();
    let rewrite_attrs = pretty && new_line_on_attributes;
    let unit = indent
        .as_bytes()
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .unwrap_or("");
    let mut out = String::new();
    let mut first = true;
    for node in nodes {
        if is_ignorable_ws(node) {
            continue;
        }
        if pretty && !first {
            out.push('\n');
        }
        first = false;
        write_node(&mut out, node, pretty, unit, rewrite_attrs, 0, false);
    }
    Ok(out)
}

fn write_node(
    out: &mut String,
    node: &Node,
    pretty: bool,
    unit: &str,
    rewrite_attrs: bool,
    depth: usize,
    preserve: bool,
) {
    match node {
        Node::Element(element) => {
            write_element(out, element, pretty, unit, rewrite_attrs, depth, preserve);
        }
        Node::Text(text) => out.push_str(&escape(text.as_str())),
        Node::CData(text) => {
            out.push_str("<![CDATA[");
            out.push_str(text);
            out.push_str("]]>");
        }
        Node::Comment(text) => {
            out.push_str("<!--");
            out.push_str(text);
            out.push_str("-->");
        }
        Node::Decl(text) => {
            out.push_str("<?");
            out.push_str(text);
            out.push_str("?>");
        }
        Node::PI(text) => {
            out.push_str("<?");
            out.push_str(text);
            out.push_str("?>");
        }
        Node::DocType(text) => {
            out.push_str("<!DOCTYPE ");
            out.push_str(text);
            out.push('>');
        }
    }
}

fn write_element(
    out: &mut String,
    element: &Element,
    pretty: bool,
    unit: &str,
    rewrite_attrs: bool,
    depth: usize,
    inherited_preserve: bool,
) {
    let preserve = xml_space_preserve(&element.attrs).unwrap_or(inherited_preserve);
    let empty = element.self_closing && element.children.is_empty();
    write_open_tag(out, element, rewrite_attrs, unit, depth, empty);
    if empty {
        return;
    }

    let element_only = !preserve && is_element_only(&element.children);
    if element_only {
        let kids = element
            .children
            .iter()
            .filter(|child| !is_ignorable_ws(child));
        if pretty {
            let mut has_child = false;
            for child in kids {
                has_child = true;
                write_indent(out, unit, depth + 1);
                write_node(out, child, pretty, unit, rewrite_attrs, depth + 1, preserve);
            }
            if has_child {
                write_indent(out, unit, depth);
            }
        } else {
            for child in kids {
                write_node(out, child, pretty, unit, rewrite_attrs, depth + 1, preserve);
            }
        }
    } else {
        for child in &element.children {
            write_node(out, child, pretty, unit, rewrite_attrs, depth + 1, preserve);
        }
    }

    out.push_str("</");
    out.push_str(&element.name);
    out.push('>');
}

fn write_open_tag(
    out: &mut String,
    element: &Element,
    rewrite_attrs: bool,
    unit: &str,
    depth: usize,
    empty: bool,
) {
    out.push('<');
    out.push_str(&element.name);
    if rewrite_attrs && !element.attrs.is_empty() {
        let attr_indent = unit.repeat(depth + 1);
        for (key, value) in &element.attrs {
            out.push('\n');
            out.push_str(&attr_indent);
            out.push_str(key);
            out.push_str("=\"");
            out.push_str(&escape_attr(value));
            out.push('"');
        }
    } else {
        for (key, value) in &element.attrs {
            out.push(' ');
            out.push_str(key);
            out.push_str("=\"");
            out.push_str(&escape_attr(value));
            out.push('"');
        }
    }
    if empty {
        out.push_str("/>");
    } else {
        out.push('>');
    }
}

/// Attribute values keep TAB/LF/CR as character references so XML
/// attribute-value normalization does not turn them into spaces.
fn escape_attr(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(c),
        }
    }
    out
}

fn write_indent(out: &mut String, unit: &str, depth: usize) {
    out.push('\n');
    for _ in 0..depth {
        out.push_str(unit);
    }
}

fn xml_space_preserve(attrs: &[(String, String)]) -> Option<bool> {
    attrs
        .iter()
        .rev()
        .find(|(key, _)| key == "xml:space")
        .map(|(_, value)| value == "preserve")
}

fn is_element_only(children: &[Node]) -> bool {
    let mut has_element = false;
    for child in children {
        match child {
            Node::Element(_) => has_element = true,
            Node::Text(text) if is_ignorable_ws_text(text) => {}
            Node::Text(_) | Node::CData(_) => return false,
            Node::Comment(_) | Node::Decl(_) | Node::PI(_) | Node::DocType(_) => {}
        }
    }
    has_element
}

fn is_ignorable_ws(node: &Node) -> bool {
    matches!(node, Node::Text(text) if is_ignorable_ws_text(text))
}

/// Newline-containing whitespace between elements is indent; a same-line space
/// such as `<b>a</b> <i>b</i>` is mixed content and must be kept.
fn is_ignorable_ws_text(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|b| b.is_ascii_whitespace())
        && text.bytes().any(|b| b == b'\n' || b == b'\r')
}

fn is_whitespace_only(bytes: &[u8]) -> bool {
    bytes.iter().all(|b| b.is_ascii_whitespace())
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
        Some(DetectedPayload::new(TYPE_XML, parent.value.clone()))
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
        let parent = DetectedPayload::new(TYPE_TEXT, "<root><a>1</a></root>");
        let hit = detector
            .detect(&RawData::text("<root><a>1</a></root>"), Some(&parent))
            .expect("xml parent text should detect");
        assert_eq!(hit.type_name, TYPE_XML);
    }

    #[test]
    fn detector_rejects_plain_text_and_junk() {
        let detector = XmlDetector;
        let text = DetectedPayload::new(TYPE_TEXT, "hello");
        assert!(detector
            .detect(&RawData::text("hello"), Some(&text))
            .is_none());
        let junk = DetectedPayload::new(TYPE_TEXT, "<");
        assert!(detector.detect(&RawData::text("<"), Some(&junk)).is_none());
    }

    fn attr_value(xml: &str, key: &str) -> String {
        let mut reader = Reader::from_str(xml);
        loop {
            let event = reader.read_event().expect("output must be legal XML");
            let start = match event {
                Event::Start(start) | Event::Empty(start) => start,
                Event::Eof => panic!("attribute {key} not found in {xml}"),
                _ => continue,
            };
            for attr in start.attributes() {
                let attr = attr.expect("attribute must parse");
                if attr.key.as_ref() == key.as_bytes() {
                    return attr
                        .unescape_value()
                        .expect("attribute value must unescape")
                        .into_owned();
                }
            }
        }
    }

    fn character_data(xml: &str) -> String {
        let mut reader = Reader::from_str(xml);
        let mut out = String::new();
        loop {
            match reader.read_event().expect("output must be legal XML") {
                Event::Text(text) => {
                    out.push_str(&text.unescape().expect("text must unescape"));
                }
                Event::CData(cdata) => {
                    out.push_str(std::str::from_utf8(cdata.as_ref()).expect("cdata utf-8"));
                }
                Event::Eof => break,
                _ => {}
            }
        }
        out
    }

    #[test]
    fn attribute_quote_damage_is_escaped_and_round_trips() {
        let got = format_xml(r#"<root a='say "hi"'/>"#, Indentation::TwoSpaces, true).unwrap();
        assert!(
            !got.contains(r#"a="say "hi""#),
            "raw quotes must not break the attribute: {got}"
        );
        assert!(
            got.contains("&quot;") || got.contains(r#"a='say "hi"'"#),
            "expected escaped or equivalently legal quotes: {got}"
        );
        assert_eq!(attr_value(&got, "a"), r#"say "hi""#);
        format_xml(&got, Indentation::Minified, false).expect("output must re-parse");
    }

    #[test]
    fn attribute_entities_round_trip() {
        let got = format_xml(r#"<root a="a&amp;b&lt;c"/>"#, Indentation::TwoSpaces, true).unwrap();
        assert_eq!(attr_value(&got, "a"), "a&b<c");
    }

    #[test]
    fn multi_root_and_illegal_markup_are_invalid() {
        for input in ["<a/><b/>", "<a></a><b></b>", "<a></b>", "<root a=1/>"] {
            let err = format_xml(input, Indentation::TwoSpaces, false).unwrap_err();
            assert_eq!(err, XmlFormatError::InvalidXml, "input: {input}");
        }
    }

    #[test]
    fn detector_rejects_multi_root() {
        assert!(!looks_like_xml("<a/><b/>"));
        let detector = XmlDetector;
        let parent = DetectedPayload::new(TYPE_TEXT, "<a/><b/>");
        assert!(detector
            .detect(&RawData::text("<a/><b/>"), Some(&parent))
            .is_none());
    }

    #[test]
    fn mixed_content_keeps_spaces_around_text() {
        let input = "<p>hello <b>world</b>!</p>";
        let pretty = format_xml(input, Indentation::TwoSpaces, false).unwrap();
        let minified = format_xml(input, Indentation::Minified, false).unwrap();
        assert_eq!(character_data(&pretty), "hello world!");
        assert_eq!(character_data(&minified), "hello world!");
        let mixed = format_xml("<p>a <b>b</b> c</p>", Indentation::Minified, false).unwrap();
        assert_eq!(character_data(&mixed), "a b c");
        let inline_ws =
            format_xml("<p><b>a</b> <i>b</i></p>", Indentation::TwoSpaces, false).unwrap();
        assert!(
            inline_ws.contains("</b> <i>"),
            "same-line space between elements is mixed content: {inline_ws}"
        );
        assert_eq!(character_data(&inline_ws), "a b");
    }

    #[test]
    fn cdata_preserves_text_semantics() {
        let input = "<root><![CDATA[a < b & c]]></root>";
        let pretty = format_xml(input, Indentation::TwoSpaces, false).unwrap();
        let minified = format_xml(input, Indentation::Minified, false).unwrap();
        assert_eq!(character_data(&pretty), "a < b & c");
        assert_eq!(character_data(&minified), "a < b & c");
    }

    #[test]
    fn xml_space_preserve_keeps_explicit_whitespace() {
        let input = "<root xml:space=\"preserve\">\n    <a/>\n</root>";
        let pretty = format_xml(input, Indentation::TwoSpaces, false).unwrap();
        let minified = format_xml(input, Indentation::Minified, false).unwrap();
        assert!(
            pretty.contains("    <a/>"),
            "pretty-print must keep preserved indent: {pretty}"
        );
        assert!(
            minified.contains("    <a/>"),
            "minify must keep preserved indent: {minified}"
        );
        assert_eq!(character_data(&pretty), character_data(input));
        assert_eq!(character_data(&minified), character_data(input));
    }

    fn all_format_options() -> impl Iterator<Item = (Indentation, bool)> {
        [
            Indentation::TwoSpaces,
            Indentation::FourSpaces,
            Indentation::OneTab,
            Indentation::Minified,
        ]
        .into_iter()
        .flat_map(|indent| [false, true].into_iter().map(move |nl| (indent, nl)))
    }

    fn raw_attr_value(xml: &str, key: &str) -> String {
        let mut reader = Reader::from_str(xml);
        loop {
            let event = reader.read_event().expect("output must be legal XML");
            let start = match event {
                Event::Start(start) | Event::Empty(start) => start,
                Event::Eof => panic!("attribute {key} not found in {xml}"),
                _ => continue,
            };
            for attr in start.attributes() {
                let attr = attr.expect("attribute must parse");
                if attr.key.as_ref() == key.as_bytes() {
                    return std::str::from_utf8(&attr.value)
                        .expect("attribute value utf-8")
                        .to_string();
                }
            }
        }
    }

    /// W3C XML 1.0 §3.3.3 Attribute-Value Normalization for CDATA attributes.
    /// Character/entity references keep the referenced character; literal
    /// #x9/#xA/#xD/#x20 become space. Independent of the formatter escape path.
    fn av_normalize_cdata(raw: &str) -> String {
        let mut out = String::new();
        let mut rest = raw;
        while !rest.is_empty() {
            if rest.starts_with('&') {
                let end = rest.find(';').expect("unterminated entity in attribute");
                let body = &rest[1..end];
                out.push(decode_xml_ref(body));
                rest = &rest[end + 1..];
                continue;
            }
            let ch = rest.chars().next().expect("non-empty rest");
            match ch {
                '\t' | '\n' | '\r' | ' ' => out.push(' '),
                _ => out.push(ch),
            }
            rest = &rest[ch.len_utf8()..];
        }
        out
    }

    fn decode_xml_ref(body: &str) -> char {
        if let Some(hex) = body.strip_prefix("#x").or_else(|| body.strip_prefix("#X")) {
            char::from_u32(u32::from_str_radix(hex, 16).expect("hex char ref"))
                .expect("hex char ref codepoint")
        } else if let Some(dec) = body.strip_prefix('#') {
            char::from_u32(dec.parse().expect("dec char ref")).expect("dec char ref codepoint")
        } else {
            match body {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                other => panic!("unknown entity &{other};"),
            }
        }
    }

    fn av_normalized_attr_codes(xml: &str, key: &str) -> Vec<u32> {
        av_normalize_cdata(&raw_attr_value(xml, key))
            .chars()
            .map(|c| c as u32)
            .collect()
    }

    #[test]
    fn attribute_tab_lf_cr_char_refs_survive_avnormalize() {
        let inputs = [
            r#"<root a="&#x9;&#xA;&#xD;"/>"#,
            r#"<root a="&#9;&#10;&#13;"/>"#,
        ];
        let expected = vec![9, 10, 13];
        for input in inputs {
            assert_eq!(
                av_normalized_attr_codes(input, "a"),
                expected,
                "independent reader must see 9,10,13 on original input: {input}"
            );
            for (indent, nl) in all_format_options() {
                let got = format_xml(input, indent, nl).unwrap();
                assert_eq!(
                    av_normalized_attr_codes(&got, "a"),
                    expected,
                    "indent={indent:?} nl={nl} input={input} output={got:?}"
                );
            }
        }
    }

    #[test]
    fn attribute_quote_amp_angle_and_whitespace_char_refs_survive_avnormalize() {
        let input = r#"<root a="&quot;&amp;&lt;&gt;&#x9;&#xA;&#xD;"/>"#;
        let expected: Vec<u32> = [
            b'"' as u32,
            b'&' as u32,
            b'<' as u32,
            b'>' as u32,
            9,
            10,
            13,
        ]
        .into_iter()
        .collect();
        assert_eq!(
            av_normalized_attr_codes(input, "a"),
            expected,
            "independent reader baseline for quote/amp/angle mix"
        );
        for (indent, nl) in all_format_options() {
            let got = format_xml(input, indent, nl).unwrap();
            assert_eq!(
                av_normalized_attr_codes(&got, "a"),
                expected,
                "indent={indent:?} nl={nl} output={got:?}"
            );
            assert_eq!(attr_value(&got, "a"), "\"&<>\t\n\r");
            format_xml(&got, Indentation::Minified, false).expect("output must re-parse");
        }
    }
}
