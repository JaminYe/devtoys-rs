//! Shared egui helpers for tool views.

use std::sync::LazyLock;

use egui::{Color32, Ui, Vec2};
pub fn t(key: &str) -> &str {
    devtoys_api::t(key)
}

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
        (t("common.copied"), success(ui))
    } else {
        (t("common.copy"), ui.visuals().selection.stroke.color)
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

static SYNTAX_SET: LazyLock<syntect::parsing::SyntaxSet> =
    LazyLock::new(syntect::parsing::SyntaxSet::load_defaults_newlines);

static DARK_THEME: LazyLock<syntect::highlighting::Theme> = LazyLock::new(|| {
    let mut themes = syntect::highlighting::ThemeSet::load_defaults();
    themes
        .themes
        .remove("base16-ocean.dark")
        .or_else(|| themes.themes.remove("Solarized (dark)"))
        .unwrap_or_else(|| {
            themes
                .themes
                .into_values()
                .next()
                .expect("syntect default theme")
        })
});

static LIGHT_THEME: LazyLock<syntect::highlighting::Theme> = LazyLock::new(|| {
    let mut themes = syntect::highlighting::ThemeSet::load_defaults();
    themes
        .themes
        .remove("InspiredGitHub")
        .or_else(|| themes.themes.remove("Solarized (light)"))
        .unwrap_or_else(|| {
            themes
                .themes
                .into_values()
                .next()
                .expect("syntect default theme")
        })
});

fn syntax_set() -> &'static syntect::parsing::SyntaxSet {
    &SYNTAX_SET
}

fn dark_theme() -> &'static syntect::highlighting::Theme {
    &DARK_THEME
}

fn light_theme() -> &'static syntect::highlighting::Theme {
    &LIGHT_THEME
}

fn syntax_for(lang: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
    let token = lang.split_whitespace().next().unwrap_or(lang);
    let ss = syntax_set();
    ss.find_syntax_by_token(token)
        .or_else(|| ss.find_syntax_by_token(&token.to_ascii_lowercase()))
        .or_else(|| ss.find_syntax_by_extension(token))
        .or_else(|| ss.find_syntax_by_extension(&token.to_ascii_lowercase()))
        .or_else(|| ss.find_syntax_by_name(token))
        .or_else(|| {
            if token.eq_ignore_ascii_case("sql") || token.to_ascii_lowercase().contains("sql") {
                ss.find_syntax_by_token("sql")
            } else {
                None
            }
        })
}

/// 构建带有语法高亮的 [`egui::text::LayoutJob`]。
///
/// - `language`: 语言标识（如 `"json"`, `"xml"`, `"sql"`）。为 `None` 或不支持的语言时回退为默认等宽文本。
/// - 自动根据当前 `ui.visuals().dark_mode` 选取深色/浅色高亮主题，确保对比度良好。
/// - 语法不完整或解析异常时平滑降级，保证文本完全展示且不崩溃。
pub fn highlight_layout_job(
    ui: &Ui,
    src: &str,
    language: Option<&str>,
    wrap_width: f32,
) -> egui::text::LayoutJob {
    let font_id = egui::TextStyle::Monospace.resolve(ui.style());
    let default_color = ui.visuals().text_color();
    let default_format = egui::text::TextFormat {
        font_id: font_id.clone(),
        color: default_color,
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;

    if src.is_empty() {
        return job;
    }

    let Some(lang) = language.map(str::trim).filter(|s| !s.is_empty()) else {
        job.append(src, 0.0, default_format);
        return job;
    };

    let Some(syntax) = syntax_for(lang) else {
        job.append(src, 0.0, default_format);
        return job;
    };

    let theme = if ui.visuals().dark_mode {
        dark_theme()
    } else {
        light_theme()
    };

    let mut highlighter = syntect::easy::HighlightLines::new(syntax, theme);
    let ss = syntax_set();
    for line in syntect::util::LinesWithEndings::from(src) {
        match highlighter.highlight_line(line, ss) {
            Ok(ranges) => {
                for (style, slice) in ranges {
                    if slice.is_empty() {
                        continue;
                    }
                    let color = Color32::from_rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    );
                    job.append(
                        slice,
                        0.0,
                        egui::text::TextFormat {
                            font_id: font_id.clone(),
                            color,
                            ..Default::default()
                        },
                    );
                }
            }
            Err(_) => {
                job.append(line, 0.0, default_format.clone());
            }
        }
    }

    if job.text.is_empty() && !src.is_empty() {
        job.append(src, 0.0, default_format);
    }

    job
}

/// 多行代码编辑器：支持语法着色、只读/可编辑切换及占位提示。
pub fn code_editor(
    ui: &mut Ui,
    id: &str,
    text: &mut String,
    hint: &str,
    editable: bool,
    language: Option<&str>,
) -> bool {
    let size = ui.available_size().max(Vec2::new(80.0, 80.0));
    let edit = egui::TextEdit::multiline(text)
        .id_salt(id)
        .hint_text(hint)
        .font(egui::TextStyle::Monospace)
        .interactive(editable);

    if let Some(lang) = language {
        let mut layouter =
            |ui: &egui::Ui, buf: &dyn egui::text_edit::TextBuffer, wrap_width: f32| {
                let job = highlight_layout_job(ui, buf.as_str(), Some(lang), wrap_width);
                ui.fonts_mut(|f| f.layout_job(job))
            };
        ui.add_sized(size, edit.layouter(&mut layouter)).changed()
    } else {
        ui.add_sized(size, edit).changed()
    }
}

pub fn fill_code(ui: &mut Ui, id: &str, text: &mut String, hint: &str, editable: bool) -> bool {
    code_editor(ui, id, text, hint, editable, None)
}

