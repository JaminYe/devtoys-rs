use crate::slot::ToolView;
use crate::ui;

use super::decode_certificate;

pub struct CertificateView {
    input: String,
    password: String,
    output: String,
    error: Option<String>,
}

impl CertificateView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            password: String::new(),
            output: String::new(),
            error: None,
        }
    }

    fn redecode(&mut self) {
        if self.input.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        let password = if self.password.is_empty() {
            None
        } else {
            Some(self.password.as_str())
        };
        match decode_certificate(self.input.as_bytes(), password) {
            Ok(result) => {
                self.error = None;
                self.output = result;
            }
            Err(err) => {
                self.error = Some(err.to_string());
                self.output.clear();
            }
        }
    }
}

impl ToolView for CertificateView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.password)
                        .id_salt("cert-pass")
                        .password(true)
                        .hint_text("PFX 密码（可选）")
                        .desired_width(220.0),
                )
                .changed()
            {
                self.redecode();
            }
            if ui::primary_button(ui, "复制").clicked() && self.error.is_none() {
                ui::copy_text(ui, &self.output);
            }
        });
        ui::error_label(ui, self.error.as_deref());
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                input_changed = ui::labeled_code(
                    ui,
                    "输入",
                    "cert-in",
                    &mut self.input,
                    "粘贴 PEM / CER / CRT",
                    true,
                );
            },
            |ui| {
                ui::labeled_code(ui, "输出", "cert-out", &mut self.output, "证书信息", false);
            },
        );
        if input_changed {
            self.redecode();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.input = payload.to_string();
        self.redecode();
    }
}
