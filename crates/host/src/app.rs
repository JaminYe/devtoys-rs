use std::collections::HashMap;
use std::sync::Arc;

use devtoys_api::{t, GroupId, ThemePreference, ToolMetadata, SETTINGS_ID};
use devtoys_core::{
    AppState, CoreError, DetectionCoordinator, DetectionEngine, Recommendation, SearchOutcome,
    SettingsStore,
};
use devtoys_tools::{ToolCatalog, ToolHandle};
use egui::{vec2, Align, CornerRadius, Frame, Layout, Margin, Stroke, Vec2};

use crate::theme::{self, Icon, Palette, SIDEBAR_WIDTH};
use crate::widgets;

#[derive(Clone, PartialEq, Eq)]
enum Page {
    AllTools,
    Favorites,
    Group(GroupId),
    Tool(String),
    Settings,
}

pub struct Workspace {
    state: AppState,
    coordinator: DetectionCoordinator,
    page: Page,
    search: String,
    sessions: HashMap<String, ToolHandle>,
    recommendations: Vec<Recommendation>,
    palette: Palette,
    applied_dark: Option<bool>,
    settings_error: Option<String>,
    catalog: ToolCatalog,
}

impl Workspace {
    pub fn new() -> Self {
        let catalog = ToolCatalog::default();
        let tools = catalog.all_metadata();
        let mut detectors = devtoys_core::all_detectors();
        detectors.extend(catalog.all_detectors());
        let engine = Arc::new(DetectionEngine::new(detectors, &tools));
        let state = AppState::bootstrap(tools, SettingsStore::user());
        let coordinator = DetectionCoordinator::system(engine);
        Self {
            state,
            coordinator,
            page: Page::AllTools,
            search: String::new(),
            sessions: HashMap::new(),
            recommendations: Vec::new(),
            palette: Palette::dark(),
            applied_dark: None,
            settings_error: None,
            catalog,
        }
    }

    fn system_dark(ctx: &egui::Context) -> bool {
        match ctx.system_theme() {
            Some(egui::Theme::Light) => false,
            Some(egui::Theme::Dark) => true,
            None => true,
        }
    }

