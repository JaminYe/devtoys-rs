use crate::slot::ToolView;
use crate::ui;

use super::{diff_lines, DiffMode, DiffTag};

pub struct TextCompareView {
    left: String,
    right: String,
    mode: DiffMode,
}

impl TextCompareView {
    pub fn new() -> Self {
        Self {
            left: String::new(),
            right: String::new(),
            mode: DiffMode::SideBySide,
        }
    }
}

fn diff_line(ui: &mut egui::Ui, text: &str, color: Option<egui::Color32>) {
    let mut rt = egui::RichText::new(text).monospace();
    if let Some(c) = color {
        rt = rt.color(c);
    }
    ui.label(rt);
}

impl ToolView for TextCompareView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui::toggle(ui, self.mode == DiffMode::SideBySide, "并排").clicked() {
                self.mode = DiffMode::SideBySide;
            }
            if ui::toggle(ui, self.mode == DiffMode::Inline, "行内").clicked() {
                self.mode = DiffMode::Inline;
            }
        });
        let editor_h = 200.0;
        ui.allocate_ui(egui::vec2(ui.available_width(), editor_h), |ui| {
            ui::split_2(
                ui,
                |ui| {
                    ui::labeled_code(ui, "左侧", "diff-left", &mut self.left, "左侧文本", true);
                },
                |ui| {
                    ui::labeled_code(ui, "右侧", "diff-right", &mut self.right, "右侧文本", true);
                },
            );
        });
        ui.label("差异");
        let hunks = diff_lines(&self.left, &self.right);
        let danger = ui::danger(ui);
        let success = ui::success(ui);
        let inline = self.mode == DiffMode::Inline;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if inline {
                    for hunk in &hunks {
                        let (color, prefix) = match hunk.tag {
                            DiffTag::Delete => (Some(danger), "- "),
                            DiffTag::Insert => (Some(success), "+ "),
                            DiffTag::Equal => (None, "  "),
                        };
                        diff_line(ui, &format!("{prefix}{}", hunk.text.trim_end()), color);
                    }
                } else {
                    ui.columns(2, |cols| {
                        for hunk in &hunks {
                            match hunk.tag {
                                DiffTag::Insert => {}
                                DiffTag::Delete => {
                                    diff_line(&mut cols[0], hunk.text.trim_end(), Some(danger));
                                }
                                DiffTag::Equal => {
                                    diff_line(&mut cols[0], hunk.text.trim_end(), None);
                                }
                            }
                        }
                        for hunk in &hunks {
                            match hunk.tag {
                                DiffTag::Delete => {}
                                DiffTag::Insert => {
                                    diff_line(&mut cols[1], hunk.text.trim_end(), Some(success));
                                }
                                DiffTag::Equal => {
                                    diff_line(&mut cols[1], hunk.text.trim_end(), None);
                                }
                            }
                        }
                    });
                }
            });
    }

    fn on_data_received(&mut self, _payload: &str) {}
}
