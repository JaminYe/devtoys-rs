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

const COPY_FEEDBACK_DURATION: std::time::Duration = std::time::Duration::from_millis(1500);

#[derive(Clone, Copy, Debug, Default)]
struct CopyFeedbackState {
    button_id: Option<egui::Id>,
    copied_at: Option<std::time::Instant>,
}

#[derive(Clone, Copy, Debug, Default)]
struct ClickedButtonState(Option<egui::Id>);

fn copy_feedback_key() -> egui::Id {
    egui::Id::new("devtoys_ui_copy_feedback")
}

fn last_clicked_button_key() -> egui::Id {
    egui::Id::new("devtoys_ui_last_clicked_copy_button")
}

pub fn primary_button(ui: &mut Ui, text: &str) -> egui::Response {
    let is_copy_btn = text == "复制" || text.starts_with("复制");
    let button_id = ui.make_persistent_id(text);
    let is_copied = if is_copy_btn {
        ui.ctx()
            .data(|d| d.get_temp::<CopyFeedbackState>(copy_feedback_key()))
            .map(|s| s.button_id == Some(button_id) && s.copied_at.map_or(false, |t| t.elapsed() < COPY_FEEDBACK_DURATION))
            .unwrap_or(false)
    } else {
        false
    };

    let response = ui
        .push_id(text, |ui| {
            let (label, fill) = if is_copied {
                ("已复制 ✓", success(ui))
            } else {
                (text, ui.visuals().selection.stroke.color)
            };
            ui.add(egui::Button::new(egui::RichText::new(label).color(Color32::WHITE)).fill(fill))
        })
        .inner;

    if is_copy_btn && response.clicked() {
        ui.ctx().data_mut(|d| {
            d.insert_temp(last_clicked_button_key(), ClickedButtonState(Some(button_id)));
        });
    }
    response
}

pub fn error_label(ui: &mut Ui, error: Option<&str>) {
    if let Some(msg) = error {
        ui.colored_label(danger(ui), msg);
    }
}

pub fn copy_text(ui: &Ui, text: &str) {
    if !text.is_empty() {
        ui.ctx().copy_text(text.to_string());
        let clicked = ui
            .ctx()
            .data(|d| d.get_temp::<ClickedButtonState>(last_clicked_button_key()))
            .and_then(|c| c.0);
        if let Some(button_id) = clicked {
            let state = CopyFeedbackState {
                button_id: Some(button_id),
                copied_at: Some(std::time::Instant::now()),
            };
            ui.ctx().data_mut(|d| {
                d.insert_temp(copy_feedback_key(), state);
                d.remove_temp::<ClickedButtonState>(last_clicked_button_key());
            });
            ui.ctx().request_repaint_after(COPY_FEEDBACK_DURATION);
            ui.ctx().request_repaint();
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_feedback_lifecycle() {
        let ctx = egui::Context::default();

        // Frame 1: Render button initially
        // Frame 1: Render button initially
        let mut out1 = ctx.run_ui(Default::default(), |ui| {
            let resp = primary_button(ui, "复制");
            assert!(resp.rect.is_positive());

            // Copying empty string should not trigger feedback
            copy_text(ui, "");
            let state = ui.ctx().data(|d| d.get_temp::<CopyFeedbackState>(copy_feedback_key()));
            assert!(state.is_none());
        });
        out1.textures_delta.clear();

        // Frame 2: Simulate clicking the copy button and copying non-empty content
        let mut out2 = ctx.run_ui(Default::default(), |ui| {
            let button_id = ui.make_persistent_id("复制");
            ui.ctx().data_mut(|d| {
                d.insert_temp(last_clicked_button_key(), ClickedButtonState(Some(button_id)));
            });
            copy_text(ui, "hello devtoys");
            let state = ui.ctx().data(|d| d.get_temp::<CopyFeedbackState>(copy_feedback_key()));
            assert!(state.is_some());
        });
        out2.textures_delta.clear();

        // Frame 3: Next frame should show active feedback
        let mut out3 = ctx.run_ui(Default::default(), |ui| {
            let button_id = ui.make_persistent_id("复制");
            let state = ui.ctx().data(|d| d.get_temp::<CopyFeedbackState>(copy_feedback_key()));
            assert!(state.is_some());
            let is_copied = state
                .map(|s| s.button_id == Some(button_id) && s.copied_at.map_or(false, |t| t.elapsed() < COPY_FEEDBACK_DURATION))
                .unwrap_or(false);
            assert!(is_copied, "button should show active copy feedback");
        });
        out3.textures_delta.clear();
    }
}
