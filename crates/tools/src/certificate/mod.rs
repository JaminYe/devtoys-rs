mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use crate::catalog::Tool;
use crate::cli::CliTool;
#[cfg(feature = "gui")]
use crate::slot::ToolHandle;
pub use cli::cli_tool;
use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};
pub use helper::{decode_certificate, looks_like_pem_certificate, CertificateError};

#[cfg(feature = "gui")]
pub use view::CertificateView;

pub const ID: &str = "CertificateDecoder";
pub const TYPE_CERTIFICATE: &str = "Certificate";

pub fn metadata() -> ToolMetadata {
    ToolMetadata {
        id: ToolId::new(ID),
        display_name: "证书",
        search_keywords: &["pem", "pfx", "x509"],
        group: GroupId::EncodersDecoders,
        searchable: true,
        favorable: true,
        accepted_types: &[TYPE_CERTIFICATE],
    }
}

pub fn detectors() -> Vec<Box<dyn Detector>> {
    vec![Box::new(detector::CertificateDetector)]
}

#[cfg(feature = "gui")]
pub fn open_view() -> crate::slot::ToolHandle {
    Box::new(CertificateView::new())
}

#[derive(Default, Debug, Clone, Copy)]
pub struct CertificateTool;

impl Tool for CertificateTool {
    fn metadata(&self) -> ToolMetadata {
        metadata()
    }

    fn cli(&self) -> Option<CliTool> {
        Some(cli_tool())
    }

    fn detectors(&self) -> Vec<Box<dyn Detector>> {
        detectors()
    }

    #[cfg(feature = "gui")]
    fn create_view(&self) -> Option<ToolHandle> {
        Some(open_view())
    }
}

#[allow(dead_code)]
pub fn tool() -> Box<dyn Tool> {
    Box::new(CertificateTool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_tool_implements_tool() {
        let tool = CertificateTool;
        assert_eq!(tool.metadata().id.as_str(), ID);
        assert!(tool.cli().is_some());
        assert!(!tool.detectors().is_empty());
        #[cfg(feature = "gui")]
        assert!(tool.create_view().is_some());

        let boxed = super::tool();
        assert_eq!(boxed.metadata().id.as_str(), ID);
    }
}
