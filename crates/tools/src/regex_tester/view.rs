use crate::slot::ToolView;
use crate::ui;

use super::helper::{test_regex, RegexMatch, RegexOptions, CHEAT_SHEET};

pub struct RegexTesterView {
    pattern: String,
    sample: String,
    output: String,
    options: RegexOptions,
    error: Option<String>,
}

impl RegexTesterView {
    pub fn new() -> Self {
        Self {
            pattern: String::new(),
            sample: String::new(),
            output: String::new(),
            options: RegexOptions::default(),
            error: None,
        }
    }

    fn rematch(&mut self) {
        if self.pattern.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match test_regex(&self.pattern, &self.sample, &self.options) {
            Ok(matches) => {
                self.error = None;
                self.output = format_matches(&matches);
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

fn format_matches(matches: &[RegexMatch]) -> String {
    if matches.is_empty() {
        return "无匹配".into();
    }
    let mut lines = Vec::new();
    for m in matches {
        lines.push(format!(
            "匹配 {}  [{}-{}]  {}",
            m.index + 1,
            m.start,
            m.end,
            m.value
        ));
        for g in &m.groups {
            let label = match &g.name {
                Some(name) => format!("分组 {name}"),
                None => format!("分组 {}", g.index),
            };
            lines.push(format!("  {label}  [{}-{}]  {}", g.start, g.end, g.value));
        }
    }
    lines.join("\n")
}

impl ToolView for RegexTesterView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let mut rematch = false;
            rematch |= ui
                .checkbox(&mut self.options.all_matches, "全部匹配")
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.ignore_case, "忽略大小写")
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.ignore_whitespace, "忽略空白")
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.singleline, "Singleline")
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.multiline, "Multiline")
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.ecmascript, "ECMAScript")
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.culture_invariant, "区域固定")
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.right_to_left, "从右向左")
                .changed();
            if rematch {
                self.rematch();
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        ui::error_label(ui, self.error.as_deref());
        let spacing = 12.0;
        let total = ui.available_size();
        let w = ((total.x - spacing) / 2.0).max(80.0);
        ui.horizontal(|ui| {
            ui.set_min_height(total.y);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                ui.vertical(|ui| {
                    ui.label("表达式");
                    if ui::singleline(ui, "re-pat", &mut self.pattern, "正则表达式") {
                        self.rematch();
                    }
                    if ui::labeled_code(
                        ui,
                        "样例文本",
                        "re-sample",
                        &mut self.sample,
                        "样例文本",
                        true,
                    ) {
                        self.rematch();
                    }
                });
            });
            ui.add_space(spacing);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                ui.vertical(|ui| {
                    let cheat_reserve = 88.0;
                    let result_h = (ui.available_height() - cheat_reserve).max(80.0);
                    ui.allocate_ui(egui::vec2(ui.available_width(), result_h), |ui| {
                        ui::labeled_code(
                            ui,
                            "匹配结果",
                            "re-out",
                            &mut self.output,
                            "匹配与分组",
                            false,
                        );
                    });
                    ui.label("速查表");
                    ui.horizontal_wrapped(|ui| {
                        for (syntax, desc) in CHEAT_SHEET {
                            ui.label(format!("{syntax}  {desc}"));
                        }
                    });
                });
            });
        });
    }

    fn on_data_received(&mut self, payload: &str) {
        self.sample = payload.to_string();
        self.rematch();
    }
}
