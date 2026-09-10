use devtoys_api::DetectedPayload;

/// Smart Detection paste target. Views implement this; the host never names a tool.
pub trait ToolView {
    fn ui(&mut self, ui: &mut egui::Ui);

    /// Text / path paste. Signature is stable; do not add parameters here.
    fn on_data_received(&mut self, payload: &str);

    /// Smart Detection dispatch. Default forwards `payload.value` so existing
    /// text tools keep working; non-empty `payload.bytes` go to
    /// [`Self::on_image_received`] instead of being treated as a file path.
    fn on_detected_data(&mut self, payload: &DetectedPayload) {
        if let Some(bytes) = payload.image_bytes() {
            self.on_image_received(bytes, payload.mime.as_deref());
            return;
        }
        self.on_data_received(&payload.value);
    }

    /// Binary clipboard/image paste. Default ignores the data.
    fn on_image_received(&mut self, bytes: &[u8], mime: Option<&str>) {
        let _ = (bytes, mime);
    }

    /// Non-sensitive options to persist. `None` means this tool has nothing to save.
    ///
    /// Host writes the `Value` to `AppSettings.tool_options[id]`. Override in 22/23
    /// to opt in; do not change [`Self::on_data_received`] / [`Self::on_detected_data`]
    /// / [`Self::on_image_received`] signatures.
    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        None
    }

    /// Restore options previously stored under this tool's id.
    ///
    /// Missing or unknown fields must fall back to the view's documented defaults.
    /// Host calls this when creating a session; views must not open settings files.
    fn restore_options(&mut self, value: &serde_json::Value) {
        let _ = value;
    }
}

/// Opaque GUI session for one open tool. Host stores these by tool id.
pub type ToolHandle = Box<dyn ToolView>;
