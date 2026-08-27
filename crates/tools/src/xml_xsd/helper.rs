use std::collections::HashSet;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

const XML_SCHEMA_NS: &str = "http://www.w3.org/2001/XMLSchema";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlReportLevel {
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlReport {
    pub level: XmlReportLevel,
    pub line: Option<usize>,
    pub message: String,
}

struct XmlDoc {
    root_name: String,
    root_line: usize,
    children: Vec<String>,
}

struct XsdDoc {
    root_name: Option<String>,
    required_children: Vec<String>,
}

/// Practical XSD subset: well-formed XML, root `xs:element` name match,
/// required child names from `xs:sequence` (`minOccurs` default 1).
/// Types, attributes, wildcards, and full identity constraints are not enforced.
pub fn validate_xml_xsd(xml: &str, xsd: &str) -> Vec<XmlReport> {
    if xml.trim().is_empty() {
        return vec![error(None, "缺少 XML")];
    }

    let xml_doc = match parse_xml(xml) {
        Ok(doc) => doc,
        Err(report) => return vec![report],
    };

    if xsd.trim().is_empty() {
        return vec![success(None, "XML 格式正确（未提供 XSD）")];
    }

    let xsd_doc = match parse_xsd(xsd) {
        Ok(doc) => doc,
        Err(report) => return vec![report],
    };

    let mut reports = Vec::new();

    if xsd_doc.root_name.is_none() {
        reports.push(warning(
            None,
            "XSD 未声明根 xs:element，跳过根名校验",
        ));
    }

    if let Some(expected) = xsd_doc.root_name.as_deref() {
        if xml_doc.root_name != expected {
            reports.push(error(
                Some(xml_doc.root_line),
                format!(
                    "根元素不匹配：期望 <{}>，实际 <{}>",
                    expected, xml_doc.root_name
                ),
            ));
        }
    }

    for required in &xsd_doc.required_children {
        if !xml_doc.children.iter().any(|c| c == required) {
            reports.push(error(
                Some(xml_doc.root_line),
                format!("缺少必需元素 <{}>", required),
            ));
        }
    }

    if reports.is_empty() {
        reports.push(success(None, "XML 符合 XSD"));
    }
    reports
}

pub fn looks_like_xsd(payload: &str) -> bool {
    payload.contains("xs:schema")
        || payload.contains("xsd:schema")
        || payload.contains("xmlns:xs")
        || payload.contains("xmlns:xsd")
}

fn parse_xml(xml: &str) -> Result<XmlDoc, XmlReport> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut root_name = None;
    let mut root_line = 1usize;
    let mut children = Vec::new();
    let mut depth = 0i32;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = local_name(&e);
                let line = byte_to_line(xml, reader.buffer_position());
                depth += 1;
                if depth == 1 {
                    root_name = Some(name);
                    root_line = line;
                } else if depth == 2 {
                    children.push(name);
                }
            }
            Ok(Event::Empty(e)) => {
                let name = local_name(&e);
                let line = byte_to_line(xml, reader.buffer_position());
                if depth == 0 {
                    root_name = Some(name);
                    root_line = line;
                } else if depth == 1 {
                    children.push(name);
                }
            }
            Ok(Event::End(_)) => {
                depth -= 1;
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                let line = byte_to_line(xml, reader.error_position());
                return Err(error(Some(line), format!("XML 无法解析：{err}")));
            }
            _ => {}
        }
    }

    if depth != 0 {
        return Err(error(
            Some(byte_to_line(xml, reader.buffer_position())),
            "XML 标签未闭合",
        ));
    }

    match root_name {
        Some(root_name) => Ok(XmlDoc {
            root_name,
            root_line,
            children,
        }),
        None => Err(error(Some(1), "XML 缺少根元素")),
    }
}

fn parse_xsd(xsd: &str) -> Result<XsdDoc, XmlReport> {
    let mut reader = Reader::from_str(xsd);
    reader.config_mut().trim_text(true);

    let mut schema_prefixes = HashSet::new();
    let mut default_schema = false;
    let mut saw_schema = false;
    let mut root_name = None;
    let mut required_children = Vec::new();
    let mut depth = 0i32;
    let mut schema_depth: Option<i32> = None;

    loop {
        let event = reader.read_event();
        match event {
            Ok(Event::Start(e)) => {
                apply_xmlns(&e, &mut schema_prefixes, &mut default_schema);
                handle_xsd_tag(
                    &e,
                    depth,
                    &schema_prefixes,
                    default_schema,
                    &mut saw_schema,
                    &mut schema_depth,
                    &mut root_name,
                    &mut required_children,
                );
                depth += 1;
            }
            Ok(Event::Empty(e)) => {
                apply_xmlns(&e, &mut schema_prefixes, &mut default_schema);
                handle_xsd_tag(
                    &e,
                    depth,
                    &schema_prefixes,
                    default_schema,
                    &mut saw_schema,
                    &mut schema_depth,
                    &mut root_name,
                    &mut required_children,
                );
            }
            Ok(Event::End(_)) => {
                depth -= 1;
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                let line = byte_to_line(xsd, reader.error_position());
                return Err(error(Some(line), format!("XSD 无法解析：{err}")));
            }
            _ => {}
        }
    }

    if !saw_schema {
        return Err(error(Some(1), "XSD 缺少 xs:schema"));
    }
    Ok(XsdDoc {
        root_name,
        required_children,
    })
}

