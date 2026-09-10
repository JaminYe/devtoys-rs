use crate::slot::ToolView;
use crate::ui;

use super::helper::{
    evaluate_gui_pattern, match_table_rows, substitute, MatchRowKind, RegexMatch, RegexOptions,
    CHEAT_SHEET, ENGINE_NOTE,
};
use super::ID;

/// Settings JSON booleans: `all_matches` | `ignore_case` | `ignore_whitespace` | `singleline` | `multiline`.
/// Unknown / missing → all_matches=true, others false. Pattern/sample/replacement are not persisted.

pub struct RegexTesterView {
    pattern: String,
    sample: String,
    replacement: String,
    matches: Vec<RegexMatch>,
    replaced: String,
    options: RegexOptions,
    error: Option<String>,
}

impl RegexTesterView {
    pub fn new() -> Self {
        Self {
            pattern: String::new(),
            sample: String::new(),
            replacement: String::new(),
            matches: Vec::new(),
            replaced: String::new(),
            options: RegexOptions::default(),
            error: None,
        }
    }

    fn rematch(&mut self) {
        match evaluate_gui_pattern(&self.pattern, &self.sample, &self.options) {
            Ok(None) => {
                self.error = None;
                self.matches.clear();
                self.replaced.clear();
            }
            Ok(Some(matches)) => {
                self.matches = matches;
                if self.replacement.is_empty() {
                    self.error = None;
                    self.replaced.clear();
                } else {
                    match substitute(
                        &self.pattern,
                        &self.sample,
                        &self.replacement,
                        &self.options,
                    ) {
                        Ok(replaced) => {
                            self.error = None;
                            self.replaced = replaced;
                        }
                        Err(err) => {
                            self.error = Some(err.to_string());
                            self.replaced.clear();
                        }
                    }
                }
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.matches.clear();
                self.replaced.clear();
            }
        }
    }
}

fn draw_match_table(ui: &mut egui::Ui, matches: &[RegexMatch]) {
    egui::ScrollArea::vertical()
        .id_salt("re-match-table-scroll")
        .show(ui, |ui| {
            egui::Grid::new("re-match-table")
                .num_columns(4)
                .striped(true)
                .min_col_width(40.0)
                .show(ui, |ui| {
                    ui.strong("名称");
                    ui.strong("起始");
                    ui.strong("结束");
                    ui.strong("文本");
                    ui.end_row();
                    if matches.is_empty() {
                        ui.label("无匹配");
                        ui.label("");
                        ui.label("");
                        ui.label("");
                        ui.end_row();
                    } else {
                        for row in match_table_rows(matches) {
                            match row.kind {
                                MatchRowKind::Match => {
                                    ui.strong(&row.label);
                                }
                                MatchRowKind::Group => {
                                    ui.label(&row.label);
                                }
                            }
                            ui.monospace(row.start.to_string());
                            ui.monospace(row.end.to_string());
                            ui.monospace(&row.text);
                            ui.end_row();
                        }
                    }
                });
        });
}

impl ToolView for RegexTesterView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let mut rematch = false;
            rematch |= ui
                .checkbox(&mut self.options.all_matches, "全部匹配")
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.ignore_case, ui::t(ui, "regex.ignore_case"))
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.ignore_whitespace, "忽略空白")
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.singleline, ui::t(ui, "regex.singleline"))
                .changed();
            rematch |= ui
                .checkbox(&mut self.options.multiline, ui::t(ui, "regex.multiline"))
                .changed();
            if rematch {
                self.rematch();
            }
        });
        ui.label(ENGINE_NOTE);
        ui::error_label(ui, self.error.as_deref());
        let spacing = 12.0;
        let total = ui.available_size();
        let w = ((total.x - spacing) / 2.0).max(80.0);
        ui.horizontal(|ui| {
            ui.set_min_height(total.y);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                ui.vertical(|ui| {
                    ui.label(ui::t(ui, "regex.pattern"));
                    if ui::singleline(ui, "re-pat", &mut self.pattern, ui::t(ui, "regex.pattern")) {
                        self.rematch();
                    }
                    let replacement_reserve = 52.0;
                    let sample_h = (ui.available_height() - replacement_reserve).max(80.0);
                    ui.allocate_ui(egui::vec2(ui.available_width(), sample_h), |ui| {
                        if ui::labeled_code(
                            ui,
                            ui::t(ui, "regex.sample"),
                            "re-sample",
                            &mut self.sample,
                            ui::t(ui, "regex.sample"),
                            true,
                        ) {
                            self.rematch();
                        }
                    });
                    ui.label(ui::t(ui, "regex.substitution"));
                    if ui::singleline(ui, "re-repl", &mut self.replacement, r"$0 $1 ${name} \n \t")
                    {
                        self.rematch();
                    }
                });
            });
            ui.add_space(spacing);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                ui.vertical(|ui| {
                    let cheat_reserve = 88.0;
                    let rest = (ui.available_height() - cheat_reserve).max(80.0);
                    let table_h = (rest * 0.55).max(80.0);
                    let result_h = (rest - table_h).max(80.0);
                    ui.allocate_ui(egui::vec2(ui.available_width(), table_h), |ui| {
                        ui.vertical(|ui| {
                            ui.label(ui::t(ui, "regex.matches"));
                            draw_match_table(ui, &self.matches);
                        });
                    });
                    ui.allocate_ui(egui::vec2(ui.available_width(), result_h), |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(ui::t(ui, "common.output"));
                                ui::copy_button(
                                    ui,
                                    self.error.is_none().then_some(self.replaced.as_str()),
                                );
                            });
                            ui::fill_code(ui, "re-replaced", &mut self.replaced, ui::t(ui, "common.output"), false);
                        });
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

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "all_matches": self.options.all_matches,
                "ignore_case": self.options.ignore_case,
                "ignore_whitespace": self.options.ignore_whitespace,
                "singleline": self.options.singleline,
                "multiline": self.options.multiline,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.options.all_matches = bool_or(value, "all_matches", true);
        self.options.ignore_case = bool_or(value, "ignore_case", false);
        self.options.ignore_whitespace = bool_or(value, "ignore_whitespace", false);
        self.options.singleline = bool_or(value, "singleline", false);
        self.options.multiline = bool_or(value, "multiline", false);
        self.rematch();
    }
}

