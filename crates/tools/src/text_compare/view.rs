use crate::slot::ToolView;
use crate::ui;

use super::{diff_lines, diff_rows, DiffLine, DiffMode, DiffRow, DiffSpan, DiffTag, ID};

/// Settings JSON `mode`: `side_by_side` | `inline`. Unknown / missing → SideBySide.
const DEFAULT_MODE: DiffMode = DiffMode::SideBySide;

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

fn side_color(
    tag: DiffTag,
    danger: egui::Color32,
    success: egui::Color32,
) -> Option<egui::Color32> {
    match tag {
        DiffTag::Delete => Some(danger),
        DiffTag::Insert => Some(success),
        DiffTag::Equal => None,
    }
}

fn span_label(ui: &mut egui::Ui, text: &str, color: Option<egui::Color32>) {
    let shown = if text.is_empty() { " " } else { text };
    let mut rt = egui::RichText::new(shown).monospace();
    if let Some(c) = color {
        rt = rt.color(c);
    }
    ui.add(egui::Label::new(rt).wrap_mode(egui::TextWrapMode::Extend));
}

/// Render intra-line spans. Concatenated label text is still the plain line.
fn paint_spans(
    ui: &mut egui::Ui,
    prefix: &str,
    spans: &[DiffSpan],
    danger: egui::Color32,
    success: egui::Color32,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if !prefix.is_empty() {
            span_label(ui, prefix, None);
        }
        let empty = spans.is_empty() || spans.iter().all(|s| s.text.is_empty());
        if empty {
            let color = spans
                .first()
                .and_then(|s| side_color(s.tag, danger, success));
            span_label(ui, " ", color);
            return;
        }
        for span in spans {
            if span.text.is_empty() {
                continue;
            }
            span_label(ui, &span.text, side_color(span.tag, danger, success));
        }
    });
}

fn side_cell(
    ui: &mut egui::Ui,
    cell: Option<&DiffLine>,
    danger: egui::Color32,
    success: egui::Color32,
) {
    match cell {
        Some(line) => paint_spans(ui, "", &line.spans, danger, success),
        None => paint_spans(ui, "", &[], danger, success),
    }
}

impl ToolView for TextCompareView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui::toggle(
                ui,
                self.mode == DiffMode::SideBySide,
                ui::t("text_compare.side_by_side"),
            )
            .clicked()
            {
                self.mode = DiffMode::SideBySide;
            }
            if ui::toggle(
                ui,
                self.mode == DiffMode::Inline,
                ui::t("text_compare.inline"),
            )
            .clicked()
            {
                self.mode = DiffMode::Inline;
            }
        });
        let editor_h = 200.0;
        ui.allocate_ui(egui::vec2(ui.available_width(), editor_h), |ui| {
            ui::split_2(
                ui,
                |ui| {
                    ui::labeled_code(
                        ui,
                        ui::t("text_compare.original"),
                        "diff-left",
                        &mut self.left,
                        ui::t("text_compare.original"),
                        true,
                    );
                },
                |ui| {
                    ui::labeled_code(
                        ui,
                        ui::t("text_compare.modified"),
                        "diff-right",
                        &mut self.right,
                        ui::t("text_compare.modified"),
                        true,
                    );
                },
            );
        });
        ui.label("差异");
        let danger = ui::danger(ui);
        let success = ui::success(ui);
        let inline = self.mode == DiffMode::Inline;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if inline {
                    for hunk in diff_lines(&self.left, &self.right) {
                        let prefix = match hunk.tag {
                            DiffTag::Delete => "- ",
                            DiffTag::Insert => "+ ",
                            DiffTag::Equal => "  ",
                        };
                        paint_spans(ui, prefix, &hunk.spans, danger, success);
                    }
                } else {
                    let rows: Vec<DiffRow> = diff_rows(&self.left, &self.right);
                    ui.columns(2, |cols| {
                        for row in &rows {
                            side_cell(&mut cols[0], row.left.as_ref(), danger, success);
                        }
                        for row in &rows {
                            side_cell(&mut cols[1], row.right.as_ref(), danger, success);
                        }
                    });
                }
            });
    }

    fn on_data_received(&mut self, _payload: &str) {}

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "mode": mode_to_settings(self.mode),
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.mode = mode_from_settings(value.get("mode").and_then(|v| v.as_str()));
    }
}

fn mode_to_settings(mode: DiffMode) -> &'static str {
    match mode {
        DiffMode::SideBySide => "side_by_side",
        DiffMode::Inline => "inline",
    }
}

fn mode_from_settings(value: Option<&str>) -> DiffMode {
    match value {
        Some("side_by_side") => DiffMode::SideBySide,
        Some("inline") => DiffMode::Inline,
        _ => DEFAULT_MODE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn persistable_options_are_mode_only() {
        let mut view = TextCompareView::new();
        view.mode = DiffMode::Inline;
        view.left = "abc".into();
        view.right = "abd".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["mode"], "inline");
        assert!(value.get("left").is_none());
        assert!(value.get("right").is_none());
        assert!(!value.to_string().contains("abc"));
    }

    #[test]
    fn restore_inline_then_diff_uses_inline_hunks() {
        let mut view = TextCompareView::new();
        view.restore_options(&serde_json::json!({ "mode": "inline" }));
        assert_eq!(view.mode, DiffMode::Inline);
        view.left = "a\nb".into();
        view.right = "a\nc".into();
        let hunks = diff_lines(&view.left, &view.right);
        assert!(hunks
            .iter()
            .any(|h| h.tag == DiffTag::Delete && h.text.contains('b')));
        assert!(hunks
            .iter()
            .any(|h| h.tag == DiffTag::Insert && h.text.contains('c')));
    }

    #[test]
    fn missing_and_illegal_mode_use_side_by_side() {
        let mut view = TextCompareView::new();
        view.mode = DiffMode::Inline;
        view.restore_options(&serde_json::json!({ "mode": "unified" }));
        assert_eq!(view.mode, DiffMode::SideBySide);
        view.restore_options(&serde_json::json!({}));
        assert_eq!(view.mode, DiffMode::SideBySide);
        view.left = "a\nb".into();
        view.right = "a\nc".into();
        let rows = diff_rows(&view.left, &view.right);
        assert!(rows
            .iter()
            .any(|r| r.left.as_ref().is_some_and(|l| l.tag == DiffTag::Delete)));
        assert!(rows
            .iter()
            .any(|r| r.right.as_ref().is_some_and(|l| l.tag == DiffTag::Insert)));
    }

    #[test]
    fn test_text_compare_i18n_keys() {
        let keys = [
            ("text_compare.original", "原始内容"),
            ("text_compare.modified", "修改后内容"),
            ("text_compare.side_by_side", "并排"),
            ("text_compare.inline", "行内"),
        ];
        for (key, zh) in keys {
            assert_eq!(devtoys_api::t(key), zh);
        }
    }
}
