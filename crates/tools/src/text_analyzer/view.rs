use crate::slot::ToolView;
use crate::ui;

use super::{apply, stats, Operation, TextStats};

pub struct TextAnalyzerView {
    input: String,
    stats: TextStats,
    cursor_line: usize,
    cursor_col: usize,
}

impl TextAnalyzerView {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            stats: stats(""),
            cursor_line: 1,
            cursor_col: 1,
        }
    }

    fn refresh_stats(&mut self) {
        self.stats = stats(&self.input);
    }

    fn apply_op(&mut self, op: Operation) {
        let mut rng = rand::thread_rng();
        self.input = apply(&self.input, &[op], &mut rng);
        self.refresh_stats();
    }
}

impl ToolView for TextAnalyzerView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("换行");
        ui.horizontal_wrapped(|ui| {
            if ui.button("LF").clicked() {
                self.apply_op(Operation::LineEndingsLf);
            }
            if ui.button("CRLF").clicked() {
                self.apply_op(Operation::LineEndingsCrlf);
            }
        });
        ui.label("大小写");
        ui.horizontal_wrapped(|ui| {
            for (label, op) in [
                ("小写", Operation::Lower),
                ("大写", Operation::Upper),
                ("句首", Operation::Sentence),
                ("标题", Operation::Title),
                ("camel", Operation::Camel),
                ("Pascal", Operation::Pascal),
                ("snake", Operation::Snake),
                ("CONSTANT", Operation::Constant),
                ("kebab", Operation::Kebab),
                ("COBOL", Operation::Cobol),
                ("Train", Operation::Train),
                ("交替", Operation::Alternating),
                ("反转", Operation::Inverse),
                ("随机", Operation::RandomCase),
            ] {
                if ui.button(label).clicked() {
                    self.apply_op(op);
                }
            }
        });
        ui.label("行");
        ui.horizontal_wrapped(|ui| {
            for (label, op) in [
                ("字母序", Operation::SortLines),
                ("倒序", Operation::SortLinesDesc),
                ("按末词", Operation::SortByLastWord),
                ("按末词倒序", Operation::SortByLastWordDesc),
                ("反转行", Operation::ReverseLines),
                ("打乱行", Operation::ShuffleLines),
            ] {
                if ui.button(label).clicked() {
                    self.apply_op(op);
                }
            }
            if ui::primary_button(ui, "复制").clicked() {
                ui::copy_text(ui, &self.input);
            }
        });
        let cursor_line = self.cursor_line;
        let cursor_col = self.cursor_col;
        let mut input_changed = false;
        ui::split_2(
            ui,
            |ui| {
                ui.vertical(|ui| {
                    ui.label("输入");
                    input_changed =
                        ui::fill_code(ui, "text-in", &mut self.input, "粘贴或输入文本", true);
                    let id = ui.id().with(egui::Id::new("text-in"));
                    if let Some(state) = egui::TextEdit::load_state(ui.ctx(), id) {
                        if let Some(range) = state.cursor.char_range() {
                            let (line, col) = line_col_at(&self.input, range.primary.index.0);
                            self.cursor_line = line;
                            self.cursor_col = col;
                        }
                    }
                });
            },
            |ui| {
                ui.vertical(|ui| {
                    ui.label("统计");
                    let s = &self.stats;
                    stat_row(ui, "选区行", &format!("{cursor_line}"));
                    stat_row(ui, "选区列", &format!("{cursor_col}"));
                    stat_row(ui, "字节", &format!("{}", s.bytes));
                    stat_row(ui, "字符", &format!("{}", s.chars));
                    stat_row(ui, "词", &format!("{}", s.words));
                    stat_row(ui, "句", &format!("{}", s.sentences));
                    stat_row(ui, "段", &format!("{}", s.paragraphs));
                    stat_row(ui, "行", &format!("{}", s.lines));
                    stat_row(ui, "换行", s.eol.as_str());
                    ui.add_space(8.0);
                    ui.label("词频");
                    for (k, v) in top_word_freq(&s.word_freq) {
                        stat_row(ui, &k, &v);
                    }
                    ui.add_space(8.0);
                    ui.label("字符频");
                    for (k, v) in top_char_freq(&s.char_freq) {
                        stat_row(ui, &k, &v);
                    }
                });
            },
        );
        if input_changed {
            self.refresh_stats();
        }
    }

    fn on_data_received(&mut self, payload: &str) {
        self.input = payload.to_string();
        self.refresh_stats();
    }
}

fn stat_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_sized([72.0, 18.0], egui::Label::new(label));
        ui.label(value);
    });
}

fn line_col_at(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in text.chars().enumerate() {
        if i >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn top_word_freq(map: &std::collections::HashMap<String, usize>) -> Vec<(String, String)> {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    items
        .into_iter()
        .take(12)
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect()
}

fn top_char_freq(map: &std::collections::HashMap<char, usize>) -> Vec<(String, String)> {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    items
        .into_iter()
        .take(12)
        .map(|(k, v)| {
            let label = if *k == ' ' {
                "⎵".to_string()
            } else {
                k.to_string()
            };
            (label, v.to_string())
        })
        .collect()
}
