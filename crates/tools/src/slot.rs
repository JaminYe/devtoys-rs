/// Smart Detection paste target. Views implement this; the host never names a tool.
pub trait ToolView {
    fn ui(&mut self, ui: &mut egui::Ui);
    fn on_data_received(&mut self, payload: &str);
}

/// Opaque GUI session for one open tool. Host stores these by tool id.
pub type ToolHandle = Box<dyn ToolView>;
