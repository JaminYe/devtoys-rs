mod cli;
mod detector;
mod helper;
#[cfg(feature = "gui")]
mod view;

use devtoys_api::{Detector, GroupId, ToolId, ToolMetadata};

pub use cli::cli_tool;
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
pub fn open_view(window: &mut gpui::Window, cx: &mut gpui::App) -> crate::slot::ToolHandle {
    crate::slot::ToolHandle::open(window, cx, CertificateView::new)
}
