//! Shared egui helpers for tool views.

use egui::{Color32, Ui, Vec2};

pub fn danger(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(0xf5, 0x71, 0x7f)
    } else {
        Color32::from_rgb(0xd6, 0x3b, 0x4c)
    }
}

pub fn success(ui: &Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgb(0x4a, 0xd1, 0x7a)
    } else {
        Color32::from_rgb(0x16, 0xa3, 0x4a)
    }
}

pub fn toggle(ui: &mut Ui, selected: bool, text: &str) -> egui::Response {
    let mut button = egui::Button::new(text);
    if selected {
        button = button.fill(ui.visuals().selection.bg_fill).selected(true);
    }
    ui.add(button)
}

pub fn primary_button(ui: &mut Ui, text: &str) -> egui::Response {
    let accent = ui.visuals().selection.stroke.color;
    ui.add(egui::Button::new(egui::RichText::new(text).color(Color32::WHITE)).fill(accent))
}

pub fn error_label(ui: &mut Ui, error: Option<&str>) {
    if let Some(msg) = error {
        ui.colored_label(danger(ui), msg);
    }
}

pub fn copy_text(ui: &Ui, text: &str) {
    if !text.is_empty() {
        ui.ctx().copy_text(text.to_string());
    }
}

pub fn singleline(ui: &mut Ui, id: &str, text: &mut String, hint: &str) -> bool {
    ui.add(
        egui::TextEdit::singleline(text)
            .id_salt(id)
            .hint_text(hint)
            .desired_width(f32::INFINITY),
    )
    .changed()
}

pub fn fill_code(ui: &mut Ui, id: &str, text: &mut String, hint: &str, editable: bool) -> bool {
    let size = ui.available_size().max(Vec2::new(80.0, 80.0));
    ui.add_sized(
        size,
        egui::TextEdit::multiline(text)
            .id_salt(id)
            .hint_text(hint)
            .font(egui::TextStyle::Monospace)
            .interactive(editable),
    )
    .changed()
}

pub fn labeled_code(
    ui: &mut Ui,
    label: &str,
    id: &str,
    text: &mut String,
    hint: &str,
    editable: bool,
) -> bool {
    ui.vertical(|ui| {
        ui.label(label);
        fill_code(ui, id, text, hint, editable)
    })
    .inner
}

pub fn split_2(ui: &mut Ui, left: impl FnOnce(&mut Ui), right: impl FnOnce(&mut Ui)) {
    let spacing = 12.0;
    let total = ui.available_size();
    let w = ((total.x - spacing) / 2.0).max(80.0);
    ui.horizontal(|ui| {
        ui.set_min_height(total.y);
        ui.allocate_ui(Vec2::new(w, total.y), left);
        ui.add_space(spacing);
        ui.allocate_ui(Vec2::new(w, total.y), right);
    });
}

pub fn png_image(bytes: &[u8]) -> Option<egui::ColorImage> {
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}