    fn desired_dark(&self, ctx: &egui::Context) -> bool {
        match self.state.settings().theme {
            ThemePreference::Light => false,
            ThemePreference::Dark => true,
            ThemePreference::System => Self::system_dark(ctx),
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let dark = self.desired_dark(ctx);
        if self.applied_dark == Some(dark) {
            return;
        }
        self.palette = if dark {
            Palette::dark()
        } else {
            Palette::light()
        };
        theme::apply(ctx, &self.palette);
        self.applied_dark = Some(dark);
    }

    fn tick_detect(&mut self, ctx: &egui::Context) {
        let enabled = self.state.settings().smart_detection_enabled;
        let active = match &self.page {
            Page::Tool(id) => Some(id.as_str()),
            _ => None,
        };
        if let Some(hits) = self.coordinator.poll(active, enabled) {
            if self.recommendations != hits {
                self.recommendations = hits.to_vec();
                ctx.request_repaint();
            }
        } else if !self.recommendations.is_empty() {
            self.recommendations.clear();
            ctx.request_repaint();
        }
        if let Some(delay) = self.coordinator.next_poll_after() {
            ctx.request_repaint_after(delay);
        }
    }

    fn tool_by_id(&self, id: &str) -> Option<&ToolMetadata> {
        self.state.registry().get(id)
    }

    fn is_recommended(&self, id: &str) -> bool {
        self.recommendations.iter().any(|item| item.tool_id == id)
    }

    fn recommended_hit(&self, id: &str) -> Option<Recommendation> {
        self.recommendations
            .iter()
            .find(|item| item.tool_id == id)
            .cloned()
    }

    fn page_is_tool(&self, id: &str) -> bool {
        matches!(&self.page, Page::Tool(open) if open == id)
    }

    fn page_is_group(&self, group: GroupId) -> bool {
        match &self.page {
            Page::Group(open) => *open == group,
            Page::Tool(id) => self.tool_by_id(id).is_some_and(|tool| tool.group == group),
            _ => false,
        }
    }

    fn go(&mut self, page: Page) {
        self.page = page;
    }

    fn open_tool(&mut self, id: &str, hit: Option<Recommendation>) {
        if id == SETTINGS_ID {
            self.go(Page::Settings);
            return;
        }

        let _ = self.state.open_tool(id);
        self.page = Page::Tool(id.to_string());

        self.ensure_session(id);

        if let Some(hit) = hit {
            if self.state.settings().paste_enabled() {
                if let Some(handle) = self.sessions.get_mut(id) {
                    let clipboard = self.coordinator.last_clipboard();
                    handle.on_detected_data(&hit.paste_payload(clipboard));
                }
            }
        }
    }

    fn apply_setting(&mut self, result: Result<(), CoreError>) {
        match result {
            Ok(()) => self.settings_error = None,
            Err(e) => self.settings_error = Some(e.to_string()),
        }
    }

    fn ensure_session(&mut self, id: &str) {
        if self.sessions.contains_key(id) {
            return;
        }
        let Some(mut handle) = self.catalog.open_view(id) else {
            return;
        };
        if let Some(value) = self.state.settings().tool_options(id) {
            handle.restore_options(value);
        }
        self.sessions.insert(id.to_string(), handle);
    }

    fn persist_session_options(&mut self, id: &str) {
        let Some((tool_id, value)) = self
            .sessions
            .get(id)
            .and_then(|handle| handle.persistable_options())
        else {
            return;
        };
        if self.state.settings().tool_options(&tool_id) == Some(&value) {
            return;
        }
        let result = self.state.set_tool_options(tool_id, value);
        self.apply_setting(result);
    }

    fn render_settings_error(&self, ui: &mut egui::Ui, palette: Palette) {
        if let Some(msg) = &self.settings_error {
            ui.colored_label(
                palette.danger,
                format!("设置保存失败，本次更改未写入磁盘：{msg}"),
            );
        }
    }

    fn group_icon(group: GroupId) -> Icon {
        match group {
            GroupId::Converters => Icon::Replace,
            GroupId::EncodersDecoders => Icon::SquareTerminal,
            GroupId::Formatters => Icon::CaseSensitive,
            GroupId::Generators => Icon::Plus,
            GroupId::Graphic => Icon::Palette,
            GroupId::Testers => Icon::Inspector,
            GroupId::Text => Icon::ALargeSmall,
        }
    }

    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette;
        let panel = egui::Panel::left("sidebar")
            .resizable(false)
            .default_size(SIDEBAR_WIDTH)
            .show_separator_line(false)
            .frame(Frame::new().fill(palette.panel).inner_margin(Margin {
                left: 12,
                right: 12,
                top: 14,
                bottom: 10,
            }));
        panel.show(ui, |ui| {
            ui.horizontal(|ui| {
                let (badge, _) = ui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(badge, CornerRadius::same(8), palette.accent);
                theme::paint_icon(ui, Icon::SquareTerminal, badge, 16.0, palette.on_accent);
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("DevToys")
                        .font(theme::semibold(17.0))
                        .color(palette.text),
                );
            });
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text(t("host.search_placeholder"))
                        .desired_width(f32::INFINITY),
                );
            });
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .id_salt("sidebar-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.sidebar_nav(ui);
                });

            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                if widgets::nav_row(
                    ui,
                    &palette,
                    Icon::Settings,
                    t("host.settings"),
                    self.page == Page::Settings,
                    4.0,
                )
                .clicked()
                {
                    self.go(Page::Settings);
                }
            });
        });
    }

    fn sidebar_nav(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette;
        match self.state.registry().search(&self.search) {
            SearchOutcome::Empty => {
                widgets::nav_row(
                    ui,
                    &palette,
                    Icon::Search,
                    t("host.no_tools_found"),
                    false,
                    4.0,
                );
                return;
            }
            SearchOutcome::Hits(hits) => {
                let items: Vec<(String, String)> = hits
                    .into_iter()
                    .map(|tool| (tool.id.as_str().to_string(), tool.display_name.to_string()))
                    .collect();
                for (id, name) in items {
                    let active = self.page_is_tool(&id);
                    if widgets::nav_row(ui, &palette, Icon::ChevronRight, &name, active, 12.0)
                        .clicked()
                    {
                        let hit = self.recommended_hit(&id);
                        self.open_tool(&id, hit);
                    }
                }
                return;
            }
            SearchOutcome::Idle => {}
        }

        if widgets::nav_row(
            ui,
            &palette,
            Icon::LayoutDashboard,
            t("host.all_tools"),
            self.page == Page::AllTools,
            4.0,
        )
        .clicked()
        {
            self.go(Page::AllTools);
        }

        let favorites: Vec<(String, String)> = self
            .state
            .favorite_tools()
            .into_iter()
            .map(|tool| (tool.id.as_str().to_string(), tool.display_name.to_string()))
            .collect();
        if widgets::nav_row(
            ui,
            &palette,
            Icon::Star,
            t("host.favorites"),
            self.page == Page::Favorites,
            4.0,
        )
        .clicked()
        {
            self.go(Page::Favorites);
        }
        for (id, name) in favorites {
            let active = self.page_is_tool(&id);
            if widgets::nav_row(ui, &palette, Icon::ChevronRight, &name, active, 18.0).clicked() {
                let hit = self.recommended_hit(&id);
                self.open_tool(&id, hit);
            }
        }

        for group in GroupId::ALL {
            let tools: Vec<(String, String)> = self
                .state
                .registry()
                .in_group(group)
                .into_iter()
                .map(|tool| (tool.id.as_str().to_string(), tool.display_name.to_string()))
                .collect();
            let group_page = Page::Group(group);
            let open = self.page_is_group(group);
            if widgets::nav_row(
                ui,
                &palette,
                Self::group_icon(group),
                group.display_name(),
                self.page == group_page,
                4.0,
            )
            .clicked()
            {
                self.go(Page::Group(group));
            }
            if open {
                for (id, name) in tools {
                    let active = self.page_is_tool(&id);
                    if widgets::nav_row(ui, &palette, Icon::ChevronRight, &name, active, 18.0)
                        .clicked()
                    {
                        let hit = self.recommended_hit(&id);
                        self.open_tool(&id, hit);
                    }
                }
            }
        }
    }

    fn show_banner(&mut self, ui: &mut egui::Ui) {
        if self.recommendations.is_empty() || !self.state.settings().smart_detection_enabled {
            return;
        }
        let palette = self.palette;
        let recs: Vec<(usize, String, String)> = self
            .recommendations
            .iter()
            .enumerate()
            .map(|(index, hit)| {
                let name = self
                    .tool_by_id(&hit.tool_id)
                    .map(|tool| tool.display_name.to_string())
                    .unwrap_or_else(|| hit.tool_id.clone());
                (index, hit.tool_id.clone(), name)
            })
            .collect();

        egui::Frame::new()
            .fill(palette.accent.gamma_multiply(0.12))
            .inner_margin(Margin::symmetric(16, 8))
            .stroke(Stroke::new(0.0, palette.outline))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    theme::icon_button(
                        ui,
                        Icon::Bot,
                        16.0,
                        palette.accent,
                        palette.accent_hover,
                        "",
                    );
                    ui.label(
                        egui::RichText::new("剪贴板建议")
                            .font(theme::semibold(12.5))
                            .color(palette.text),
                    );
                    for (index, _, name) in recs {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new(name).color(palette.accent))
                                    .fill(egui::Color32::TRANSPARENT),
                            )
                            .clicked()
                        {
                            if let Some(hit) = self.recommendations.get(index).cloned() {
                                let id = hit.tool_id.clone();
                                self.open_tool(&id, Some(hit));
                            }
                        }
                    }
                });
            });
    }

    fn tool_cards(&self, tools: Vec<&ToolMetadata>) -> Vec<(String, String, String, Icon, bool)> {
        tools
            .into_iter()
            .map(|tool| {
                let id = tool.id.as_str().to_string();
                (
                    id.clone(),
                    tool.display_name.to_string(),
                    tool.group.display_name().to_string(),
                    Self::group_icon(tool.group),
                    self.is_recommended(&id),
                )
            })
            .collect()
    }

    fn show_tool_list(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        cards: Vec<(String, String, String, Icon, bool)>,
        empty: &str,
    ) {
        let palette = self.palette;
        let count = cards.len();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(title)
                        .font(theme::semibold(22.0))
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new("选择一个工具开始处理内容")
                        .font(theme::regular(13.0))
                        .color(palette.dim),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let chip = format!("{count} 个工具");
                let galley = ui.fonts_mut(|fonts| {
                    fonts.layout(chip, theme::regular(12.0), palette.accent, 120.0)
                });
                let size = vec2(galley.size().x + 16.0, 24.0);
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(12),
                    palette.accent.gamma_multiply(0.12),
                );
                ui.painter().galley(
                    egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0),
                    galley,
                    palette.accent,
                );
            });
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        if cards.is_empty() {
            widgets::empty_state(
                ui,
                &palette,
                empty,
                "可从左侧浏览其他分组，或使用搜索快速定位",
            );
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("tool-list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                for (id, name, group, icon, recommended) in cards {
                    if widgets::tool_card(ui, &palette, icon, &name, &group, recommended).clicked()
                    {
                        let hit = self.recommended_hit(&id);
                        self.open_tool(&id, hit);
                    }
                }
            });
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette;
        let theme = self.state.settings().theme;
        let detection = self.state.settings().smart_detection_enabled;
        let paste = self.state.settings().smart_detection_paste;
        egui::ScrollArea::vertical()
            .id_salt("settings")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(t("settings.title"))
                        .font(theme::semibold(20.0))
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new(t("settings.theme_desc"))
                        .font(theme::regular(13.0))
                        .color(palette.dim),
                );
                ui.add_space(16.0);
                self.render_settings_error(ui, palette);

                widgets::section_card(ui, &palette, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                        theme::paint_icon(ui, Icon::Palette, icon_rect, 16.0, palette.secondary);
                        ui.label(
                            egui::RichText::new(t("settings.appearance"))
                                .font(theme::semibold(15.0))
                                .color(palette.text),
                        );
                    });
                    ui.label(
                        egui::RichText::new(t("settings.theme_desc"))
                            .font(theme::regular(13.0))
                            .color(palette.dim),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        for (label, value) in [
                            (t("settings.theme_light"), ThemePreference::Light),
                            (t("settings.theme_dark"), ThemePreference::Dark),
                            (t("settings.theme_system"), ThemePreference::System),
                        ] {
                            let selected = theme == value;
                            if crate_toggle(ui, selected, label).clicked() {
                                let result = self.state.set_theme(value);
                                self.apply_setting(result);
                                self.applied_dark = None;
                            }
                        }
                    });
                });

                ui.add_space(12.0);
                widgets::section_card(ui, &palette, |ui| {
                    ui.label(
                        egui::RichText::new(t("settings.behavior"))
                            .font(theme::semibold(15.0))
                            .color(palette.text),
                    );
                    ui.label(
                        egui::RichText::new(t("settings.smart_detection_desc"))
                            .font(theme::regular(13.0))
                            .color(palette.dim),
                    );
                    ui.add_space(8.0);
                    let mut detection = detection;
                    if ui
                        .checkbox(&mut detection, t("settings.smart_detection"))
                        .changed()
                    {
                        let result = self.state.set_smart_detection_enabled(detection);
                        self.apply_setting(result);
                    }
                    ui.add_enabled_ui(detection, |ui| {
                        let mut paste = paste;
                        if ui.checkbox(&mut paste, t("settings.auto_paste")).changed() {
                            let result = self.state.set_smart_detection_paste(paste);
                            self.apply_setting(result);
                        }
                    });
                });
            });
    }

    fn show_tool_page(&mut self, ui: &mut egui::Ui, id: &str) {
        let palette = self.palette;
        let meta = self.tool_by_id(id);
        let title = meta.map(|tool| tool.display_name).unwrap_or(id).to_string();
        let favorable = meta.map(|tool| tool.favorable).unwrap_or(false);
        let group = meta.map(|tool| tool.group.display_name());
        let favorited = self.state.is_favorite(id);
        let tool_id = id.to_string();

        self.render_settings_error(ui, palette);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .font(theme::semibold(16.0))
                    .color(palette.text),
            );
            if let Some(group) = group {
                ui.label(
                    egui::RichText::new(group)
                        .font(theme::regular(12.0))
                        .color(palette.dim),
                );
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if favorable {
                    let (icon, label) = if favorited {
                        (Icon::StarOff, t("host.favorite_remove"))
                    } else {
                        (Icon::Star, t("host.favorite_add"))
                    };
                    if theme::icon_button(ui, icon, 16.0, palette.secondary, palette.accent, label)
                        .clicked()
                    {
                        let result = self.state.toggle_favorite(&tool_id);
                        self.apply_setting(result);
                    }
                }
            });
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        if let Some(handle) = self.sessions.get_mut(id) {
            handle.ui(ui);
        } else {
            ui.colored_label(palette.dim, t("host.tool_unavailable"));
        }
        self.persist_session_options(id);
    }

    fn show_content(&mut self, ui: &mut egui::Ui) {
        match self.page.clone() {
            Page::AllTools => {
                let cards = self.tool_cards(self.state.registry().all().iter().collect());
                self.show_tool_list(ui, t("host.all_tools"), cards, t("host.no_tools"));
            }
            Page::Favorites => {
                let cards = self.tool_cards(self.state.favorite_tools());
                self.show_tool_list(ui, t("host.favorites"), cards, t("host.no_favorites"));
            }
            Page::Group(group) => {
                let cards = self.tool_cards(self.state.registry().in_group(group));
                self.show_tool_list(ui, group.display_name(), cards, t("host.no_tools"));
            }
            Page::Tool(id) => self.show_tool_page(ui, &id),
            Page::Settings => self.show_settings(ui),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette;
        ui.painter().rect_filled(ui.max_rect(), 0.0, palette.window);

        self.show_sidebar(ui);
        egui::CentralPanel::default()
            .frame(Frame::new().fill(palette.window).inner_margin(Margin::ZERO))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    self.show_banner(ui);
                    Frame::new()
                        .inner_margin(Margin::symmetric(24, 20))
                        .show(ui, |ui| {
                            self.show_content(ui);
                        });
                });
            });
    }
}

