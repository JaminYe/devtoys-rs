use crate::slot::ToolView;
use crate::ui;

use super::{
    parse_cron, DEFAULT_DATE_FORMAT, DEFAULT_EXPR_WITHOUT_SECONDS, DEFAULT_EXPR_WITH_SECONDS,
};

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
                .checkbox(&mut self.include_seconds, "包含秒字段")
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
        ui::error_label(ui, self.error.as_deref());
        ui.label("表达式");
        if ui::singleline(
            ui,
            "cron-expr",
            &mut self.expression,
            DEFAULT_EXPR_WITH_SECONDS,
        ) {
            self.reparse();
        }
        ui.label("输出");
        ui::fill_code(ui, "cron-out", &mut self.output, "解析结果", false);
    }

    fn on_data_received(&mut self, _payload: &str) {}
}