fn handle_xsd_tag(
    e: &BytesStart<'_>,
    depth: i32,
    schema_prefixes: &HashSet<String>,
    default_schema: bool,
    saw_schema: &mut bool,
    schema_depth: &mut Option<i32>,
    root_name: &mut Option<String>,
    required_children: &mut Vec<String>,
) {
    let qname = qualified_name(e);
    let (in_schema_ns, local) = classify_name(&qname, schema_prefixes, default_schema);

    if in_schema_ns && local == "schema" {
        *saw_schema = true;
        *schema_depth = Some(depth);
    }

    if in_schema_ns && local == "element" {
        if let Some(name) = attr(e, "name") {
            if schema_depth.map(|d| depth == d + 1).unwrap_or(false) && root_name.is_none() {
                *root_name = Some(name);
            } else if schema_depth.map(|d| depth > d + 1).unwrap_or(false) {
                let min_occurs = attr(e, "minOccurs").unwrap_or_else(|| "1".into());
                if min_occurs != "0" {
                    required_children.push(name);
                }
            }
        }
    }
}

fn apply_xmlns(e: &BytesStart<'_>, prefixes: &mut HashSet<String>, default_schema: &mut bool) {
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let value = attr
            .unescape_value()
            .map(|v| v.into_owned())
            .unwrap_or_default();
        if !is_schema_ns(&value) {
            continue;
        }
        if key == "xmlns" {
            *default_schema = true;
        } else if let Some(prefix) = key.strip_prefix("xmlns:") {
            prefixes.insert(prefix.to_string());
        }
    }
}

fn classify_name<'a>(
    qname: &'a str,
    prefixes: &HashSet<String>,
    default_schema: bool,
) -> (bool, &'a str) {
    if let Some((prefix, local)) = qname.split_once(':') {
        let known = prefixes.contains(prefix) || prefix == "xs" || prefix == "xsd";
        (known, local)
    } else {
        (default_schema, qname)
    }
}

fn qualified_name(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.name().as_ref()).into_owned()
}

fn local_name(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

fn attr(e: &BytesStart<'_>, name: &str) -> Option<String> {
    e.try_get_attribute(name)
        .ok()
        .flatten()
        .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
}

fn is_schema_ns(uri: &str) -> bool {
    uri.trim() == XML_SCHEMA_NS
}

fn byte_to_line(src: &str, byte: u64) -> usize {
    let end = (byte as usize).min(src.len());
    src[..end].bytes().filter(|&b| b == b'\n').count() + 1
}

fn success(line: Option<usize>, message: impl Into<String>) -> XmlReport {
    XmlReport {
        level: XmlReportLevel::Success,
        line,
        message: message.into(),
    }
}

fn warning(line: Option<usize>, message: impl Into<String>) -> XmlReport {
    XmlReport {
        level: XmlReportLevel::Warning,
        line,
        message: message.into(),
    }
}

fn error(line: Option<usize>, message: impl Into<String>) -> XmlReport {
    XmlReport {
        level: XmlReportLevel::Error,
        line,
        message: message.into(),
    }
}

pub fn format_reports(reports: &[XmlReport]) -> String {
    reports
        .iter()
        .map(|r| {
            let level = match r.level {
                XmlReportLevel::Success => "Success",
                XmlReportLevel::Warning => "Warning",
                XmlReportLevel::Error => "Error",
            };
            match r.line {
                Some(line) => format!("{level}  行 {line}  {}", r.message),
                None => format!("{level}  {}", r.message),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRIVIAL_XSD: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="root" type="xs:string"/></xs:schema>"#;
    const TRIVIAL_XML: &str = "<root>hello</root>";

    #[test]
    fn well_formed_xml_and_trivial_xsd_success() {
        let reports = validate_xml_xsd(TRIVIAL_XML, TRIVIAL_XSD);
        assert!(
            reports.iter().any(|r| r.level == XmlReportLevel::Success),
            "{reports:?}"
        );
        assert!(reports.iter().all(|r| r.level != XmlReportLevel::Error));
    }

    #[test]
    fn malformed_xml_is_error() {
        let reports = validate_xml_xsd("<root>", TRIVIAL_XSD);
        assert!(
            reports.iter().any(|r| r.level == XmlReportLevel::Error),
            "{reports:?}"
        );
    }

    #[test]
    fn mismatch_reports_error_with_line() {
        let xml = "<foo>hello</foo>";
        let reports = validate_xml_xsd(xml, TRIVIAL_XSD);
        let err = reports
            .iter()
            .find(|r| r.level == XmlReportLevel::Error)
            .expect("expected mismatch error");
        assert!(err.line.is_some(), "mismatch should include a line number");
        assert!(err.message.contains("foo") || err.message.contains("root"));
    }

    #[test]
    fn missing_required_child_is_error() {
        let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xs:element name="root">
    <xs:complexType>
      <xs:sequence>
        <xs:element name="child" type="xs:string"/>
      </xs:sequence>
    </xs:complexType>
  </xs:element>
</xs:schema>"#;
        let reports = validate_xml_xsd("<root></root>", xsd);
        assert!(
            reports
                .iter()
                .any(|r| r.level == XmlReportLevel::Error && r.message.contains("child")),
            "{reports:?}"
        );
    }
}
