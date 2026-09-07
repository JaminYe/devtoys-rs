use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_XML, TYPE_XSD};

use super::helper::looks_like_xsd;

/// `xsd` is a child of `xml`: well-formed XML that carries a schema namespace.
#[derive(Debug, Default, Clone, Copy)]
pub struct XsdDetector;

impl Detector for XsdDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_XSD,
            parent: Some(TYPE_XML),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        if parent.type_name != TYPE_XML {
            return None;
        }
        if !looks_like_xsd(&parent.value) {
            return None;
        }
        Some(DetectedPayload {
            type_name: TYPE_XSD.to_string(),
            value: parent.value.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xml_parent(s: &str) -> DetectedPayload {
        DetectedPayload {
            type_name: TYPE_XML.to_string(),
            value: s.to_string(),
        }
    }

    #[test]
    fn requires_schema_marker_and_xml_parent() {
        let xsd = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema"><xs:element name="a" type="xs:string"/></xs:schema>"#;
        assert!(XsdDetector
            .detect(&RawData::text(xsd), Some(&xml_parent(xsd)))
            .is_some());
        assert!(XsdDetector.detect(&RawData::text(xsd), None).is_none());
        assert!(XsdDetector
            .detect(&RawData::text("<a></a>"), Some(&xml_parent("<a></a>")))
            .is_none());
    }
}