pub fn labeled_code_editor(
    ui: &mut Ui,
    label: &str,
    id: &str,
    text: &mut String,
    hint: &str,
    editable: bool,
    language: Option<&str>,
) -> bool {
    ui.vertical(|ui| {
        ui.label(label);
        code_editor(ui, id, text, hint, editable, language)
    })
    .inner
}

pub fn labeled_code(
    ui: &mut Ui,
    label: &str,
    id: &str,
    text: &mut String,
    hint: &str,
    editable: bool,
) -> bool {
    labeled_code_editor(ui, label, id, text, hint, editable, None)
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

    #[test]
    fn test_highlight_layout_job_json_xml_sql_and_invalid_inputs() {
        let ctx = egui::Context::default();

        for dark_mode in [true, false] {
            let mut visuals = if dark_mode {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            };
            ctx.set_visuals(visuals);

            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                // 1. JSON 有效与无效
                let json_valid = r#"{"name": "DevToys", "count": 42, "active": true}"#;
                let job = highlight_layout_job(ui, json_valid, Some("json"), 500.0);
                assert_eq!(job.text, json_valid);
                let colors: std::collections::HashSet<_> =
                    job.sections.iter().map(|s| s.format.color).collect();
                assert!(colors.len() > 1, "dark={dark_mode}: JSON 应有多种高亮颜色");

                let json_invalid = r#"{"name": "DevToys", incomplete"#;
                let job_inv = highlight_layout_job(ui, json_invalid, Some("json"), 500.0);
                assert_eq!(job_inv.text, json_invalid, "无效输入内容必须完整保留");

                // 2. XML 有效与无效
                let xml_valid = r#"<config version="1.0"><item key="test">hello</item></config>"#;
                let job = highlight_layout_job(ui, xml_valid, Some("xml"), 500.0);
                assert_eq!(job.text, xml_valid);
                let colors: std::collections::HashSet<_> =
                    job.sections.iter().map(|s| s.format.color).collect();
                assert!(colors.len() > 1, "dark={dark_mode}: XML 应有多种高亮颜色");

                let xml_invalid = r#"<config <broken unclosed"#;
                let job_inv = highlight_layout_job(ui, xml_invalid, Some("xml"), 500.0);
                assert_eq!(job_inv.text, xml_invalid, "无效 XML 内容必须完整保留");

                // 3. SQL 有效与无效
                let sql_valid =
                    "SELECT id, name, created_at FROM users WHERE active = 1 ORDER BY id DESC;";
                let job = highlight_layout_job(ui, sql_valid, Some("sql"), 500.0);
                assert_eq!(job.text, sql_valid);
                let colors: std::collections::HashSet<_> =
                    job.sections.iter().map(|s| s.format.color).collect();
                assert!(colors.len() > 1, "dark={dark_mode}: SQL 应有多种高亮颜色");

                let sql_invalid = "SELEC * FORM unclosed 'string";
                let job_inv = highlight_layout_job(ui, sql_invalid, Some("sql"), 500.0);
                assert_eq!(job_inv.text, sql_invalid, "无效 SQL 内容必须完整保留");

                // 4. 空文本
                let job_empty = highlight_layout_job(ui, "", Some("json"), 500.0);
                assert_eq!(job_empty.text, "");

                // 5. None / 未知语言：平滑降级为普通文本
                let plain_job = highlight_layout_job(ui, "plain text content", None, 500.0);
                assert_eq!(plain_job.text, "plain text content");
                assert_eq!(plain_job.sections.len(), 1);

                let unknown_job =
                    highlight_layout_job(ui, "some code", Some("unknown-lang-1234"), 500.0);
                assert_eq!(unknown_job.text, "some code");
                assert_eq!(unknown_job.sections.len(), 1);
            });
            out.textures_delta.clear();
        }
    }

    #[test]
    fn test_code_editor_in_ui() {
        let ctx = egui::Context::default();
        let mut input = String::from(r#"{"msg": "hi"}"#);
        let mut output = String::from(r#"{\n  "msg": "hi"\n}"#);

        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let changed = code_editor(ui, "json-edit", &mut input, "hint", true, Some("json"));
            assert!(!changed);
            let changed_out = code_editor(ui, "json-out", &mut output, "hint", false, Some("json"));
            assert!(!changed_out);

            // 纯文本工具使用 labeled_code 不应崩溃
            let mut plain = String::from("plain text");
            labeled_code(ui, "标签", "plain-id", &mut plain, "hint", true);
        });
        out.textures_delta.clear();
    }

    #[test]
    fn test_copy_button_localization() {
        let ctx = egui::Context::default();

        // 1. Default (ZhCn): button label is "复制"
        let (out_zh, resp_zh, _) = render_frame(&ctx, 0.0, Some("hello"), vec![]);
        let has_zh = out_zh.shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(t) => t.galley.text() == "复制",
            _ => false,
        });
        assert!(has_zh, "ZhCn copy button should display '复制'");

        // Click in ZhCn: feedback label is "已复制 ✓"
        let (_, _, feedback_zh) = render_frame(
            &ctx,
            0.1,
            Some("hello"),
            click_events(resp_zh.rect.center()),
        );
        assert!(feedback_zh);
        let (out_zh_copied, _, _) = render_frame(&ctx, 0.15, Some("hello"), vec![]);
        let has_zh_copied = out_zh_copied.shapes.iter().any(|s| match &s.shape {
            egui::Shape::Text(t) => t.galley.text() == "已复制 ✓",
            _ => false,
        });
        assert!(has_zh_copied, "ZhCn feedback should display '已复制 ✓'");
    }
}
