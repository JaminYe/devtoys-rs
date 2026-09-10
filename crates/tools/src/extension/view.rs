use crate::slot::ToolView;
use crate::ui;

use super::helper::{apply_transform, Transform};

pub struct ManifestToolView {
    input: String,
    output: String,
    transform: Transform,
}

impl ManifestToolView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            output: String::new(),
            transform: Transform::Uppercase,
        }
    }

    fn apply(&mut self) {
        self.output = apply_transform(&self.input, self.transform);
    }
}

impl ToolView for ManifestToolView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [("原样", Transform::Echo), ("大写", Transform::Uppercase)] {
                if ui::toggle(ui, self.transform == value, label).clicked() {
                    self.transform = value;
                    self.apply();
                }
            }
            ui::copy_button(ui, Some(self.output.as_str()));
        });
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed =
                    ui::labeled_code(ui, "输入", "ext-in", &mut self.input, "粘贴文本", true);
            },
            |ui| {
                ui::labeled_code(ui, "输出", "ext-out", &mut self.output, "结果", false);
            },
        );
        if input_changed {
            self.apply();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.input = payload.to_string();
        self.apply();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paste_runs_uppercase() {
        let mut view = ManifestToolView::new();
        view.on_data_received("hello");
        assert_eq!(view.output, "HELLO");
        view.transform = Transform::Echo;
        view.apply();
        assert_eq!(view.output, "hello");
    }
}
