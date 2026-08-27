//! Registry, navigation, settings store, and Smart Detection engine.

mod detection;
mod detectors;
mod error;
mod registry;
mod settings_store;
mod state;

pub use detection::{DetectOptions, DetectionEngine, Recommendation};
pub use detectors::{
    Base64ImageDetector, Base64TextDetector, DateDetector, FileDetector, FilesDetector,
    GzipDetector, ImageDetector, ImageFileDetector, JsonArrayDetector, JsonDetector, TextDetector,
    XmlDetector, XsdDetector,
};
pub use error::CoreError;
pub use registry::{SearchOutcome, ToolRegistry};
pub use settings_store::SettingsStore;
pub use state::AppState;

use devtoys_api::Detector;

/// Host generic Smart Detection detectors (`text` → nested types, plus `image` / `files`).
pub fn all_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(TextDetector),
        Box::new(JsonDetector),
        Box::new(JsonArrayDetector),
        Box::new(XmlDetector),
        Box::new(XsdDetector),
        Box::new(Base64TextDetector),
        Box::new(Base64ImageDetector),
        Box::new(GzipDetector),
        Box::new(DateDetector),
        Box::new(ImageDetector),
        Box::new(FilesDetector),
        Box::new(FileDetector),
        Box::new(ImageFileDetector),
    ]
}
