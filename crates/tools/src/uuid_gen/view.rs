use crate::slot::ToolView;
use crate::ui;

use super::{generate_uuid, UuidOptions, UuidVersion};

pub struct UuidGenView {
    count: String,
    output: String,
    version: UuidVersion,
    hyphens: bool,
    uppercase: bool,
    error: Option<String>,
}

impl UuidGenView {
    pub fn new() -> Self {
        let mut this = Self {
            count: "1".into(),
            output: String::new(),
            version: UuidVersion::Four,
            hyphens: true,
            uppercase: false,
            error: None,
        };
        this.regenerate();
        this
    }

    fn options(&self) -> UuidOptions {
        UuidOptions {
            version: self.version,
            hyphens: self.hyphens,
            uppercase: self.uppercase,
            count: self.count.parse::<usize>().unwrap_or(1).max(1),
        }
    }

    fn regenerate(&mut self) {
        match generate_uuid(&self.options()) {
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

impl ToolView for UuidGenView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("v1", UuidVersion::One),
                ("v4", UuidVersion::Four),
                ("v7", UuidVersion::Seven),
            ] {
                if ui::toggle(ui, self.version == value, label).clicked() {
                    self.version = value;
                    dirty = true;
                }
            }
            dirty |= ui.checkbox(&mut self.hyphens, "连字符").changed();
            dirty |= ui.checkbox(&mut self.uppercase, "大写").changed();
            ui.label("数量");
            ui.allocate_ui(egui::vec2(72.0, ui.spacing().interact_size.y), |ui| {
                dirty |= ui::singleline(ui, "uuid-count", &mut self.count, "数量");
            });
            if ui.button("生成").clicked() {
                dirty = true;
            }
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        if dirty {
            self.regenerate();
        }
        ui::error_label(ui, self.error.as_deref());
        ui::labeled_code(ui, "输出", "uuid-out", &mut self.output, "生成结果", false);
    }

    fn on_data_received(&mut self, _: &str) {}
}
