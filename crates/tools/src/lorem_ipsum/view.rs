use crate::slot::ToolView;
use crate::ui;

use super::{generate_lorem, Corpus, LoremUnit};

pub struct LoremIpsumView {
    length: String,
    output: String,
    corpus: Corpus,
    unit: LoremUnit,
    error: Option<String>,
}

impl LoremIpsumView {
    pub fn new() -> Self {
        let mut this = Self {
            length: "1".into(),
            output: String::new(),
            corpus: Corpus::LoremIpsum,
            unit: LoremUnit::Paragraphs,
            error: None,
        };
        this.regenerate();
        this
    }

    fn length_value(&self) -> usize {
        self.length.parse::<usize>().unwrap_or(1).max(1)
    }

    fn regenerate(&mut self) {
        match generate_lorem(self.corpus, self.unit, self.length_value()) {
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

impl ToolView for LoremIpsumView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let mut dirty = false;
        ui.horizontal_wrapped(|ui| {
            for corpus in Corpus::ALL {
                if ui::toggle(ui, self.corpus == corpus, corpus.as_str()).clicked() {
                    self.corpus = corpus;
                    dirty = true;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("段落", LoremUnit::Paragraphs),
                ("句子", LoremUnit::Sentences),
                ("词", LoremUnit::Words),
                ("字符", LoremUnit::Characters),
            ] {
                if ui::toggle(ui, self.unit == value, label).clicked() {
                    self.unit = value;
                    dirty = true;
                }
            }
            ui.label("长度");
            ui.allocate_ui(egui::vec2(72.0, ui.spacing().interact_size.y), |ui| {
                dirty |= ui::singleline(ui, "lorem-len", &mut self.length, "长度");
            });
            ui::copy_button(ui, self.error.is_none().then_some(self.output.as_str()));
        });
        if dirty {
            self.regenerate();
        }
        ui::error_label(ui, self.error.as_deref());
        ui::labeled_code(ui, "输出", "lorem-out", &mut self.output, "生成结果", false);
    }

    fn on_data_received(&mut self, _: &str) {}
}
