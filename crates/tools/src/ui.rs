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

pub fn error_label(ui: &mut Ui, error: Option<&str>) {
    if let Some(msg) = error {
        ui.colored_label(danger(ui), msg);
    }
}

#[derive(Clone, Copy, Debug)]
struct CopyButtonState {
    /// egui 帧时间（秒）——可控、可测，不依赖真实挂钟。
    copied_at: f64,
}

/// 复制按钮：动作、身份与反馈内聚于本模块。
///
/// 调用方只声明"复制什么、是否可复制"：`None` 表示当前不可复制（如存在错误），
/// 空内容同样不产生复制与反馈。按钮身份由 egui 按布局位置自动分配，
/// 反馈状态按按钮自身隔离，不存在跨按钮的全局点击标记。
pub fn copy_button(ui: &mut Ui, payload: Option<&str>) -> egui::Response {
    let now = ui.ctx().input(|i| i.time);
    let is_copied = ui
        .ctx()
        .data(|d| d.get_temp::<CopyButtonState>(ui.next_auto_id()))
        .is_some_and(|s| now - s.copied_at < COPY_FEEDBACK_DURATION.as_secs_f64());

    let (label, fill) = if is_copied {
        ("已复制 ✓", success(ui))
    } else {
        ("复制", ui.visuals().selection.stroke.color)
    };
    let response =
        ui.add(egui::Button::new(egui::RichText::new(label).color(Color32::WHITE)).fill(fill));

    if response.clicked() {
        if let Some(text) = payload.filter(|t| !t.is_empty()) {
            ui.ctx().copy_text(text.to_string());
            ui.ctx()
                .data_mut(|d| d.insert_temp(response.id, CopyButtonState { copied_at: now }));
            ui.ctx().request_repaint_after(COPY_FEEDBACK_DURATION);
            ui.ctx().request_repaint();
        }
    }
    response
}

/// 只读查询：该复制按钮当前是否处于"已复制 ✓"反馈期。
pub fn copy_feedback_active(ui: &Ui, response: &egui::Response) -> bool {
    let now = ui.ctx().input(|i| i.time);
    ui.ctx()
        .data(|d| d.get_temp::<CopyButtonState>(response.id))
        .is_some_and(|s| now - s.copied_at < COPY_FEEDBACK_DURATION.as_secs_f64())
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

    // ---- copy_button 深模块测试：真实指针事件，不注入私有状态 ----

    fn copied_text(out: &egui::FullOutput) -> Option<String> {
        out.platform_output.commands.iter().find_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        })
    }

    fn click_events(pos: egui::Pos2) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            },
        ]
    }

    fn render_frame(
        ctx: &egui::Context,
        time: f64,
        payload: Option<&str>,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, egui::Response, bool) {
        let input = egui::RawInput {
            time: Some(time),
            events,
            ..Default::default()
        };
        let mut feedback = false;
        let mut resp: Option<egui::Response> = None;
        let mut out = ctx.run_ui(input, |ui| {
            let r = copy_button(ui, payload);
            feedback = copy_feedback_active(ui, &r);
            resp = Some(r);
        });
        out.textures_delta.clear();
        (out, resp.expect("copy_button response"), feedback)
    }

    #[test]
    fn test_copy_button_real_click_copies_and_feedback_expires() {
        let ctx = egui::Context::default();

        // 帧1：渲染按钮，取得位置
        let (out1, resp1, feedback1) = render_frame(&ctx, 0.0, Some("hello"), vec![]);
        assert!(resp1.rect.is_positive());
        assert!(copied_text(&out1).is_none());
        assert!(!feedback1);

        // 帧2：真实指针事件点击 → 复制发生，反馈生效
        let (out2, _, feedback2) =
            render_frame(&ctx, 0.1, Some("hello"), click_events(resp1.rect.center()));
        assert_eq!(copied_text(&out2).as_deref(), Some("hello"));
        assert!(feedback2);

        // 帧3：反馈期内仍生效
        let (_, _, feedback3) = render_frame(&ctx, 0.5, Some("hello"), vec![]);
        assert!(feedback3);

        // 帧4：超过 1.5s 反馈到期消失
        let (_, _, feedback4) = render_frame(&ctx, 2.0, Some("hello"), vec![]);
        assert!(!feedback4);
    }

    #[test]
    fn test_copy_button_empty_content_no_copy_no_feedback() {
        let ctx = egui::Context::default();
        let (_, resp1, _) = render_frame(&ctx, 0.0, Some(""), vec![]);
        let (out2, _, feedback2) =
            render_frame(&ctx, 0.1, Some(""), click_events(resp1.rect.center()));
        assert!(copied_text(&out2).is_none());
        assert!(!feedback2);
    }

    #[test]
    fn test_copy_button_unavailable_blocks_copy() {
        let ctx = egui::Context::default();
        // payload=None 模拟错误阻断：按钮可点但复制不可用
        let (_, resp1, _) = render_frame(&ctx, 0.0, None, vec![]);
        let (out2, _, feedback2) = render_frame(&ctx, 0.1, None, click_events(resp1.rect.center()));
        assert!(copied_text(&out2).is_none());
        assert!(!feedback2);
        // 无遗留状态：随后仍无反馈
        let (_, _, feedback3) = render_frame(&ctx, 0.5, None, vec![]);
        assert!(!feedback3);
    }

    #[test]
    fn test_copy_button_same_name_siblings_isolated() {
        let ctx = egui::Context::default();

        // 帧1：同层两个复制按钮
        let input1 = egui::RawInput {
            time: Some(0.0),
            ..Default::default()
        };
        let mut rect1 = egui::Rect::NOTHING;
        let mut rect2 = egui::Rect::NOTHING;
        let mut out1 = ctx.run_ui(input1, |ui| {
            rect1 = copy_button(ui, Some("a")).rect;
            rect2 = copy_button(ui, Some("b")).rect;
        });
        out1.textures_delta.clear();

        // 帧2：点击第二个按钮 → 只有它复制并进入反馈期
        let input2 = egui::RawInput {
            time: Some(0.1),
            events: click_events(rect2.center()),
            ..Default::default()
        };
        let mut copied: Option<String> = None;
        let mut feedback_a = false;
        let mut feedback_b = false;
        let mut out2 = ctx.run_ui(input2, |ui| {
            let a = copy_button(ui, Some("a"));
            let b = copy_button(ui, Some("b"));
            feedback_a = copy_feedback_active(ui, &a);
            feedback_b = copy_feedback_active(ui, &b);
        });
        out2.textures_delta.clear();
        copied = copied_text(&out2);
        assert_eq!(copied.as_deref(), Some("b"));
        assert!(!feedback_a, "未点击的按钮不应进入反馈期");
        assert!(feedback_b, "被点击的按钮应进入反馈期");
    }
}
