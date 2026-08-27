use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_TEXT};

use super::{looks_like_pem_certificate, TYPE_CERTIFICATE};

#[derive(Debug, Default, Clone, Copy)]
pub struct CertificateDetector;

impl Detector for CertificateDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_CERTIFICATE,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        if !looks_like_pem_certificate(&parent.value) {
            return None;
        }
        Some(DetectedPayload {
            type_name: TYPE_CERTIFICATE.to_string(),
            value: parent.value.clone(),
        })
    }
}