fn bool_or(value: &serde_json::Value, key: &str, default: bool) -> bool {
    value.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn persistable_options_are_flags_only() {
        let mut view = RegexTesterView::new();
        view.options.ignore_case = true;
        view.options.all_matches = false;
        view.options.ignore_whitespace = true;
        view.pattern = "a+".into();
        view.sample = "aaa".into();
        view.replacement = "$0".into();
        view.replaced = "secret-re".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["all_matches"], false);
        assert_eq!(value["ignore_case"], true);
        assert_eq!(value["ignore_whitespace"], true);
        assert_eq!(value["singleline"], false);
        assert_eq!(value["multiline"], false);
        assert!(value.get("pattern").is_none());
        assert!(value.get("sample").is_none());
        assert!(value.get("replacement").is_none());
        let dumped = value.to_string();
        assert!(!dumped.contains("a+"));
        assert!(!dumped.contains("secret-re"));
        assert!(!dumped.contains("$0"));
    }

    #[test]
    fn restore_ignore_case_first_match_then_evaluates() {
        let mut view = RegexTesterView::new();
        view.restore_options(&serde_json::json!({
            "all_matches": false,
            "ignore_case": true
        }));
        assert!(!view.options.all_matches);
        assert!(view.options.ignore_case);
        view.pattern = "a".into();
        view.sample = "aA".into();
        view.rematch();
        assert!(view.error.is_none());
        assert_eq!(view.matches.len(), 1);
        assert_eq!(view.matches[0].start, 0);
        assert_eq!(view.matches[0].end, 1);
        assert_eq!(view.matches[0].value, "a");
        assert!(view.replaced.is_empty());
    }

    #[test]
    fn restore_ignore_whitespace_then_matches_without_spaces() {
        let mut view = RegexTesterView::new();
        view.restore_options(&serde_json::json!({
            "ignore_whitespace": true
        }));
        assert!(view.options.ignore_whitespace);
        assert!(view.options.all_matches);
        view.pattern = "a b".into();
        view.sample = "ab".into();
        view.rematch();
        assert!(view.error.is_none());
        assert_eq!(view.matches.len(), 1);
        assert_eq!(view.matches[0].value, "ab");
    }

    #[test]
    fn missing_fields_use_documented_defaults() {
        let mut view = RegexTesterView::new();
        view.options.all_matches = false;
        view.options.ignore_case = true;
        view.options.multiline = true;
        view.restore_options(&serde_json::json!({}));
        assert!(view.options.all_matches);
        assert!(!view.options.ignore_case);
        assert!(!view.options.ignore_whitespace);
        assert!(!view.options.singleline);
        assert!(!view.options.multiline);
    }

    #[test]
    fn illegal_replacement_clears_stale_output_keeps_matches() {
        let mut view = RegexTesterView::new();
        view.pattern = r"(\w+)@(\w+)".into();
        view.sample = "a@b".into();
        view.replacement = "$2/$1".into();
        view.rematch();
        assert!(view.error.is_none());
        assert_eq!(view.matches.len(), 1);
        assert_eq!(view.replaced, "b/a");
        view.replacement = r"\x".into();
        view.rematch();
        assert_eq!(view.error.as_deref(), Some("非法替换式"));
        assert!(view.replaced.is_empty(), "must not keep stale replacement");
        assert_eq!(view.matches.len(), 1);
        assert_eq!(view.matches[0].value, "a@b");
    }

    #[test]
    fn no_match_clears_stale_table_rows() {
        let mut view = RegexTesterView::new();
        view.pattern = r"(\w+)@(\w+)".into();
        view.sample = "a@b".into();
        view.rematch();
        assert_eq!(view.matches.len(), 1);
        view.pattern = "xyz".into();
        view.rematch();
        assert!(view.error.is_none());
        assert!(view.matches.is_empty());
        assert!(view.replaced.is_empty());
    }

    #[test]
    fn test_regex_i18n_keys() {
        let keys = [
            ("regex.pattern", "正则表达式", "Regular expression"),
            ("regex.sample", "测试文本", "Sample text"),
            ("regex.substitution", "替换式", "Substitution"),
            ("regex.ignore_case", "忽略大小写", "Ignore case"),
            ("regex.multiline", "多行模式", "Multiline"),
            ("regex.singleline", "单行模式", "Singleline"),
            ("regex.matches", "匹配项", "Matches"),
        ];
        for (key, zh, en) in keys {
            assert_eq!(devtoys_api::t(key, devtoys_api::Language::ZhCn), zh);
            assert_eq!(devtoys_api::t(key, devtoys_api::Language::EnUs), en);
        }
    }
}
