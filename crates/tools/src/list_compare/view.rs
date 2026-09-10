use crate::slot::ToolView;
use crate::ui;

use super::{compare_lists, ListMode, ID};

/// Settings JSON `mode`: `a_inter_b` | `a_union_b` | `a_only` | `b_only`.
/// Unknown / missing → AInterB. Booleans missing → false.
const DEFAULT_MODE: ListMode = ListMode::AInterB;
const DEFAULT_CASE_SENSITIVE: bool = false;
const DEFAULT_IGNORE_SURROUNDING_WHITESPACE: bool = false;

pub struct ListCompareView {
    list_a: String,
    list_b: String,
    output: String,
    mode: ListMode,
    case_sensitive: bool,
    ignore_surrounding_whitespace: bool,
}

impl ListCompareView {
    pub fn new() -> Self {
        Self {
            list_a: String::new(),
            list_b: String::new(),
            output: String::new(),
            mode: ListMode::AInterB,
            case_sensitive: false,
            ignore_surrounding_whitespace: false,
        }
    }

    fn recompute(&mut self) {
        self.output = compare_lists(
            &self.list_a,
            &self.list_b,
            self.mode,
            self.case_sensitive,
            self.ignore_surrounding_whitespace,
        );
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
                (ui::t(ui, "list_compare.intersection"), ListMode::AInterB),
                ("并集", ListMode::AUnionB),
                (ui::t(ui, "list_compare.difference_a"), ListMode::AOnly),
                (ui::t(ui, "list_compare.difference_b"), ListMode::BOnly),
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
            if ui
                .checkbox(&mut self.ignore_surrounding_whitespace, "忽略首尾空白")
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
                    ui::t(ui, "list_compare.list_a"),
                    "list-a",
                    &mut self.list_a,
                    ui::t(ui, "list_compare.list_a"),
                    true,
                ) {
                    self.recompute();
                }
            });
            ui.add_space(spacing);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                if ui::labeled_code(
                    ui,
                    ui::t(ui, "list_compare.list_b"),
                    "list-b",
                    &mut self.list_b,
                    ui::t(ui, "list_compare.list_b"),
                    true,
                ) {
                    self.recompute();
                }
            });
            ui.add_space(spacing);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                ui::labeled_code(
                    ui,
                    ui::t(ui, "common.output"),
                    "list-out",
                    &mut self.output,
                    ui::t(ui, "common.output"),
                    false,
                );
            });
        });
    }

    fn on_data_received(&mut self, _payload: &str) {}

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "mode": mode_to_settings(self.mode),
                "case_sensitive": self.case_sensitive,
                "ignore_surrounding_whitespace": self.ignore_surrounding_whitespace,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.mode = mode_from_settings(value.get("mode").and_then(|v| v.as_str()));
        self.case_sensitive = value
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_CASE_SENSITIVE);
        self.ignore_surrounding_whitespace = value
            .get("ignore_surrounding_whitespace")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_IGNORE_SURROUNDING_WHITESPACE);
        self.recompute();
    }
}

fn mode_to_settings(mode: ListMode) -> &'static str {
    match mode {
        ListMode::AInterB => "a_inter_b",
        ListMode::AUnionB => "a_union_b",
        ListMode::AOnly => "a_only",
        ListMode::BOnly => "b_only",
    }
}

fn mode_from_settings(value: Option<&str>) -> ListMode {
    match value {
        Some("a_inter_b") => ListMode::AInterB,
        Some("a_union_b") => ListMode::AUnionB,
        Some("a_only") => ListMode::AOnly,
        Some("b_only") => ListMode::BOnly,
        _ => DEFAULT_MODE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn persistable_options_are_mode_and_flags_only() {
        let mut view = ListCompareView::new();
        view.mode = ListMode::AOnly;
        view.case_sensitive = true;
        view.ignore_surrounding_whitespace = true;
        view.list_a = "alpha".into();
        view.list_b = "beta".into();
        view.output = "alpha".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["mode"], "a_only");
        assert_eq!(value["case_sensitive"], true);
        assert_eq!(value["ignore_surrounding_whitespace"], true);
        assert!(value.get("list_a").is_none());
        assert!(value.get("list_b").is_none());
        assert!(!value.to_string().contains("alpha"));
    }

    #[test]
    fn restore_a_only_case_sensitive_ignore_ws_then_compares() {
        let mut view = ListCompareView::new();
        view.restore_options(&serde_json::json!({
            "mode": "a_only",
            "case_sensitive": true,
            "ignore_surrounding_whitespace": true
        }));
        assert_eq!(view.mode, ListMode::AOnly);
        assert!(view.case_sensitive);
        assert!(view.ignore_surrounding_whitespace);
        view.list_a = " A \nB\nb".into();
        view.list_b = "A\nB".into();
        view.recompute();
        assert_eq!(view.output, "b");
    }

    #[test]
    fn missing_and_illegal_mode_use_defaults() {
        let mut view = ListCompareView::new();
        view.mode = ListMode::BOnly;
        view.case_sensitive = true;
        view.ignore_surrounding_whitespace = true;
        view.restore_options(&serde_json::json!({ "mode": "xor" }));
        assert_eq!(view.mode, ListMode::AInterB);
        assert!(!view.case_sensitive);
        assert!(!view.ignore_surrounding_whitespace);
        view.list_a = "A\nB".into();
        view.list_b = "a".into();
        view.recompute();
        assert_eq!(view.output, "A");
    }

    #[test]
    fn test_list_compare_i18n_keys() {
        let keys = [
            ("list_compare.list_a", "列表 A", "List A"),
            ("list_compare.list_b", "列表 B", "List B"),
            (
                "list_compare.intersection",
                "交集 (A ∩ B)",
                "Intersection (A ∩ B)",
            ),
            (
                "list_compare.difference_a",
                "A 独有 (A - B)",
                "Only in A (A - B)",
            ),
            (
                "list_compare.difference_b",
                "B 独有 (B - A)",
                "Only in B (B - A)",
            ),
        ];
        for (key, zh, en) in keys {
            assert_eq!(devtoys_api::t(key, devtoys_api::Language::ZhCn), zh);
            assert_eq!(devtoys_api::t(key, devtoys_api::Language::EnUs), en);
        }
    }
}
