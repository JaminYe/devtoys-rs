use crate::slot::ToolView;
use crate::ui;

use super::{compare_lists, ListMode};

pub struct ListCompareView {
    list_a: String,
    list_b: String,
    output: String,
    mode: ListMode,
    case_sensitive: bool,
}

impl ListCompareView {
    pub fn new() -> Self {
        Self {
            list_a: String::new(),
            list_b: String::new(),
            output: String::new(),
            mode: ListMode::AInterB,
            case_sensitive: false,
        }
    }

    fn recompute(&mut self) {
        self.output = compare_lists(&self.list_a, &self.list_b, self.mode, self.case_sensitive);
    }

    fn set_mode(&mut self, mode: ListMode) {
        self.mode = mode;
        self.recompute();
    }
}

impl ToolView for ListCompareView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("交集", ListMode::AInterB),
                ("并集", ListMode::AUnionB),
                ("仅 A", ListMode::AOnly),
                ("仅 B", ListMode::BOnly),
            ] {
                if ui::toggle(ui, self.mode == value, label).clicked() {
                    self.set_mode(value);
                }
            }
            if ui
                .checkbox(&mut self.case_sensitive, "大小写敏感")
                .changed()
            {
                self.recompute();
            }
            ui::copy_button(ui, Some(self.output.as_str()));
        });
        let spacing = 12.0;
        let total = ui.available_size();
        let w = ((total.x - spacing * 2.0) / 3.0).max(80.0);
        ui.horizontal(|ui| {
            ui.set_min_height(total.y);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                if ui::labeled_code(
                    ui,
                    "列表 A",
                    "list-a",
                    &mut self.list_a,
                    "列表 A，一行一项",
                    true,
                ) {
                    self.recompute();
                }
            });
            ui.add_space(spacing);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                if ui::labeled_code(
                    ui,
                    "列表 B",
                    "list-b",
                    &mut self.list_b,
                    "列表 B，一行一项",
                    true,
                ) {
                    self.recompute();
                }
            });
            ui.add_space(spacing);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                ui::labeled_code(ui, "结果", "list-out", &mut self.output, "比对结果", false);
            });
        });
    }

    fn on_data_received(&mut self, _payload: &str) {}
}
