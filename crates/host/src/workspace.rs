use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use devtoys_api::{
    GroupId, RawData, ThemePreference, ToolMetadata, ALL_TOOLS_LABEL, FAVORITES_LABEL, SETTINGS_ID,
};
use devtoys_core::{
    AppState, DetectOptions, DetectionEngine, Recommendation, SearchOutcome, SettingsStore,
};
use devtoys_tools::{all_tools, open_gui_tool, ToolHandle};
use gpui::{
    div, prelude::*, px, App, ClickEvent, ClipboardEntry, Context, Entity, SharedString,
    Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::{Radio, RadioGroup};
use gpui_component::scroll::ScrollableElement;
use gpui_component::sidebar::{Sidebar, SidebarMenu, SidebarMenuItem};
use gpui_component::switch::Switch;
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Disableable, Icon, IconName, StyledExt, Theme, ThemeMode,
};

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
    engine: Arc<DetectionEngine>,
    page: Page,
    search: Entity<InputState>,
    search_query: String,
    sessions: HashMap<String, ToolHandle>,
    last_clipboard: Option<RawData>,
    recommendations: Vec<Recommendation>,
    detect_cancel: Arc<AtomicBool>,
    detect_gen: u64,
    _subscriptions: Vec<Subscription>,
}

impl Workspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let tools = all_tools();
        let mut detectors = devtoys_core::all_detectors();
        detectors.extend(devtoys_tools::tool_detectors());
        let engine = Arc::new(DetectionEngine::new(detectors, &tools));
        let state = AppState::bootstrap(tools, SettingsStore::user());

        let search = cx.new(|cx| {
            InputState::new(window, cx).placeholder("搜索工具")
        });

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe(
            &search,
            |this, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.search_query = input.read(cx).value().to_string();
                    cx.notify();
                }
            },
        ));
        subscriptions.push(cx.observe_window_appearance(window, |this, window, cx| {
            if this.state.settings().theme == ThemePreference::System {
                Theme::sync_system_appearance(Some(window), cx);
            }
        }));

        let this = Self {
            state,
            engine,
            page: Page::AllTools,
            search,
            search_query: String::new(),
            sessions: HashMap::new(),
            last_clipboard: None,
            recommendations: Vec::new(),
            detect_cancel: Arc::new(AtomicBool::new(false)),
            detect_gen: 0,
            _subscriptions: subscriptions,
        };
        this.apply_theme(window, cx);
        this.start_clipboard_poll(window, cx);
        this
    }

    fn apply_theme(&self, window: &mut Window, cx: &mut App) {
        match self.state.settings().theme {
            ThemePreference::Light => Theme::change(ThemeMode::Light, Some(window), cx),
            ThemePreference::Dark => Theme::change(ThemeMode::Dark, Some(window), cx),
            ThemePreference::System => Theme::sync_system_appearance(Some(window), cx),
        }
    }

    fn start_clipboard_poll(&self, window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(800))
                .await;
            let _ = this.update_in(cx, |this, window, cx| this.tick_clipboard(window, cx));
        })
        .detach();
    }

    fn tick_clipboard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.state.settings().smart_detection_enabled {
            if !self.recommendations.is_empty() {
                self.recommendations.clear();
                cx.notify();
            }
            return;
        }

        let Some(item) = cx.read_from_clipboard() else {
            return;
        };
        let raw = clipboard_to_raw(&item);
        if raw.is_none() {
            return;
        }
        if self.last_clipboard.as_ref() == raw.as_ref() {
            return;
        }
        self.last_clipboard = raw.clone();
        if let Some(raw) = raw {
            self.start_detect(raw, window, cx);
        }
    }

    fn start_detect(&mut self, raw: RawData, window: &mut Window, cx: &mut Context<Self>) {
        self.detect_cancel.store(true, Ordering::Relaxed);
        let cancel = Arc::new(AtomicBool::new(false));
        self.detect_cancel = cancel.clone();
        self.detect_gen = self.detect_gen.wrapping_add(1);
        let gen = self.detect_gen;

        let engine = self.engine.clone();
        let active = match &self.page {
            Page::Tool(id) => Some(id.clone()),
            _ => None,
        };
        let timeout_exec = cx.background_executor().clone();
        timeout_exec
            .spawn({
                let cancel = cancel.clone();
                let timeout_exec = timeout_exec.clone();
                async move {
                    timeout_exec.timer(Duration::from_secs(2)).await;
                    cancel.store(true, Ordering::Relaxed);
                }
            })
            .detach();

        cx.spawn_in(window, async move |this, cx| {
            let cancel_flag = cancel.clone();
            let detected = cx
                .background_spawn(async move {
                    engine.detect(
                        &raw,
                        DetectOptions {
                            strict: false,
                            active_tool: active.as_deref(),
                            enabled: true,
                            cancel: cancel_flag.as_ref(),
                        },
                    )
                })
                .await;

            if cancel.load(Ordering::Relaxed) {
                return;
            }

            let _ = this.update(cx, |this, cx| {
                if this.detect_gen != gen {
                    return;
                }
                this.recommendations = detected
                    .into_iter()
                    .filter(|hit| match &this.page {
                        Page::Tool(open) => open != &hit.tool_id,
                        _ => true,
                    })
                    .collect();
                cx.notify();
            });
        })
        .detach();
    }

    fn tool_by_id(&self, id: &str) -> Option<&ToolMetadata> {
        self.state.registry().get(id)
    }

    fn favorite_tools(&self) -> Vec<&ToolMetadata> {
        self.state.favorite_tools()
    }

    fn recent_tools(&self) -> Vec<&ToolMetadata> {
        self.state.recent_tools()
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

    fn go(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        if let Page::Tool(id) = &self.page {
            self.recommendations.retain(|item| item.tool_id != *id);
        }
        cx.notify();
    }

    fn open_tool(
        &mut self,
        id: &str,
        payload: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if id == SETTINGS_ID {
            self.go(Page::Settings, cx);
            return;
        }

        let _ = self.state.open_tool(id);
        self.page = Page::Tool(id.to_string());
        self.recommendations.retain(|item| item.tool_id != id);

        if !self.sessions.contains_key(id) {
            if let Some(handle) = open_gui_tool(id, window, cx) {
                self.sessions.insert(id.to_string(), handle);
            }
        }

        if let Some(payload) = payload {
            if self.state.settings().paste_enabled() {
                if let Some(handle) = self.sessions.get_mut(id) {
                    handle.on_data_received(&payload, window, cx);
                }
            }
        }
        cx.notify();
    }

    fn open_recommendation(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(hit) = self.recommendations.get(index).cloned() {
            self.open_tool(&hit.tool_id, Some(hit.payload), window, cx);
        }
    }

    fn toggle_favorite(&mut self, id: &str, cx: &mut Context<Self>) {
        let _ = self.state.toggle_favorite(id);
        cx.notify();
    }

    fn set_theme(&mut self, theme: ThemePreference, window: &mut Window, cx: &mut Context<Self>) {
        let _ = self.state.set_theme(theme);
        self.apply_theme(window, cx);
        cx.notify();
    }

    fn set_smart_detection(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let _ = self.state.set_smart_detection_enabled(enabled);
        if !enabled {
            self.detect_cancel.store(true, Ordering::Relaxed);
            self.recommendations.clear();
        }
        cx.notify();
    }

    fn set_smart_paste(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if !self.state.settings().smart_detection_enabled {
            return;
        }
        let _ = self.state.set_smart_detection_paste(enabled);
        cx.notify();
    }

    fn set_show_recent(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let _ = self.state.set_show_recent(enabled);
        cx.notify();
    }

    fn tool_item(&self, tool: &ToolMetadata, cx: &mut Context<Self>) -> SidebarMenuItem {
        let id = tool.id.as_str().to_string();
        SidebarMenuItem::new(tool.display_name)
            .active(self.page_is_tool(&id))
            .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                let payload = this.recommended_payload(&id);
                this.open_tool(&id, payload, window, cx);
            }))
    }

    fn sidebar_menu(&self, cx: &mut Context<Self>) -> SidebarMenu {
        match self.state.registry().search(&self.search_query) {
            SearchOutcome::Empty => {
                return SidebarMenu::new().child(
                    SidebarMenuItem::new("未找到匹配工具")
                        .icon(IconName::Search)
                        .disable(true),
                );
            }
            SearchOutcome::Hits(hits) => {
                return SidebarMenu::new()
                    .children(hits.into_iter().map(|tool| self.tool_item(tool, cx)));
            }
            SearchOutcome::Idle => {}
        }

        let mut menu = SidebarMenu::new().child(
            SidebarMenuItem::new(ALL_TOOLS_LABEL)
                .icon(IconName::LayoutDashboard)
                .active(self.page == Page::AllTools)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.go(Page::AllTools, cx);
                })),
        );

        let favorites = self.favorite_tools();
        menu = menu.child(
            SidebarMenuItem::new(FAVORITES_LABEL)
                .icon(IconName::Star)
                .active(self.page == Page::Favorites)
                .default_open(!favorites.is_empty())
                .click_to_open(true)
                .children(favorites.into_iter().map(|tool| self.tool_item(tool, cx)))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.go(Page::Favorites, cx);
                })),
        );

        if self.state.settings().show_recent {
            let recent = self.recent_tools();
            if !recent.is_empty() {
                menu = menu.child(
                    SidebarMenuItem::new("最近使用")
                        .icon(IconName::Calendar)
                        .default_open(false)
                        .children(recent.into_iter().map(|tool| self.tool_item(tool, cx))),
                );
            }
        }

        for group in GroupId::ALL {
            let tools = self.state.registry().in_group(group);
            let group_page = Page::Group(group);
            let group_is_open = self.page_is_group(group);
            menu = menu.child(
                SidebarMenuItem::new(group.display_name())
                    .icon(group_icon(group))
                    .active(self.page == group_page)
                    .default_open(group_is_open)
                    .click_to_open(true)
                    .children(tools.into_iter().map(|tool| self.tool_item(tool, cx)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.go(Page::Group(group), cx);
                    })),
            );
        }

        menu
    }

    fn render_tool_list(
        &self,
        title: impl Into<SharedString>,
        tools: Vec<&ToolMetadata>,
        empty: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let title = title.into();
        let empty = empty.into();
        let tool_count = tools.len();

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .flex_none()
                    .items_center()
                    .justify_between()
                    .px_8()
                    .pt_7()
                    .pb_5()
                    .child(
                        v_flex()
                            .gap_1p5()
                            .child(div().text_xl().font_semibold().child(title))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("选择一个工具开始处理内容"),
                            ),
                    )
                    .child(
                        div()
                            .px_2p5()
                            .py_1()
                            .rounded_full()
                            .bg(cx.theme().primary.opacity(0.1))
                            .text_xs()
                            .font_medium()
                            .text_color(cx.theme().primary)
                            .child(format!("{tool_count} 个工具")),
                    ),
            )
            .child(div().mx_8().border_b_1().border_color(cx.theme().border))
            .map(|this| {
                if tools.is_empty() {
                    this.child(
                        v_flex()
                            .flex_1()
                            .items_center()
                            .justify_center()
                            .gap_3()
                            .child(
                                div()
                                    .size(px(48.))
                                    .rounded_full()
                                    .bg(cx.theme().muted)
                                    .text_color(cx.theme().muted_foreground)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(Icon::new(IconName::Inbox).size_5()),
                            )
                            .child(div().font_medium().child(empty))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("可从左侧浏览其他分组，或使用搜索快速定位"),
                            ),
                    )
                } else {
                    this.child(
                        v_flex()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .gap_2p5()
                            .p_8()
                            .bg(cx.theme().muted.opacity(0.18))
                            .children(tools.into_iter().map(|tool| {
                                let id = tool.id.as_str().to_string();
                                let recommended = self.is_recommended(&id);
                                div()
                                    .id(SharedString::from(format!("tool-card-{id}")))
                                    .flex_none()
                                    .cursor_pointer()
                                    .p_4()
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .rounded(cx.theme().radius_lg)
                                    .bg(cx.theme().background)
                                    .shadow_xs()
                                    .hover(|style| {
                                        style
                                            .bg(cx.theme().accent.opacity(0.5))
                                            .border_color(cx.theme().ring.opacity(0.55))
                                    })
                                    .active(|style| style.bg(cx.theme().accent.opacity(0.75)))
                                    .on_click(cx.listener(
                                        move |this, _: &ClickEvent, window, cx| {
                                            let payload = this.recommended_payload(&id);
                                            this.open_tool(&id, payload, window, cx);
                                        },
                                    ))
                                    .child(
                                        h_flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_4()
                                            .child(
                                                h_flex()
                                                    .min_w_0()
                                                    .gap_3()
                                                    .items_center()
                                                    .child(
                                                        div()
                                                            .size(px(40.))
                                                            .flex_none()
                                                            .rounded(cx.theme().radius)
                                                            .bg(cx.theme().primary.opacity(0.1))
                                                            .text_color(cx.theme().primary)
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .child(
                                                                Icon::new(group_icon(tool.group))
                                                                    .size_4(),
                                                            ),
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .min_w_0()
                                                            .gap_1()
                                                            .child(
                                                                div()
                                                                    .font_semibold()
                                                                    .child(tool.display_name),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(
                                                                        cx.theme()
                                                                            .muted_foreground,
                                                                    )
                                                                    .child(
                                                                        tool.group.display_name(),
                                                                    ),
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                h_flex()
                                                    .flex_none()
                                                    .gap_3()
                                                    .items_center()
                                                    .when(recommended, |row| {
                                                        row.child(
                                                            div()
                                                                .px_2()
                                                                .py_1()
                                                                .rounded_full()
                                                                .bg(cx.theme().primary.opacity(0.12))
                                                                .text_xs()
                                                                .font_medium()
                                                                .text_color(cx.theme().primary)
                                                                .child("推荐"),
                                                        )
                                                    })
                                                    .child(
                                                        Icon::new(IconName::ChevronRight)
                                                            .size_4()
                                                            .text_color(
                                                                cx.theme().muted_foreground,
                                                            ),
                                                    ),
                                            ),
                                    )
                            })),
                    )
                }
            })
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = self.state.settings();
        let theme_index = Some(match settings.theme {
            ThemePreference::Light => 0,
            ThemePreference::Dark => 1,
            ThemePreference::System => 2,
        });
        let detection = settings.smart_detection_enabled;
        let paste = settings.smart_detection_paste;
        let show_recent = settings.show_recent;

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_y_scrollbar()
            .p_6()
            .gap_6()
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_lg().font_semibold().child("设置"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("按你的工作习惯调整外观与智能辅助行为"),
                    ),
            )
            .child(
                v_flex()
                    .gap_4()
                    .p_5()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius_lg)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::Palette).size_4())
                            .child(div().font_semibold().child("外观")),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("主题切换会立即应用，并在下次启动时保留。"),
                    )
                    .child(
                        RadioGroup::horizontal("theme")
                            .child(Radio::new("theme-light").label("浅色"))
                            .child(Radio::new("theme-dark").label("深色"))
                            .child(Radio::new("theme-system").label("跟随系统"))
                            .selected_index(theme_index)
                            .on_click(cx.listener(|this, index: &usize, window, cx| {
                                let theme = match index {
                                    0 => ThemePreference::Light,
                                    1 => ThemePreference::Dark,
                                    _ => ThemePreference::System,
                                };
                                this.set_theme(theme, window, cx);
                            })),
                    ),
            )
            .child(
                v_flex()
                    .gap_4()
                    .p_5()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius_lg)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::Settings2).size_4())
                            .child(div().font_semibold().child("行为")),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("控制剪贴板识别、自动填充与最近使用记录。"),
                    )
                    .child(
                        Switch::new("smart-detection")
                            .label("Smart Detection 总开关")
                            .checked(detection)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.set_smart_detection(*checked, cx);
                            })),
                    )
                    .child(
                        Switch::new("smart-paste")
                            .label("推荐工具自动粘贴")
                            .checked(paste)
                            .disabled(!detection)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.set_smart_paste(*checked, cx);
                            })),
                    )
                    .child(
                        Switch::new("show-recent")
                            .label("显示最近使用的工具")
                            .checked(show_recent)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.set_show_recent(*checked, cx);
                            })),
                    ),
            )
    }

    fn render_tool_page(&self, id: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let meta = self.tool_by_id(id);
        let title = meta
            .map(|tool| SharedString::from(tool.display_name))
            .unwrap_or_else(|| SharedString::from(id.to_string()));
        let favorable = meta.map(|tool| tool.favorable).unwrap_or(false);
        let group = meta.map(|tool| tool.group.display_name());
        let favorited = self.state.is_favorite(id);
        let tool_id = id.to_string();

        v_flex()
            .size_full()
            .child(
                h_flex()
                    .px_6()
                    .py_3()
                    .gap_3()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().font_semibold().child(title))
                            .when_some(group, |header, group| {
                                header.child(
                                    div()
                                        .px_2()
                                        .rounded_full()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(group),
                                )
                            }),
                    )
                    .when(favorable, |bar| {
                        bar.child(
                            Button::new("toggle-favorite")
                                .ghost()
                                .compact()
                                .icon(if favorited {
                                    IconName::StarOff
                                } else {
                                    IconName::Star
                                })
                                .label(if favorited { "取消收藏" } else { "收藏" })
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    this.toggle_favorite(&tool_id, cx);
                                })),
                        )
                    }),
            )
            .child(
                div().flex_1().min_h_0().map(|body| match self.sessions.get(id) {
                    Some(handle) => body.child(handle.view.clone()),
                    None => body.child(
                        div()
                            .p_6()
                            .text_color(cx.theme().muted_foreground)
                            .child("此工具暂不可打开"),
                    ),
                }),
            )
    }

    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.page {
            Page::AllTools => self
                .render_tool_list(
                    ALL_TOOLS_LABEL,
                    self.state.registry().all().iter().collect(),
                    "暂无工具",
                    cx,
                )
                .into_any_element(),
            Page::Favorites => self
                .render_tool_list(FAVORITES_LABEL, self.favorite_tools(), "暂无收藏", cx)
                .into_any_element(),
            Page::Group(group) => self
                .render_tool_list(
                    group.display_name(),
                    self.state.registry().in_group(*group),
                    "此分组暂无工具",
                    cx,
                )
                .into_any_element(),
            Page::Tool(id) => self.render_tool_page(id, cx).into_any_element(),
            Page::Settings => self.render_settings(cx).into_any_element(),
        }
    }

    fn render_banner(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if self.recommendations.is_empty() || !self.state.settings().smart_detection_enabled {
            return div().into_any_element();
        }

        h_flex()
            .px_4()
            .py_1()
            .gap_2()
            .items_center()
            .flex_wrap()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().accent.opacity(0.12))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .text_xs()
                    .font_medium()
                    .child(Icon::new(IconName::Bot).size_4())
                    .child("剪贴板建议"),
            )
            .children(self.recommendations.iter().enumerate().map(|(index, hit)| {
                let name = self
                    .tool_by_id(&hit.tool_id)
                    .map(|tool| SharedString::from(tool.display_name))
                    .unwrap_or_else(|| SharedString::from(hit.tool_id.clone()));
                div()
                    .id(SharedString::from(format!("rec-{index}")))
                    .px_2()
                    .cursor_pointer()
                    .rounded(cx.theme().radius)
                    .text_sm()
                    .text_color(cx.theme().primary)
                    .hover(|style| style.bg(cx.theme().accent))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.open_recommendation(index, window, cx);
                    }))
                    .child(
                        h_flex().items_center().child(name),
                    )
            }))
            .into_any_element()
    }
}

