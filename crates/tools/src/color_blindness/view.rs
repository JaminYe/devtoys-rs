use crate::slot::ToolView;
use crate::ui;

use super::simulate_color_blindness;

pub struct ColorBlindnessView {
    path: String,
    error: Option<String>,
    original: Option<Vec<u8>>,
    protanopia: Option<Vec<u8>>,
    deuteranopia: Option<Vec<u8>>,
    tritanopia: Option<Vec<u8>>,
    tex_original: Option<egui::TextureHandle>,
    tex_protanopia: Option<egui::TextureHandle>,
    tex_deuteranopia: Option<egui::TextureHandle>,
    tex_tritanopia: Option<egui::TextureHandle>,
}

impl ColorBlindnessView {
    pub fn new() -> Self {
        Self {
            path: String::new(),
            error: None,
            original: None,
            protanopia: None,
            deuteranopia: None,
            tritanopia: None,
            tex_original: None,
            tex_protanopia: None,
            tex_deuteranopia: None,
            tex_tritanopia: None,
        }
    }

    fn load_path(&mut self, payload: &str) {
        self.path = payload.trim().to_string();
        self.resimulate();
    }

    fn resimulate(&mut self) {
        let path = self.path.trim().to_string();
        if path.is_empty() {
            self.error = None;
            self.clear_images();
            return;
        }

        match std::fs::read(&path) {
            Ok(bytes) => match simulate_color_blindness(&bytes) {
                Ok(images) => {
                    self.error = None;
                    self.original = Some(images.original);
                    self.protanopia = Some(images.protanopia);
                    self.deuteranopia = Some(images.deuteranopia);
                    self.tritanopia = Some(images.tritanopia);
                    self.clear_textures();
                }
                Err(_) => {
                    self.error = Some("无法解码图像".into());
                    self.clear_images();
                }
            },
            Err(_) => {
                self.error = Some("无法读取图像".into());
                self.clear_images();
            }
        }
    }

    fn clear_images(&mut self) {
        self.original = None;
        self.protanopia = None;
        self.deuteranopia = None;
        self.tritanopia = None;
        self.clear_textures();
    }

    fn clear_textures(&mut self) {
        self.tex_original = None;
        self.tex_protanopia = None;
        self.tex_deuteranopia = None;
        self.tex_tritanopia = None;
    }
}

impl ToolView for ColorBlindnessView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("图像路径");
        if ui::singleline(ui, "cb-path", &mut self.path, "图像文件路径") {
            self.resimulate();
        }
        ui::error_label(ui, self.error.as_deref());

        let spacing = 12.0;
        let total = ui.available_size();
        let h = ((total.y - spacing) / 2.0).max(80.0);

        ui.allocate_ui(egui::vec2(total.x, h), |ui| {
            ui::split_2(
                ui,
                |ui| {
                    pane(
                        ui,
                        "原图",
                        "cb-original",
                        self.original.as_deref(),
                        &mut self.tex_original,
                    );
                },
                |ui| {
                    pane(
                        ui,
                        "红色盲（Protanopia）",
                        "cb-protanopia",
                        self.protanopia.as_deref(),
                        &mut self.tex_protanopia,
                    );
                },
            );
        });
        ui.add_space(spacing);
        ui.allocate_ui(egui::vec2(total.x, h), |ui| {
            ui::split_2(
                ui,
                |ui| {
                    pane(
                        ui,
                        "绿色盲（Deuteranopia）",
                        "cb-deuteranopia",
                        self.deuteranopia.as_deref(),
                        &mut self.tex_deuteranopia,
                    );
                },
                |ui| {
                    pane(
                        ui,
                        "黄蓝色盲（Tritanopia）",
                        "cb-tritanopia",
                        self.tritanopia.as_deref(),
                        &mut self.tex_tritanopia,
                    );
                },
            );
        });
    }

    fn on_data_received(&mut self, payload: &str) {
        let path = payload
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("");
        if path.is_empty() {
            return;
        }
        self.load_path(path);
    }
}

fn pane(
    ui: &mut egui::Ui,
    title: &str,
    name: &'static str,
    png: Option<&[u8]>,
    tex: &mut Option<egui::TextureHandle>,
) {
    ui.vertical(|ui| {
        ui.label(title);
        let Some(bytes) = png else {
            return;
        };
        if tex.is_none() {
            if let Some(color) = ui::png_image(bytes) {
                *tex = Some(ui.ctx().load_texture(name, color, Default::default()));
            }
        }
        if let Some(handle) = tex.as_ref() {
            ui.add(egui::Image::new(handle).max_size(ui.available_size()));
        }
    });
}
