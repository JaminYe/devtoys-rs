use crate::slot::ToolView;
use crate::ui;

use super::helper::{format_reports, looks_like_xsd, validate_xml_xsd, XmlReportLevel};

pub struct XmlXsdView {
    xsd: String,
    xml: String,
    output: String,
    error: Option<String>,
}

impl XmlXsdView {
    pub fn new() -> Self {
        Self {
            xsd: String::new(),
            xml: String::new(),
            output: String::new(),
            error: None,
        }
    }

    fn revalidate(&mut self) {
        if self.xml.trim().is_empty() {
            self.error = None;
            self.output.clear();
            return;
        }
        let reports = validate_xml_xsd(&self.xml, &self.xsd);
        self.output = format_reports(&reports);
        self.error = if reports.iter().any(|r| r.level == XmlReportLevel::Error) {
            Some("校验失败".into())
        } else {
            None
        };
    }
}

impl ToolView for XmlXsdView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui::primary_button(ui, "复制").clicked() {
                ui::copy_text(ui, &self.output);
            }
        });
        ui::error_label(ui, self.error.as_deref());
        let avail = ui.available_size();
        let bottom = (avail.y * 0.28).clamp(80.0, 180.0);
        let mut xsd_changed = false;
        let mut xml_changed = false;
        ui.allocate_ui(egui::vec2(avail.x, (avail.y - bottom).max(80.0)), |ui| {
            ui::split_2(
                ui,
                |ui| {
                    xsd_changed =
                        ui::labeled_code(ui, "XSD", "xml-xsd-xsd", &mut self.xsd, "粘贴 XSD", true);
                },
                |ui| {
                    xml_changed =
                        ui::labeled_code(ui, "XML", "xml-xsd-xml", &mut self.xml, "粘贴 XML", true);
                },
            );
        });
        if xsd_changed || xml_changed {
            self.revalidate();
        }
        ui::labeled_code(
            ui,
            "校验结果",
            "xml-xsd-out",
            &mut self.output,
            "校验结果",
            false,
        );
    }

    fn on_data_received(&mut self, payload: &str) {
        if looks_like_xsd(payload) {
            self.xsd = payload.to_string();
        } else {
            self.xml = payload.to_string();
        }
        self.revalidate();
    }
}