fn clipboard_to_raw(item: &gpui::ClipboardItem) -> Option<RawData> {
    if let Some(text) = item.text() {
        if !text.is_empty() {
            return Some(RawData::text(text));
        }
    }
    for entry in item.entries() {
        if let ClipboardEntry::Image(image) = entry {
            if !image.bytes.is_empty() {
                return Some(RawData::Image {
                    bytes: image.bytes.clone(),
                    mime: Some(image.format.mime_type().to_string()),
                });
            }
        }
    }
    None
}

fn group_icon(group: GroupId) -> IconName {
    match group {
        GroupId::Converters => IconName::Replace,
        GroupId::EncodersDecoders => IconName::SquareTerminal,
        GroupId::Formatters => IconName::CaseSensitive,
        GroupId::Generators => IconName::Plus,
        GroupId::Graphic => IconName::Palette,
        GroupId::Testers => IconName::Inspector,
        GroupId::Text => IconName::ALargeSmall,
    }
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                Sidebar::left()
                    .collapsible(false)
                    .w(px(256.))
                    .header(
                        v_flex()
                            .gap_4()
                            .px_1()
                            .py_1()
                            .child(
                                h_flex()
                                    .gap_2p5()
                                    .items_center()
                                    .child(
                                        div()
                                            .size(px(32.))
                                            .flex_none()
                                            .rounded(cx.theme().radius)
                                            .bg(cx.theme().primary)
                                            .text_color(cx.theme().primary_foreground)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(Icon::new(IconName::SquareTerminal).size_4()),
                                    )
                                    .child(div().text_lg().font_semibold().child("DevToys")),
                            )
                            .child(
                                Input::new(&self.search)
                                    .prefix(
                                        Icon::new(IconName::Search)
                                            .size_4()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .cleanable(true)
                                    .w_full(),
                            ),
                    )
                    .child(self.sidebar_menu(cx))
                    .footer(
                        SidebarMenuItem::new("设置")
                            .icon(IconName::Settings)
                            .active(self.page == Page::Settings)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.go(Page::Settings, cx);
                            })),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .size_full()
                    .child(self.render_banner(cx))
                    .child(div().flex_1().min_h_0().child(self.render_content(cx))),
            )
    }
}