fn crate_toggle(ui: &mut egui::Ui, selected: bool, text: &str) -> egui::Response {
    let mut button = egui::Button::new(text);
    if selected {
        button = button.fill(ui.visuals().selection.bg_fill).selected(true);
    }
    ui.add(button)
}

impl eframe::App for Workspace {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_theme(ctx);
        self.tick_detect(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.palette.window.to_normalized_gamma_f32()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use devtoys_api::{RawData, JSON_FORMATTER_ID, TYPE_IMAGE, TYPE_JSON};
    use devtoys_core::{rgba_to_png, ClipboardSource, InMemoryClipboard};
    use devtoys_tools::{default_catalog, format_json, Indentation, ToolView};
    use std::time::Duration;

    #[test]
    fn idle_detection_tick_requests_a_future_egui_pass() {
        let mut workspace = Workspace {
            state: AppState::bootstrap(
                Vec::new(),
                SettingsStore::in_dir(
                    std::env::temp_dir().join(format!("devtoys-idle-host-{}", std::process::id())),
                ),
            ),
            coordinator: DetectionCoordinator::new(
                Arc::new(DetectionEngine::new(Vec::new(), &[])),
                Box::new(InMemoryClipboard::new()),
            ),
            page: Page::AllTools,
            search: String::new(),
            sessions: HashMap::new(),
            recommendations: Vec::new(),
            palette: Palette::dark(),
            applied_dark: None,
            settings_error: None,
            catalog: ToolCatalog::default(),
        };
        let ctx = egui::Context::default();
        // Egui requests startup passes; settle those before observing the host.
        for _ in 0..4 {
            ctx.run_ui(egui::RawInput::default(), |_| {})
                .textures_delta
                .clear();
        }
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            workspace.tick_detect(ui.ctx())
        });
        let delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
        output.textures_delta.clear();
        assert!(
            delay > Duration::ZERO && delay <= Duration::from_millis(800),
            "idle monitoring must schedule egui, got {delay:?}"
        );
    }

    #[derive(Default)]
    struct RecordingView {
        text: Option<String>,
        image: Option<(Vec<u8>, Option<String>)>,
    }

    impl ToolView for RecordingView {
        fn ui(&mut self, _ui: &mut egui::Ui) {}

        fn on_data_received(&mut self, payload: &str) {
            self.text = Some(payload.to_string());
        }

        fn on_image_received(&mut self, bytes: &[u8], mime: Option<&str>) {
            self.image = Some((bytes.to_vec(), mime.map(str::to_string)));
        }
    }

    fn known_png() -> Vec<u8> {
        let rgba = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];
        rgba_to_png(2, 2, &rgba).expect("encode known pixels")
    }

    fn dispatch(view: &mut RecordingView, hit: &Recommendation, clipboard: Option<&RawData>) {
        view.on_detected_data(&hit.paste_payload(clipboard));
    }

    #[test]
    fn paste_dispatch_image_bytes_not_mime_string() {
        let png = known_png();
        let mut hit = Recommendation::new("ImageConverter", TYPE_IMAGE, "image/png");
        hit.bytes = Some(png.clone());
        hit.mime = Some("image/png".into());
        let mut view = RecordingView::default();
        dispatch(&mut view, &hit, None);
        assert_eq!(view.text, None, "must not treat MIME as a path/text paste");
        let (bytes, mime) = view.image.expect("image bytes dispatched");
        assert_eq!(bytes, png);
        assert_eq!(mime.as_deref(), Some("image/png"));
    }

    #[test]
    fn paste_dispatch_keeps_clipboard_image_when_recommendation_lacks_bytes() {
        let png = known_png();
        let raw = RawData::Image {
            bytes: png.clone(),
            mime: Some("image/png".into()),
        };
        let hit = Recommendation::new("ImageConverter", TYPE_IMAGE, "image/png");
        let mut view = RecordingView::default();
        dispatch(&mut view, &hit, Some(&raw));
        assert_eq!(view.text, None);
        let (bytes, mime) = view.image.expect("clipboard image fallback");
        assert_eq!(bytes, png);
        assert_eq!(mime.as_deref(), Some("image/png"));
    }

    #[test]
    fn paste_dispatch_text_json_still_uses_string() {
        let hit = Recommendation::new("JsonFormatter", TYPE_JSON, r#"{"a":1}"#);
        let mut view = RecordingView::default();
        dispatch(&mut view, &hit, Some(&RawData::text(r#"{"a":1}"#)));
        assert_eq!(view.text.as_deref(), Some(r#"{"a":1}"#));
        assert!(view.image.is_none());
    }

    #[test]
    fn assembled_clipboard_image_recommends_converter_with_pixels() {
        use std::sync::atomic::AtomicBool;

        use devtoys_core::{all_detectors, DetectOptions};

        let png = known_png();
        let clip = InMemoryClipboard::new();
        clip.set_image(png.clone(), Some("image/png".into()));
        let raw = clip.read_raw().unwrap();

        let catalog = default_catalog();
        let tools = catalog.all_metadata();
        let mut detectors = all_detectors();
        detectors.extend(catalog.all_detectors());
        let engine = DetectionEngine::new(detectors, &tools);
        let recs = engine.detect(
            &raw,
            DetectOptions {
                strict: false,
                active_tool: None,
                enabled: true,
                cancel: &AtomicBool::new(false),
            },
        );
        let hit = recs
            .iter()
            .find(|r| r.tool_id == "ImageConverter")
            .expect("Image Converter must be recommended for a clipboard PNG");
        assert_eq!(hit.bytes.as_ref(), Some(&png));

        let mut view = RecordingView::default();
        dispatch(&mut view, hit, Some(&raw));
        let (bytes, _) = view.image.expect("view received image bytes");
        assert_eq!(bytes, png);
        assert_eq!(view.text, None);
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_ne!(bytes.as_slice(), b"image/png");
    }

    fn workspace_in(dir: impl AsRef<std::path::Path>) -> Workspace {
        let catalog = ToolCatalog::default();
        let tools = catalog.all_metadata();
        Workspace {
            state: AppState::bootstrap(tools, SettingsStore::in_dir(dir)),
            coordinator: DetectionCoordinator::new(
                Arc::new(DetectionEngine::new(Vec::new(), &[])),
                Box::new(InMemoryClipboard::new()),
            ),
            page: Page::AllTools,
            search: String::new(),
            sessions: HashMap::new(),
            recommendations: Vec::new(),
            palette: Palette::dark(),
            applied_dark: None,
            settings_error: None,
            catalog,
        }
    }

    fn indent_from_settings(value: &str) -> Indentation {
        match value {
            "two_spaces" => Indentation::TwoSpaces,
            "four_spaces" => Indentation::FourSpaces,
            "one_tab" => Indentation::OneTab,
            "minified" => Indentation::Minified,
            other => panic!("unexpected indent {other}"),
        }
    }

    #[test]
    fn json_formatter_options_survive_store_rebuild_and_format() {
        let dir = tempfile::tempdir().unwrap();
        let mut workspace = workspace_in(dir.path());
        workspace.open_tool(JSON_FORMATTER_ID, None);
        workspace
            .sessions
            .get_mut(JSON_FORMATTER_ID)
            .unwrap()
            .restore_options(&serde_json::json!({
                "indent": "four_spaces",
                "sort_properties": true
            }));
        workspace.persist_session_options(JSON_FORMATTER_ID);
        assert_eq!(workspace.settings_error, None);

        let mut rebuilt = workspace_in(dir.path());
        rebuilt.open_tool(JSON_FORMATTER_ID, None);
        let (id, value) = rebuilt
            .sessions
            .get(JSON_FORMATTER_ID)
            .unwrap()
            .persistable_options()
            .unwrap();
        assert_eq!(id, JSON_FORMATTER_ID);
        assert_eq!(value["indent"], "four_spaces");
        assert_eq!(value["sort_properties"], true);
        assert!(value.get("input").is_none());
        assert!(value.get("output").is_none());

        let got = format_json(
            r#"{"z":1,"a":2}"#,
            indent_from_settings(value["indent"].as_str().unwrap()),
            value["sort_properties"].as_bool().unwrap(),
        )
        .unwrap();
        assert_eq!(got, "{\n    \"a\": 2,\n    \"z\": 1\n}");
        assert_eq!(
            rebuilt.state.settings().theme,
            workspace.state.settings().theme
        );
        assert!(rebuilt.state.settings().smart_detection_enabled);
    }

    #[test]
    fn json_formatter_option_save_failure_is_visible_and_keeps_disk() {
        let dir = tempfile::tempdir().unwrap();
        let store_dir = dir.path().join("ok");
        let mut workspace = workspace_in(&store_dir);
        workspace.open_tool(JSON_FORMATTER_ID, None);
        workspace
            .sessions
            .get_mut(JSON_FORMATTER_ID)
            .unwrap()
            .restore_options(&serde_json::json!({
                "indent": "minified",
                "sort_properties": true
            }));
        workspace.persist_session_options(JSON_FORMATTER_ID);
        assert_eq!(workspace.settings_error, None);
        let before = std::fs::read(store_dir.join("settings.json")).unwrap();

        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let mut failing = workspace_in(blocker.join("nested"));
        failing.open_tool(JSON_FORMATTER_ID, None);
        failing
            .sessions
            .get_mut(JSON_FORMATTER_ID)
            .unwrap()
            .restore_options(&serde_json::json!({
                "indent": "one_tab",
                "sort_properties": false
            }));
        failing.persist_session_options(JSON_FORMATTER_ID);
        assert!(
            failing.settings_error.is_some(),
            "save failure must surface via settings_error"
        );
        assert_eq!(
            std::fs::read(store_dir.join("settings.json")).unwrap(),
            before
        );

        let mut rebuilt = workspace_in(&store_dir);
        rebuilt.open_tool(JSON_FORMATTER_ID, None);
        let value = rebuilt
            .sessions
            .get(JSON_FORMATTER_ID)
            .unwrap()
            .persistable_options()
            .unwrap()
            .1;
        assert_eq!(value["indent"], "minified");
        assert_eq!(value["sort_properties"], true);
    }

    #[test]
    fn illegal_stored_indent_restores_default_then_formats() {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::in_dir(dir.path());
        let mut settings = devtoys_api::AppSettings::default();
        settings.theme = ThemePreference::Dark;
        settings.set_tool_options(
            JSON_FORMATTER_ID,
            serde_json::json!({"indent": "eight_spaces", "sort_properties": true}),
        );
        store.save(&settings).unwrap();

        let mut workspace = workspace_in(dir.path());
        workspace.open_tool(JSON_FORMATTER_ID, None);
        let value = workspace
            .sessions
            .get(JSON_FORMATTER_ID)
            .unwrap()
            .persistable_options()
            .unwrap()
            .1;
        assert_eq!(value["indent"], "two_spaces");
        assert_eq!(value["sort_properties"], true);
        let got = format_json(
            r#"{"z":1,"a":2}"#,
            indent_from_settings(value["indent"].as_str().unwrap()),
            value["sort_properties"].as_bool().unwrap(),
        )
        .unwrap();
        assert_eq!(got, "{\n  \"a\": 2,\n  \"z\": 1\n}");
        assert_eq!(workspace.state.settings().theme, ThemePreference::Dark);
    }

    #[test]
    fn host_shell_legacy_english_setting_loads_in_chinese() {
        let dir = tempfile::tempdir().unwrap();
        // Write old settings with language: "en_us"
        let settings_path = dir.path().join("settings.json");
        std::fs::write(
            &settings_path,
            serde_json::json!({
                "theme": "dark",
                "language": "en_us",
                "smart_detection_enabled": true,
                "smart_detection_paste": true,
                "favorites": [JSON_FORMATTER_ID],
                "window": null,
                "tool_options": {}
            })
            .to_string(),
        )
        .unwrap();

        let mut workspace = workspace_in(dir.path());
        assert_eq!(t("host.all_tools"), "全部工具");
        assert_eq!(t("host.favorites"), "收藏");
        assert_eq!(t("host.settings"), "设置");
        assert_eq!(GroupId::Converters.display_name(), "转换器");

        // Saving persists without language key
        workspace.state.set_theme(ThemePreference::Light).unwrap();
        let saved = std::fs::read_to_string(settings_path).unwrap();
        assert!(
            !saved.contains("language"),
            "persisted settings must not contain language: {saved}"
        );

        // Reload
        let reloaded = workspace_in(dir.path());
        assert_eq!(reloaded.state.settings().theme, ThemePreference::Light);
    }

    #[test]
    fn host_shell_search_labels_localized() {
        assert_eq!(t("host.search_placeholder"), "键入以搜索工具...");
        assert_eq!(t("host.no_tools_found"), "未找到相关工具");
        assert_eq!(t("settings.title"), "设置");
    }

    #[test]
    fn legacy_extension_in_favorites_does_not_break_startup_or_favorites() {
        let dir = tempfile::tempdir().unwrap();
        // Simulate a legacy extensions directory on disk that should remain untouched
        let ext_dir = dir.path().join("extensions").join("legacy_ext");
        std::fs::create_dir_all(&ext_dir).unwrap();
        std::fs::write(ext_dir.join("devtoys-extension.toml"), "id = 'LegacyExt'").unwrap();

        // Pre-populate settings with a legacy extension ID and a valid builtin tool in favorites
        let settings_path = dir.path().join("settings.json");
        std::fs::write(
            settings_path,
            serde_json::json!({
                "theme": "dark",
                "language": "zh_cn",
                "smart_detection_enabled": true,
                "smart_detection_paste": true,
                "favorites": ["LegacyExt", JSON_FORMATTER_ID],
                "window": null,
                "tool_options": {}
            })
            .to_string(),
        )
        .unwrap();

        let workspace = workspace_in(dir.path());
        // Favorite tools only return existing tools (JsonFormatter), LegacyExt is ignored gracefully
        let favs = workspace.state.favorite_tools();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].id.as_str(), JSON_FORMATTER_ID);

        // Reload from disk
        let reloaded = workspace_in(dir.path());
        let reloaded_favs = reloaded.state.favorite_tools();
        assert_eq!(reloaded_favs.len(), 1);
        assert_eq!(reloaded_favs[0].id.as_str(), JSON_FORMATTER_ID);

        // Verify disk extension file remains untouched
        assert!(ext_dir.join("devtoys-extension.toml").exists());
    }

    #[test]
    fn normal_window_tool_navigation_and_session_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let mut workspace = workspace_in(dir.path());
        workspace.open_tool(JSON_FORMATTER_ID, None);

        let sample_json = r#"{"devtoys":"rocks","active":true}"#;
        let handle = workspace.sessions.get_mut(JSON_FORMATTER_ID).unwrap();
        handle.on_data_received(sample_json);

        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            workspace.show(ui);
        });
        let cmds = output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .clone();
        output.textures_delta.clear();
        assert!(!cmds.contains(&egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop
        )));

        // Session data is intact
        let session = workspace.sessions.get(JSON_FORMATTER_ID).unwrap();
        let options = session.persistable_options().unwrap().1;
        assert_eq!(options["indent"], "two_spaces");
    }
}
