//! Public types for the host, tools, and CLI.
//!
//! This crate is the only place those callers agree on IDs, grouping,
//! tool metadata, settings, and the detector trait. No algorithms live here.

mod detection;
mod groups;
mod i18n;
mod settings;
mod tool;

pub use detection::{
    DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_BASE64_IMAGE, TYPE_BASE64_TEXT,
    TYPE_DATE, TYPE_FILE, TYPE_FILES, TYPE_IMAGE, TYPE_IMAGE_FILE, TYPE_JSON, TYPE_JSON_ARRAY,
    TYPE_TEXT, TYPE_XML,
};
pub use groups::{GroupId, ALL_TOOLS_ID, ALL_TOOLS_LABEL, FAVORITES_ID, FAVORITES_LABEL};
pub use i18n::t;
pub use settings::{AppSettings, ThemePreference, WindowState};
pub use tool::{ToolId, ToolMetadata, JSON_FORMATTER_ID, SETTINGS_ID};
