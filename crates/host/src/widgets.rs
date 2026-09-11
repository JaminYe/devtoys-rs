use egui::{pos2, vec2, Align, CornerRadius, Layout, Rect, Sense, Stroke, Ui, Vec2};

use crate::theme::{self, Icon, Palette, RADIUS, RADIUS_SMALL};

pub fn nav_row(
    ui: &mut Ui,
    palette: &Palette,
    icon: Icon,
    label: &str,
    active: bool,
    indent: f32,
) -> egui::Response {
    let height = 32.0;
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        if active || hovered {
            let fill = if active {
                palette.accent.gamma_multiply(0.18)
            } else {
                palette.surface_hover
            };
            ui.painter()
                .rect_filled(rect, CornerRadius::same(RADIUS_SMALL + 2), fill);
        }
        let color = if active {
            palette.accent
        } else {
            palette.secondary
        };
        let icon_center = pos2(rect.left() + indent + 14.0, rect.center().y);
        theme::paint_icon(
            ui,
            icon,
            Rect::from_center_size(icon_center, Vec2::splat(22.0)),
            16.0,
            color,
        );
        let galley = ui.fonts_mut(|fonts| {
            fonts.layout(
                label.to_owned(),
                theme::regular(13.5),
                if active {
                    palette.text
                } else {
                    palette.secondary
                },
                (rect.right() - 10.0 - (icon_center.x + 16.0)).max(0.0),
            )
        });
        ui.painter().galley(
            pos2(
                icon_center.x + 16.0,
                rect.center().y - galley.size().y / 2.0,
            ),
            galley,
            color,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn tool_card(
    ui: &mut Ui,
    palette: &Palette,
    icon: Icon,
    name: &str,
    group: &str,
    recommended: bool,
) -> egui::Response {
    let width = ui.available_width();
    let height = 64.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    if ui.is_rect_visible(rect) {
        let hovered = response.hovered();
        let fill = if hovered {
            palette.surface_hover
        } else {
            palette.panel
        };
        let stroke = Stroke::new(
            1.0,
            if hovered {
                palette.accent.gamma_multiply(0.55)
            } else {
                palette.outline
            },
        );
        ui.painter().rect(
            rect,
            CornerRadius::same(RADIUS),
            fill,
            stroke,
            egui::StrokeKind::Inside,
        );
        let badge = Rect::from_min_size(
            pos2(rect.left() + 14.0, rect.center().y - 20.0),
            Vec2::splat(40.0),
        );
        ui.painter().rect_filled(
            badge,
            CornerRadius::same(RADIUS_SMALL + 2),
            palette.accent.gamma_multiply(0.12),
        );
        theme::paint_icon(ui, icon, badge, 18.0, palette.accent);

        let text_x = badge.right() + 12.0;
        let name_galley = ui.fonts_mut(|fonts| {
            fonts.layout(
                name.to_owned(),
                theme::semibold(15.0),
                palette.text,
                rect.right() - 48.0 - text_x,
            )
        });
        ui.painter()
            .galley(pos2(text_x, rect.top() + 14.0), name_galley, palette.text);
        let group_galley = ui.fonts_mut(|fonts| {
            fonts.layout(
                group.to_owned(),
                theme::regular(12.0),
                palette.dim,
                rect.right() - 48.0 - text_x,
            )
        });
        ui.painter()
            .galley(pos2(text_x, rect.top() + 34.0), group_galley, palette.dim);

        if recommended {
            let chip = "推荐";
            let chip_galley = ui.fonts_mut(|fonts| {
                fonts.layout(chip.to_owned(), theme::regular(11.0), palette.accent, 80.0)
            });
            let chip_size = vec2(chip_galley.size().x + 16.0, 22.0);
            let chip_rect = Rect::from_min_size(
                pos2(rect.right() - 36.0 - chip_size.x, rect.center().y - 11.0),
                chip_size,
            );
            ui.painter().rect_filled(
                chip_rect,
                CornerRadius::same(11),
                palette.accent.gamma_multiply(0.14),
            );
            ui.painter().galley(
                pos2(
                    chip_rect.left() + 8.0,
                    chip_rect.center().y - chip_galley.size().y / 2.0,
                ),
                chip_galley,
                palette.accent,
            );
        }

        theme::paint_icon(
            ui,
            Icon::ChevronRight,
            Rect::from_center_size(
                pos2(rect.right() - 20.0, rect.center().y),
                Vec2::splat(18.0),
            ),
            16.0,
            palette.dim,
        );
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn empty_state(ui: &mut Ui, palette: &Palette, title: &str, hint: &str) {
    ui.allocate_ui_with_layout(ui.available_size(), Layout::top_down(Align::Center), |ui| {
        ui.add_space(ui.available_height() * 0.28);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(48.0), Sense::hover());
        ui.painter()
            .circle_filled(rect.center(), 24.0, palette.surface);
        theme::paint_icon(ui, Icon::Inbox, rect, 22.0, palette.dim);
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(title)
                .font(theme::semibold(15.0))
                .color(palette.text),
        );
        ui.label(
            egui::RichText::new(hint)
                .font(theme::regular(13.0))
                .color(palette.dim),
        );
    });
}

pub fn section_card(ui: &mut Ui, palette: &Palette, add: impl FnOnce(&mut Ui)) {
    egui::Frame::new()
        .fill(palette.panel)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(RADIUS)
        .inner_margin(egui::Margin::same(20))
        .show(ui, add);
}

pub fn setting_card_header(ui: &mut Ui, palette: &Palette, icon: Icon, title: &str) {
    ui.horizontal(|ui| {
        let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
        theme::paint_icon(ui, icon, icon_rect, 18.0, palette.secondary);
        ui.label(
            egui::RichText::new(title)
                .font(theme::semibold(15.0))
                .color(palette.text),
        );
    });
    ui.add_space(8.0);
}

pub fn setting_row<R>(
    ui: &mut Ui,
    palette: &Palette,
    title: &str,
    description: Option<&str>,
    add_controls: impl FnOnce(&mut Ui) -> R,
) -> R {
    let available_w = ui.available_width();
    let is_wide = available_w >= 480.0;

    if is_wide {
        let mut result = None;
        ui.horizontal(|ui| {
            // Reserve space for controls on the right (up to 280px)
            let control_max_w = 280.0f32.min(available_w * 0.45);
            let text_w = (available_w - control_max_w - 16.0).max(140.0);

            // 1. Left side: Title and Description in top-down layout
            ui.allocate_ui_with_layout(
                vec2(text_w, ui.available_height()),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.set_width(text_w);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(title)
                                .font(theme::semibold(14.0))
                                .color(palette.text),
                        )
                        .wrap(),
                    );
                    if let Some(desc) = description {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(desc)
                                    .font(theme::regular(13.0))
                                    .color(palette.dim),
                            )
                            .wrap(),
                        );
                    }
                },
            );

            // 2. Right side: right-aligned controls
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                result = Some(add_controls(ui));
            });
        });
        result.expect("controls added")
    } else {
        ui.vertical(|ui| {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(title)
                        .font(theme::semibold(14.0))
                        .color(palette.text),
                )
                .wrap(),
            );
            if let Some(desc) = description {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(desc)
                            .font(theme::regular(13.0))
                            .color(palette.dim),
                    )
                    .wrap(),
                );
            }
            ui.add_space(8.0);
            add_controls(ui)
        })
        .inner
    }
}
