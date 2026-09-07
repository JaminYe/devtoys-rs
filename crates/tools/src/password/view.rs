use crate::slot::ToolView;
use crate::ui;

use super::{generate_password, PasswordOptions};

pub struct PasswordView {
    length: String,
    count: String,
    exclude: String,
    output: String,
    uppercase: bool,
    lowercase: bool,
    digits: bool,
    special: bool,
    error: Option<String>,
}

impl PasswordView {
    pub fn new() -> Self {
        let mut this = Self {
            length: "30".into(),
            count: "1".into(),
            exclude: String::new(),
            output: String::new(),
            uppercase: true,
            lowercase: true,
            digits: true,
            special: true,
            error: None,
        };
        this.regenerate();
        this
    }

    fn options(&self) -> PasswordOptions {
        PasswordOptions {
            length: self.length.parse::<usize>().unwrap_or(30),
            uppercase: self.uppercase,
            lowercase: self.lowercase,
            digits: self.digits,
            special: self.special,
            exclude: self.exclude.clone(),
            count: self.count.parse::<usize>().unwrap_or(1).max(1),
        }
    }

    fn regenerate(&mut self) {
        match generate_password(&self.options(), &mut rand::thread_rng()) {
            Ok(text) => {
                self.error = None;
                self.output = text;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

impl ToolView for PasswordView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        ui.horizontal_wrapped(|ui| {
            dirty |= ui.checkbox(&mut self.uppercase, "大写").changed();
            dirty |= ui.checkbox(&mut self.lowercase, "小写").changed();
            dirty |= ui.checkbox(&mut self.digits, "数字").changed();
            dirty |= ui.checkbox(&mut self.special, "特殊字符").changed();
            ui.label("长度");
            ui.allocate_ui(egui::vec2(72.0, ui.spacing().interact_size.y), |ui| {
                dirty |= ui::singleline(ui, "pwd-len", &mut self.length, "长度");
            });
            ui.label("数量");
            ui.allocate_ui(egui::vec2(72.0, ui.spacing().interact_size.y), |ui| {
                dirty |= ui::singleline(ui, "pwd-count", &mut self.count, "数量");
            });
            if ui.button("生成").clicked() {
                dirty = true;
            }
            if ui::primary_button(ui, "复制").clicked() && self.error.is_none() {
                ui::copy_text(ui, &self.output);
            }
        });
        ui.label("排除字符");
        dirty |= ui::singleline(ui, "pwd-exclude", &mut self.exclude, "排除字符");
        if dirty {
            self.regenerate();
        }
        ui::error_label(ui, self.error.as_deref());
        ui::labeled_code(ui, "输出", "pwd-out", &mut self.output, "生成结果", false);
    }

    fn on_data_received(&mut self, _: &str) {}
}
