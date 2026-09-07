use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use crate::slot::ToolView;
use crate::ui;

use super::markdown_to_html;

pub struct MarkdownPreviewView {
    editor: String,
    html: String,
    preview_dark: bool,
}

impl MarkdownPreviewView {
    pub fn new() -> Self {
        Self {
            editor: String::new(),
            html: String::new(),
            preview_dark: true,
        }
    }

    fn refresh_html(&mut self) {
        self.html = markdown_to_html(&self.editor);
    }
}

fn render_markdown_preview(ui: &mut egui::Ui, markdown: &str) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);

    let mut inlines: Vec<(String, bool)> = Vec::new();
    let mut heading = false;
    let mut bullet = false;
    let mut in_code_block = false;

    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Start(Tag::Heading { .. }) => {
                inlines.clear();
                heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                flush_inlines(ui, &mut inlines, heading, false);
                heading = false;
            }
            Event::Start(Tag::Paragraph) => {
                if !bullet {
                    inlines.clear();
                }
            }
            Event::End(TagEnd::Paragraph) => {
                flush_inlines(ui, &mut inlines, heading, bullet);
                bullet = false;
            }
            Event::Start(Tag::CodeBlock(_)) => {
                inlines.clear();
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                let text: String = inlines.drain(..).map(|(t, _)| t).collect();
                ui.label(egui::RichText::new(text.trim_end()).monospace());
                in_code_block = false;
            }
            Event::Start(Tag::Item) => {
                inlines.clear();
                bullet = true;
            }
            Event::End(TagEnd::Item) => {
                if bullet {
                    flush_inlines(ui, &mut inlines, false, true);
                    bullet = false;
                }
            }
            Event::Code(code) => inlines.push((code.to_string(), true)),
            Event::Text(text) => {
                if in_code_block {
                    inlines.push((text.to_string(), true));
                } else {
                    inlines.push((text.to_string(), false));
                }
            }
            Event::SoftBreak | Event::HardBreak => inlines.push(("\n".into(), false)),
            Event::Rule => {
                ui.separator();
            }
            Event::TaskListMarker(checked) => {
                inlines.push(((if checked { "[x] " } else { "[ ] " }).into(), false));
            }
            _ => {}
        }
    }
    flush_inlines(ui, &mut inlines, heading, bullet);
}

fn flush_inlines(
    ui: &mut egui::Ui,
    inlines: &mut Vec<(String, bool)>,
    heading: bool,
    bullet: bool,
) {
    if inlines.is_empty() {
        return;
    }
    if inlines.iter().all(|(t, _)| t.trim().is_empty()) {
        inlines.clear();
        return;
    }
    ui.horizontal_wrapped(|ui| {
        if bullet {
            ui.label("• ");
        }
        for (text, is_code) in inlines.drain(..) {
            if text.is_empty() {
                continue;
            }
            let mut rt = egui::RichText::new(text);
            if heading {
                rt = rt.strong();
            }
            if is_code {
                rt = rt.monospace();
            }
            ui.label(rt);
        }
    });
}

impl ToolView for MarkdownPreviewView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui::toggle(ui, self.preview_dark, "深色").clicked() {
                self.preview_dark = true;
            }
            if ui::toggle(ui, !self.preview_dark, "浅色").clicked() {
                self.preview_dark = false;
            }
        });
        let spacing = 12.0;
        let total = ui.available_size();
        let w = ((total.x - spacing) / 2.0).max(80.0);
        ui.horizontal(|ui| {
            ui.set_min_height(total.y);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                if ui::labeled_code(ui, "编辑", "md-editor", &mut self.editor, "Markdown", true) {
                    self.refresh_html();
                }
            });
            ui.add_space(spacing);
            let source = self.editor.clone();
            let preview_dark = self.preview_dark;
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                ui.vertical(|ui| {
                    ui.label("预览");
                    let html_reserve = 120.0;
                    let preview_h = (ui.available_height() - html_reserve).max(80.0);
                    ui.allocate_ui(egui::vec2(ui.available_width(), preview_h), |ui| {
                        let fill = if preview_dark {
                            egui::Color32::from_rgb(0x1e, 0x1e, 0x1e)
                        } else {
                            egui::Color32::from_rgb(0xf7, 0xf7, 0xf7)
                        };
                        let text = if preview_dark {
                            egui::Color32::from_rgb(0xe8, 0xe8, 0xe8)
                        } else {
                            egui::Color32::from_rgb(0x22, 0x22, 0x22)
                        };
                        egui::Frame::new()
                            .fill(fill)
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.set_min_size(ui.available_size());
                                ui.style_mut().visuals.override_text_color = Some(text);
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        render_markdown_preview(ui, &source);
                                    });
                            });
                    });
                    ui::labeled_code(ui, "HTML", "md-html", &mut self.html, "HTML", false);
                });
            });
        });
    }

    fn on_data_received(&mut self, payload: &str) {
        self.editor = payload.to_string();
        self.refresh_html();
    }
}
