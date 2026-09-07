use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use devtoys_api::{
    GroupId, ThemePreference, ToolMetadata, ALL_TOOLS_LABEL, FAVORITES_LABEL, SETTINGS_ID,
};
use devtoys_core::{
    AppState, DetectionCoordinator, DetectionEngine, Recommendation, SearchOutcome, SettingsStore,
};
use devtoys_tools::{default_catalog, ToolHandle};
use egui::{Align, CornerRadius, Frame, Layout, Margin, Stroke, Vec2, vec2};

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
}

impl Workspace {
    pub fn new() -> Self {
        let catalog = default_catalog();
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
        if self.coordinator.is_detecting() {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
    }

    fn tool_by_id(&self, id: &str) -> Option<&ToolMetadata> {
        self.state.registry().get(id)
    }

    fn is_recommended(&self, id: &str) -> bool {
        self.recommendations.iter().any(|item| item.tool_id == id)
    }

    fn recommended_payload(&self, id: &str) -> Option<String> {
        self.recommendations
            .iter()
            .find(|item| item.tool_id == id)
            .map(|item| item.payload.clone())
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
        if let Page::Tool(id) = &self.page {
            self.recommendations.retain(|item| item.tool_id != *id);
        }
    }

    fn open_tool(&mut self, id: &str, payload: Option<String>) {
        if id == SETTINGS_ID {
            self.go(Page::Settings);
            return;
        }

        let _ = self.state.open_tool(id);
        self.page = Page::Tool(id.to_string());
        self.recommendations.retain(|item| item.tool_id != id);

        if !self.sessions.contains_key(id) {
            if let Some(handle) = default_catalog().open_view(id) {
                self.sessions.insert(id.to_string(), handle);
            }
        }

        if let Some(payload) = payload {
            if self.state.settings().paste_enabled() {
                if let Some(handle) = self.sessions.get_mut(id) {
                    handle.on_data_received(&payload);
                }
            }
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
                ui.painter().rect_filled(badge, CornerRadius::same(8), palette.accent);
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
                        .hint_text("搜索工具")
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
                    "设置",
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
                widgets::nav_row(ui, &palette, Icon::Search, "未找到匹配工具", false, 4.0);
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
                        let payload = self.recommended_payload(&id);
                        self.open_tool(&id, payload);
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
            ALL_TOOLS_LABEL,
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
            FAVORITES_LABEL,
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
                let payload = self.recommended_payload(&id);
                self.open_tool(&id, payload);
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
                        let payload = self.recommended_payload(&id);
                        self.open_tool(&id, payload);
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
                                egui::Button::new(
                                    egui::RichText::new(name).color(palette.accent),
                                )
                                .fill(egui::Color32::TRANSPARENT),
                            )
                            .clicked()
                        {
                            if let Some(hit) = self.recommendations.get(index).cloned() {
                                self.open_tool(&hit.tool_id, Some(hit.payload));
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
                    egui::pos2(
                        rect.left() + 8.0,
                        rect.center().y - galley.size().y / 2.0,
                    ),
                    galley,
                    palette.accent,
                );
            });
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        if cards.is_empty() {
            widgets::empty_state(ui, &palette, empty, "可从左侧浏览其他分组，或使用搜索快速定位");
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
                        let payload = self.recommended_payload(&id);
                        self.open_tool(&id, payload);
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
                    egui::RichText::new("设置")
                        .font(theme::semibold(20.0))
                        .color(palette.text),
                );
                ui.label(
                    egui::RichText::new("按你的工作习惯调整外观与智能辅助行为")
                        .font(theme::regular(13.0))
                        .color(palette.dim),
                );
                ui.add_space(16.0);

                widgets::section_card(ui, &palette, |ui| {
                    ui.horizontal(|ui| {
                        let (icon_rect, _) =
                            ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
                        theme::paint_icon(ui, Icon::Palette, icon_rect, 16.0, palette.secondary);
                        ui.label(
                            egui::RichText::new("外观")
                                .font(theme::semibold(15.0))
                                .color(palette.text),
                        );
                    });
                    ui.label(
                        egui::RichText::new("主题切换会立即应用，并在下次启动时保留。")
                            .font(theme::regular(13.0))
                            .color(palette.dim),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        for (label, value) in [
                            ("浅色", ThemePreference::Light),
                            ("深色", ThemePreference::Dark),
                            ("跟随系统", ThemePreference::System),
                        ] {
                            let selected = theme == value;
                            if crate_toggle(ui, selected, label).clicked() {
                                let _ = self.state.set_theme(value);
                                self.applied_dark = None;
                            }
                        }
                    });
                });

                ui.add_space(12.0);
                widgets::section_card(ui, &palette, |ui| {
                    ui.label(
                        egui::RichText::new("行为")
                            .font(theme::semibold(15.0))
                            .color(palette.text),
                    );
                    ui.label(
                        egui::RichText::new("控制剪贴板识别与自动填充。")
                            .font(theme::regular(13.0))
                            .color(palette.dim),
                    );
                    ui.add_space(8.0);
                    let mut detection = detection;
                    if ui
                        .checkbox(&mut detection, "Smart Detection 总开关")
                        .changed()
                    {
                        let _ = self.state.set_smart_detection_enabled(detection);
                        if !detection {
                            self.coordinator.clear();
                            self.recommendations.clear();
                        }
                    }
                    ui.add_enabled_ui(detection, |ui| {
                        let mut paste = paste;
                        if ui.checkbox(&mut paste, "推荐工具自动粘贴").changed() {
                            let _ = self.state.set_smart_detection_paste(paste);
                        }
                    });
                });
            });
    }

    fn show_tool_page(&mut self, ui: &mut egui::Ui, id: &str) {
        let palette = self.palette;
        let meta = self.tool_by_id(id);
        let title = meta
            .map(|tool| tool.display_name)
            .unwrap_or(id)
            .to_string();
        let favorable = meta.map(|tool| tool.favorable).unwrap_or(false);
        let group = meta.map(|tool| tool.group.display_name());
        let favorited = self.state.is_favorite(id);
        let tool_id = id.to_string();

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
            if favorable {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let (icon, label) = if favorited {
                        (Icon::StarOff, "取消收藏")
                    } else {
                        (Icon::Star, "收藏")
                    };
                    if theme::icon_button(
                        ui,
                        icon,
                        16.0,
                        palette.secondary,
                        palette.accent,
                        label,
                    )
                    .clicked()
                    {
                        let _ = self.state.toggle_favorite(&tool_id);
                    }
                });
            }
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        match self.sessions.get_mut(id) {
            Some(handle) => {
                handle.ui(ui);
            }
            None => {
                ui.colored_label(palette.dim, "此工具暂不可打开");
            }
        }
    }

    fn show_content(&mut self, ui: &mut egui::Ui) {
        match self.page.clone() {
            Page::AllTools => {
                let cards = self.tool_cards(self.state.registry().all().iter().collect());
                self.show_tool_list(ui, ALL_TOOLS_LABEL, cards, "暂无工具");
            }
            Page::Favorites => {
                let cards = self.tool_cards(self.state.favorite_tools());
                self.show_tool_list(ui, FAVORITES_LABEL, cards, "暂无收藏");
            }
            Page::Group(group) => {
                let cards = self.tool_cards(self.state.registry().in_group(group));
                self.show_tool_list(ui, group.display_name(), cards, "此分组暂无工具");
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
        button = button
            .fill(ui.visuals().selection.bg_fill)
            .selected(true);
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
