use crate::slot::ToolView;
use crate::ui;

use super::{
    parse_cron, DEFAULT_DATE_FORMAT, DEFAULT_EXPR_WITHOUT_SECONDS, DEFAULT_EXPR_WITH_SECONDS, ID,
};

/// Settings JSON: `include_seconds` (bool), `count` (5|10|25|50|100), `date_format` (string).
/// Unknown / missing → include_seconds=true, count=5, date_format=`DEFAULT_DATE_FORMAT`.
const DEFAULT_INCLUDE_SECONDS: bool = true;
const DEFAULT_COUNT: usize = 5;
const ALLOWED_COUNTS: [usize; 5] = [5, 10, 25, 50, 100];

pub struct CronParserView {
    expression: String,
    date_format: String,
    output: String,
    include_seconds: bool,
    count: usize,
    error: Option<String>,
}

impl CronParserView {
    pub fn new() -> Self {
        let mut view = Self {
            expression: DEFAULT_EXPR_WITH_SECONDS.to_string(),
            date_format: DEFAULT_DATE_FORMAT.to_string(),
            output: String::new(),
            include_seconds: true,
            count: 5,
            error: None,
        };
        view.reparse();
        view
    }

    fn reparse(&mut self) {
        if self.expression.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        match parse_cron(
            &self.expression,
            self.include_seconds,
            self.count,
            &self.date_format,
        ) {
            Ok(result) => {
                self.error = None;
                self.output = result.display_text();
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

impl ToolView for CronParserView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .checkbox(&mut self.include_seconds, ui::t(ui, "cron.include_seconds"))
                .changed()
            {
                if self.include_seconds {
                    if self.expression.trim() == DEFAULT_EXPR_WITHOUT_SECONDS {
                        self.expression = DEFAULT_EXPR_WITH_SECONDS.to_string();
                    }
                } else if self.expression.trim() == DEFAULT_EXPR_WITH_SECONDS {
                    self.expression = DEFAULT_EXPR_WITHOUT_SECONDS.to_string();
                }
                self.reparse();
            }
            ui.label("预览条数");
            for (label, count) in [("5", 5), ("10", 10), ("25", 25), ("50", 50), ("100", 100)] {
                if ui::toggle(ui, self.count == count, label).clicked() {
                    self.count = count;
                    self.reparse();
                }
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        ui.horizontal(|ui| {
            ui.label("日期格式");
            if ui::singleline(
                ui,
                "cron-date-fmt",
                &mut self.date_format,
                DEFAULT_DATE_FORMAT,
            ) {
                self.reparse();
            }
        });
        let err_text = self.error.as_deref().map(|e| {
            if e == "非法 Cron 表达式" {
                ui::t(ui, "cron.invalid_expression")
            } else {
                e
            }
        });
        ui::error_label(ui, err_text);
        ui.label(ui::t(ui, "cron.expression"));
        if ui::singleline(
            ui,
            "cron-expr",
            &mut self.expression,
            DEFAULT_EXPR_WITH_SECONDS,
        ) {
            self.reparse();
        }
        ui.label(ui::t(ui, "common.output"));
        ui::fill_code(ui, "cron-out", &mut self.output, "解析结果", false);
    }

    fn on_data_received(&mut self, _payload: &str) {}

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "include_seconds": self.include_seconds,
                "count": self.count,
                "date_format": self.date_format,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.include_seconds = value
            .get("include_seconds")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_INCLUDE_SECONDS);
        self.count = count_from_settings(value.get("count").and_then(|v| v.as_u64()));
        self.date_format =
            date_format_from_settings(value.get("date_format").and_then(|v| v.as_str()));
        sync_default_expression(&mut self.expression, self.include_seconds);
        self.reparse();
    }
}

fn count_from_settings(value: Option<u64>) -> usize {
    match value {
        Some(n) if ALLOWED_COUNTS.contains(&(n as usize)) => n as usize,
        _ => DEFAULT_COUNT,
    }
}

fn date_format_from_settings(value: Option<&str>) -> String {
    match value {
        Some(fmt) if !fmt.trim().is_empty() => fmt.to_string(),
        _ => DEFAULT_DATE_FORMAT.to_string(),
    }
}

fn sync_default_expression(expression: &mut String, include_seconds: bool) {
    if include_seconds {
        if expression.trim() == DEFAULT_EXPR_WITHOUT_SECONDS {
            *expression = DEFAULT_EXPR_WITH_SECONDS.to_string();
        }
    } else if expression.trim() == DEFAULT_EXPR_WITH_SECONDS {
        *expression = DEFAULT_EXPR_WITHOUT_SECONDS.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn persistable_options_are_non_content_only() {
        let mut view = CronParserView::new();
        view.include_seconds = false;
        view.count = 25;
        view.date_format = "yyyy".into();
        view.expression = "0 0 * * *".into();
        view.output = "secret-cron".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["include_seconds"], false);
        assert_eq!(value["count"], 25);
        assert_eq!(value["date_format"], "yyyy");
        assert!(value.get("expression").is_none());
        assert!(value.get("output").is_none());
        let dumped = value.to_string();
        assert!(!dumped.contains("0 0 * * *"));
        assert!(!dumped.contains("secret-cron"));
    }

    #[test]
    fn restore_count_and_date_format_then_parses() {
        let mut view = CronParserView::new();
        view.restore_options(&serde_json::json!({
            "include_seconds": false,
            "count": 10,
            "date_format": "yyyy"
        }));
        assert!(!view.include_seconds);
        assert_eq!(view.count, 10);
        assert_eq!(view.date_format, "yyyy");
        assert_eq!(view.expression, DEFAULT_EXPR_WITHOUT_SECONDS);
        assert!(view.error.is_none());
        let next = view
            .output
            .split("下次执行：\n")
            .nth(1)
            .expect("preview list");
        let years: Vec<&str> = next.lines().collect();
        assert_eq!(years.len(), 10);
        assert!(years
            .iter()
            .all(|y| y.len() == 4 && y.chars().all(|c| c.is_ascii_digit())));
    }

    #[test]
    fn missing_and_illegal_count_use_defaults() {
        let mut view = CronParserView::new();
        view.include_seconds = false;
        view.count = 100;
        view.date_format = "HH:mm".into();
        view.restore_options(&serde_json::json!({ "count": 7, "date_format": "" }));
        assert!(view.include_seconds);
        assert_eq!(view.count, 5);
        assert_eq!(view.date_format, DEFAULT_DATE_FORMAT);
        assert_eq!(view.expression, DEFAULT_EXPR_WITH_SECONDS);
        view.restore_options(&serde_json::json!({}));
        assert!(view.include_seconds);
        assert_eq!(view.count, 5);
        assert_eq!(view.date_format, DEFAULT_DATE_FORMAT);
    }
}
