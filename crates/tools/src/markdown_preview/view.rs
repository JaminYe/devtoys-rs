use std::collections::{HashMap, HashSet};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc,
};

use crate::slot::ToolView;
use crate::ui;

use super::helper::{
    highlight_code, load_preview_image_with, parse_markdown, preview_click_dest, Block,
    ImageFetcher, Inline, ListItem, MarkdownDocument, PreviewClickNode, TableAlignment,
    UreqFetcher,
};
use super::markdown_to_html;
use super::ID;

/// Settings JSON `preview_dark`. Unknown / missing → true. Editor/HTML are not persisted.
const DEFAULT_PREVIEW_DARK: bool = true;

const HEADING_SIZES: [f32; 6] = [22.0, 18.0, 16.0, 15.0, 14.0, 13.0];
const LIST_INDENT: f32 = 16.0;

pub struct MarkdownPreviewView {
    editor: String,
    html: String,
    document: MarkdownDocument,
    preview_dark: bool,
    doc_generation: u64,
    images: HashMap<String, CachedImage>,
    image_tx: Sender<ImageLoadResult>,
    image_rx: Receiver<ImageLoadResult>,
    fetcher: Arc<dyn ImageFetcher>,
}

#[derive(Clone)]
pub enum CachedImage {
    Loading { generation: u64 },
    Ready(egui::TextureHandle),
    Failed(String),
}

impl std::fmt::Debug for CachedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loading { generation } => f
                .debug_struct("Loading")
                .field("generation", generation)
                .finish(),
            Self::Ready(tex) => f
                .debug_struct("Ready")
                .field("name", &tex.name())
                .field("size", &tex.size())
                .finish(),
            Self::Failed(err) => f.debug_tuple("Failed").field(err).finish(),
        }
    }
}

#[allow(dead_code)]
impl CachedImage {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading { .. })
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

pub struct ImageLoadResult {
    pub url: String,
    pub generation: u64,
    pub result: Result<(u32, u32, Vec<u8>), String>,
}

impl MarkdownPreviewView {
    pub fn new() -> Self {
        Self::with_fetcher(Arc::new(UreqFetcher))
    }

    pub fn with_fetcher(fetcher: Arc<dyn ImageFetcher>) -> Self {
        let (image_tx, image_rx) = channel();
        Self {
            editor: String::new(),
            html: String::new(),
            document: MarkdownDocument::default(),
            preview_dark: true,
            doc_generation: 0,
            images: HashMap::new(),
            image_tx,
            image_rx,
            fetcher,
        }
    }

    #[allow(dead_code)]
    pub fn images(&self) -> &HashMap<String, CachedImage> {
        &self.images
    }

    #[allow(dead_code)]
    pub fn doc_generation(&self) -> u64 {
        self.doc_generation
    }

    #[allow(dead_code)]
    pub fn image_tx(&self) -> Sender<ImageLoadResult> {
        self.image_tx.clone()
    }

    fn refresh(&mut self) {
        self.doc_generation += 1;
        self.html = markdown_to_html(&self.editor);
        self.document = parse_markdown(&self.editor);
        prune_images(&self.document, &mut self.images);
    }

    pub fn poll_image_results(&mut self, ctx: &egui::Context) {
        let mut repainted = false;
        while let Ok(msg) = self.image_rx.try_recv() {
            if msg.generation != self.doc_generation {
                continue;
            }
            match self.images.get(&msg.url) {
                Some(CachedImage::Loading { generation }) if *generation == msg.generation => {
                    match msg.result {
                        Ok((width, height, rgba)) => {
                            let color = egui::ColorImage::from_rgba_unmultiplied(
                                [width as usize, height as usize],
                                &rgba,
                            );
                            let tex = ctx.load_texture(
                                format!("md-preview-image:{}", msg.url),
                                color,
                                Default::default(),
                            );
                            self.images.insert(msg.url, CachedImage::Ready(tex));
                            if !repainted {
                                ctx.request_repaint();
                                repainted = true;
                            }
                        }
                        Err(err) => {
                            self.images.insert(msg.url, CachedImage::Failed(err));
                            if !repainted {
                                ctx.request_repaint();
                                repainted = true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl Default for MarkdownPreviewView {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Default)]
struct Style {
    strong: bool,
    emphasis: bool,
    strike: bool,
    code: bool,
    size: Option<f32>,
}

struct RenderCtx<'a> {
    images: &'a mut HashMap<String, CachedImage>,
    table_seq: u32,
    #[allow(dead_code)]
    dark: bool,
    fetcher: &'a Arc<dyn ImageFetcher>,
    image_tx: &'a Sender<ImageLoadResult>,
    generation: u64,
}

fn styled(text: impl Into<String>, style: Style) -> egui::RichText {
    let mut rt = egui::RichText::new(text.into());
    if style.strong {
        rt = rt.strong();
    }
    if style.emphasis {
        rt = rt.italics();
    }
    if style.strike {
        rt = rt.strikethrough();
    }
    if style.code {
        rt = rt.monospace();
    }
    if let Some(size) = style.size {
        rt = rt.size(size);
    }
    rt
}

fn render_document(ui: &mut egui::Ui, document: &MarkdownDocument, ctx: &mut RenderCtx<'_>) {
    for (index, block) in document.blocks.iter().enumerate() {
        if index > 0 {
            ui.add_space(6.0);
        }
        render_block(ui, block, ctx, 0);
    }
}

fn render_block(ui: &mut egui::Ui, block: &Block, ctx: &mut RenderCtx<'_>, list_depth: u32) {
    match block {
        Block::Heading { level, children } => {
            let size = HEADING_SIZES[(*level as usize).clamp(1, 6) - 1];
            let style = Style {
                strong: true,
                size: Some(size),
                ..Style::default()
            };
            ui.horizontal_wrapped(|ui| render_inlines(ui, children, style, ctx));
        }
        Block::Paragraph { children } => {
            ui.horizontal_wrapped(|ui| render_inlines(ui, children, Style::default(), ctx));
        }
        Block::List {
            ordered,
            start,
            items,
        } => {
            render_list(ui, *ordered, start.unwrap_or(1), items, ctx, list_depth);
        }
        Block::Table {
            alignments,
            header,
            rows,
        } => {
            render_table(ui, alignments, header, rows, ctx);
        }
        Block::CodeBlock { language, content } => {
            render_code_block(ui, language.as_deref(), content);
        }
        Block::BlockQuote { children } => {
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    for child in children {
                        render_block(ui, child, ctx, list_depth);
                    }
                });
            });
        }
        Block::ThematicBreak => {
            ui.separator();
        }
        Block::Html { content } => {
            ui.label(content);
        }
        Block::FootnoteDefinition { label, children } => {
            ui.label(format!("[^{label}]:"));
            for child in children {
                render_block(ui, child, ctx, list_depth);
            }
        }
    }
}

fn render_list(
    ui: &mut egui::Ui,
    ordered: bool,
    start: u64,
    items: &[ListItem],
    ctx: &mut RenderCtx<'_>,
    depth: u32,
) {
    for (index, item) in items.iter().enumerate() {
        ui.horizontal(|ui| {
            if depth > 0 {
                ui.add_space(LIST_INDENT * depth as f32);
            }
            match item.checked {
                Some(checked) => {
                    let mut checked = checked;
                    ui.add_enabled(false, egui::Checkbox::without_text(&mut checked));
                }
                None if ordered => {
                    ui.label(format!("{}.", start + index as u64));
                }
                None => {
                    ui.label("•");
                }
            }
            ui.vertical(|ui| {
                for child in &item.children {
                    render_block(ui, child, ctx, depth + 1);
                }
            });
        });
    }
}

fn render_table(
    ui: &mut egui::Ui,
    alignments: &[TableAlignment],
    header: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    ctx: &mut RenderCtx<'_>,
) {
    ctx.table_seq += 1;
    let id = ui.id().with("md-table").with(ctx.table_seq);
    let columns = header
        .len()
        .max(rows.iter().map(Vec::len).max().unwrap_or(0))
        .max(alignments.len())
        .max(1);
    egui::Grid::new(id)
        .num_columns(columns)
        .striped(true)
        .min_col_width(48.0)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            if !header.is_empty() {
                let header_style = Style {
                    strong: true,
                    ..Style::default()
                };
                for (index, cell) in header.iter().enumerate() {
                    render_table_cell(ui, cell, header_style, alignment_at(alignments, index), ctx);
                }
                ui.end_row();
            }
            for row in rows {
                for (index, cell) in row.iter().enumerate() {
                    render_table_cell(
                        ui,
                        cell,
                        Style::default(),
                        alignment_at(alignments, index),
                        ctx,
                    );
                }
                ui.end_row();
            }
        });
}

fn alignment_at(alignments: &[TableAlignment], index: usize) -> TableAlignment {
    alignments
        .get(index)
        .copied()
        .unwrap_or(TableAlignment::None)
}

fn render_table_cell(
    ui: &mut egui::Ui,
    cell: &[Inline],
    style: Style,
    alignment: TableAlignment,
    ctx: &mut RenderCtx<'_>,
) {
    let align = match alignment {
        TableAlignment::Right => egui::Align::RIGHT,
        TableAlignment::Center => egui::Align::Center,
        TableAlignment::None | TableAlignment::Left => egui::Align::LEFT,
    };
    ui.with_layout(
        egui::Layout::left_to_right(egui::Align::Center).with_main_align(align),
        |ui| {
            ui.horizontal(|ui| render_inlines(ui, cell, style, ctx));
        },
    );
}

fn render_code_block(ui: &mut egui::Ui, language: Option<&str>, content: &str) {
    let src = content.trim_end();
    let highlighted = language.filter(|token| !token.is_empty()).is_some();
    let fill = if highlighted {
        egui::Color32::from_rgb(0xfb, 0xfb, 0xfb)
    } else {
        ui.visuals().code_bg_color
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(4)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            if highlighted {
                ui.style_mut().visuals.override_text_color = None;
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for span in highlight_code(language, src) {
                        let (r, g, b) = span.color;
                        let color = egui::Color32::from_rgb(r, g, b);
                        for (index, piece) in span.text.split('\n').enumerate() {
                            if index > 0 {
                                ui.label(egui::RichText::new("\n").monospace());
                            }
                            if !piece.is_empty() {
                                ui.label(egui::RichText::new(piece).monospace().color(color));
                            }
                        }
                    }
                });
            } else {
                ui.add(
                    egui::Label::new(egui::RichText::new(src).monospace())
                        .wrap_mode(egui::TextWrapMode::Extend)
                        .selectable(true),
                );
            }
        });
}

fn render_inlines(ui: &mut egui::Ui, inlines: &[Inline], style: Style, ctx: &mut RenderCtx<'_>) {
    for inline in inlines {
        match inline {
            Inline::Text(text) => {
                if !text.is_empty() {
                    ui.label(styled(text.clone(), style));
                }
            }
            Inline::Strong(children) => render_inlines(
                ui,
                children,
                Style {
                    strong: true,
                    ..style
                },
                ctx,
            ),
            Inline::Emphasis(children) => render_inlines(
                ui,
                children,
                Style {
                    emphasis: true,
                    ..style
                },
                ctx,
            ),
            Inline::Strikethrough(children) => render_inlines(
                ui,
                children,
                Style {
                    strike: true,
                    ..style
                },
                ctx,
            ),
            Inline::Code(code) => {
                ui.label(styled(
                    code.clone(),
                    Style {
                        code: true,
                        ..style
                    },
                ));
            }
            Inline::Link { dest, children, .. } => render_link(ui, dest, children, style, ctx),
            Inline::Image { dest, alt, .. } => {
                render_image(ui, dest, alt, style, ctx);
            }
            Inline::SoftBreak => {
                ui.label(" ");
            }
            Inline::HardBreak => {
                ui.label("\n");
            }
            Inline::Html(html) => {
                ui.label(styled(html.clone(), style));
            }
            Inline::FootnoteReference(label) => {
                ui.label(styled(format!("[{label}]"), style));
            }
        }
    }
}

fn render_link(
    ui: &mut egui::Ui,
    dest: &str,
    children: &[Inline],
    style: Style,
    ctx: &mut RenderCtx<'_>,
) {
    if children.is_empty() {
        add_hyperlink(ui, dest, dest, style);
        return;
    }
    render_link_inlines(ui, dest, children, style, ctx);
}

fn render_link_inlines(
    ui: &mut egui::Ui,
    dest: &str,
    inlines: &[Inline],
    style: Style,
    ctx: &mut RenderCtx<'_>,
) {
    for inline in inlines {
        match inline {
            Inline::Text(text) => add_hyperlink(ui, dest, text, style),
            Inline::Code(code) => add_hyperlink(
                ui,
                dest,
                code,
                Style {
                    code: true,
                    ..style
                },
            ),
            Inline::Strong(children) => render_link_inlines(
                ui,
                dest,
                children,
                Style {
                    strong: true,
                    ..style
                },
                ctx,
            ),
            Inline::Emphasis(children) => render_link_inlines(
                ui,
                dest,
                children,
                Style {
                    emphasis: true,
                    ..style
                },
                ctx,
            ),
            Inline::Strikethrough(children) => render_link_inlines(
                ui,
                dest,
                children,
                Style {
                    strike: true,
                    ..style
                },
                ctx,
            ),
            Inline::Link { children, .. } => render_link_inlines(ui, dest, children, style, ctx),
            Inline::Image {
                dest: image, alt, ..
            } => {
                let click = preview_click_dest(PreviewClickNode::LinkedImage {
                    loaded: true,
                    link_dest: dest,
                });
                if !show_cached_image(ui, image, ctx, click, style) {
                    let label = if alt.is_empty() {
                        image.as_str()
                    } else {
                        alt.as_str()
                    };
                    let click = preview_click_dest(PreviewClickNode::LinkedImage {
                        loaded: false,
                        link_dest: dest,
                    });
                    add_hyperlink(ui, click.unwrap_or(dest), label, style);
                }
            }
            Inline::SoftBreak => add_hyperlink(ui, dest, " ", style),
            Inline::HardBreak => {
                ui.label("\n");
            }
            Inline::Html(html) => add_hyperlink(ui, dest, html, style),
            Inline::FootnoteReference(label) => {
                add_hyperlink(ui, dest, &format!("[{label}]"), style)
            }
        }
    }
}

fn add_hyperlink(ui: &mut egui::Ui, dest: &str, text: &str, style: Style) {
    if text.is_empty() {
        return;
    }
    let dest = preview_click_dest(PreviewClickNode::TextLink { dest }).unwrap_or(dest);
    let color = ui.visuals().hyperlink_color;
    ui.add(egui::Hyperlink::from_label_and_url(
        styled(text.to_string(), style).underline().color(color),
        dest.to_string(),
    ));
}

fn open_link_on_click(ui: &mut egui::Ui, response: egui::Response, dest: &str) {
    if dest.is_empty() {
        return;
    }
    if response.clicked_with_open_in_background() {
        ui.open_url(egui::OpenUrl {
            url: dest.to_string(),
            new_tab: true,
        });
    } else if response.clicked() {
        ui.open_url(egui::OpenUrl {
            url: dest.to_string(),
            new_tab: false,
        });
    }
    if response.hovered() {
        ui.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if ui.style().url_in_tooltip {
        response.on_hover_text(dest);
    }
}

fn render_image(ui: &mut egui::Ui, dest: &str, alt: &str, style: Style, ctx: &mut RenderCtx<'_>) {
    let click = preview_click_dest(PreviewClickNode::PlainImage { loaded: true });
    if show_cached_image(ui, dest, ctx, click, style) {
        return;
    }
    let hint = if alt.is_empty() { dest } else { alt };
    let text = if hint.is_empty() {
        "无法加载图片".to_string()
    } else {
        format!("无法加载图片 {hint}")
    };
    ui.label(styled(text, style));
}

fn show_cached_image(
    ui: &mut egui::Ui,
    dest: &str,
    ctx: &mut RenderCtx<'_>,
    click_dest: Option<&str>,
    style: Style,
) -> bool {
    let should_spawn = match ctx.images.get(dest) {
        None => true,
        Some(CachedImage::Loading { generation }) if *generation < ctx.generation => true,
        _ => false,
    };

    if should_spawn {
        ctx.images.insert(
            dest.to_string(),
            CachedImage::Loading {
                generation: ctx.generation,
            },
        );
        let tx = ctx.image_tx.clone();
        let url = dest.to_string();
        let generation = ctx.generation;
        let fetcher = Arc::clone(ctx.fetcher);
        let egui_ctx = ui.ctx().clone();
        std::thread::spawn(move || {
            let res = load_preview_image_with(&url, Some(&*fetcher)).map_err(|e| e.to_string());
            let _ = tx.send(ImageLoadResult {
                url,
                generation,
                result: res,
            });
            egui_ctx.request_repaint();
        });
    }

    match ctx.images.get(dest) {
        Some(CachedImage::Ready(tex)) => {
            let size = tex.size_vec2();
            let mut image = egui::Image::new(tex)
                .fit_to_exact_size(size)
                .maintain_aspect_ratio(true);
            if click_dest.is_some() {
                image = image.sense(egui::Sense::click());
            }
            let response = ui.add(image);
            if let Some(url) = click_dest {
                open_link_on_click(ui, response, url);
            }
            true
        }
        Some(CachedImage::Loading { .. }) => {
            let response = ui
                .horizontal(|ui| {
                    ui.spinner();
                    ui.label(styled(format!("加载图片中: {dest}..."), style));
                })
                .response;
            if let Some(url) = click_dest {
                open_link_on_click(ui, response, url);
            }
            true
        }
        Some(CachedImage::Failed(_)) => false,
        None => false,
    }
}

fn prune_images(document: &MarkdownDocument, cache: &mut HashMap<String, CachedImage>) {
    let mut used = HashSet::new();
    collect_image_dests(&document.blocks, &mut used);
    cache.retain(|dest, _| used.contains(dest));
}

fn collect_image_dests(blocks: &[Block], out: &mut HashSet<String>) {
    for block in blocks {
        match block {
            Block::Heading { children, .. } | Block::Paragraph { children } => {
                collect_inline_images(children, out);
            }
            Block::List { items, .. } => {
                for item in items {
                    collect_image_dests(&item.children, out);
                }
            }
            Block::Table { header, rows, .. } => {
                for cell in header.iter().chain(rows.iter().flatten()) {
                    collect_inline_images(cell, out);
                }
            }
            Block::BlockQuote { children } | Block::FootnoteDefinition { children, .. } => {
                collect_image_dests(children, out);
            }
            Block::CodeBlock { .. } | Block::ThematicBreak | Block::Html { .. } => {}
        }
    }
}

fn collect_inline_images(inlines: &[Inline], out: &mut HashSet<String>) {
    for inline in inlines {
        match inline {
            Inline::Image { dest, .. } => {
                out.insert(dest.clone());
            }
            Inline::Strong(children)
            | Inline::Emphasis(children)
            | Inline::Strikethrough(children)
            | Inline::Link { children, .. } => collect_inline_images(children, out),
            Inline::Text(_)
            | Inline::Code(_)
            | Inline::SoftBreak
            | Inline::HardBreak
            | Inline::Html(_)
            | Inline::FootnoteReference(_) => {}
        }
    }
}

impl ToolView for MarkdownPreviewView {
    fn ui(&mut self, ui: &mut egui::Ui) {
        self.poll_image_results(ui.ctx());
        ui.horizontal_wrapped(|ui| {
            if ui::toggle(ui, self.preview_dark, ui::t("settings.theme_dark")).clicked() {
                self.preview_dark = true;
            }
            if ui::toggle(ui, !self.preview_dark, ui::t("settings.theme_light")).clicked() {
                self.preview_dark = false;
            }
        });
        let spacing = 12.0;
        let total = ui.available_size();
        let w = ((total.x - spacing) / 2.0).max(80.0);
        ui.horizontal(|ui| {
            ui.set_min_height(total.y);
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                if ui::labeled_code(
                    ui,
                    ui::t("common.input"),
                    "md-editor",
                    &mut self.editor,
                    "Markdown",
                    true,
                ) {
                    self.refresh();
                }
            });
            ui.add_space(spacing);
            let preview_dark = self.preview_dark;
            ui.allocate_ui(egui::vec2(w, total.y), |ui| {
                ui.vertical(|ui| {
                    ui.label("预览");
                    let html_reserve = 120.0;
                    let preview_h = (ui.available_height() - html_reserve).max(80.0);
                    ui.allocate_ui(egui::vec2(ui.available_width(), preview_h), |ui| {
                        let fill = if preview_dark {
                            egui::Color32::from_rgb(0x1e, 0x1e, 0x1e)
                        } else {
                            egui::Color32::from_rgb(0xf7, 0xf7, 0xf7)
                        };
                        let text = if preview_dark {
                            egui::Color32::from_rgb(0xe8, 0xe8, 0xe8)
                        } else {
                            egui::Color32::from_rgb(0x22, 0x22, 0x22)
                        };
                        let link = if preview_dark {
                            egui::Color32::from_rgb(0x6e, 0xb6, 0xff)
                        } else {
                            egui::Color32::from_rgb(0x0b, 0x57, 0xd0)
                        };
                        egui::Frame::new()
                            .fill(fill)
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.set_min_size(ui.available_size());
                                ui.style_mut().visuals.override_text_color = Some(text);
                                ui.style_mut().visuals.hyperlink_color = link;
                                egui::ScrollArea::both()
                                    .id_salt("md-preview-scroll")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        let mut ctx = RenderCtx {
                                            images: &mut self.images,
                                            table_seq: 0,
                                            dark: preview_dark,
                                            fetcher: &self.fetcher,
                                            image_tx: &self.image_tx,
                                            generation: self.doc_generation,
                                        };
                                        render_document(ui, &self.document, &mut ctx);
                                    });
                            });
                    });
                    ui::labeled_code(ui, "HTML", "md-html", &mut self.html, "HTML", false);
                });
            });
        });
    }

    fn on_data_received(&mut self, payload: &str) {
        self.editor = payload.to_string();
        self.refresh();
    }

    fn persistable_options(&self) -> Option<(String, serde_json::Value)> {
        Some((
            ID.to_string(),
            serde_json::json!({
                "preview_dark": self.preview_dark,
            }),
        ))
    }

    fn restore_options(&mut self, value: &serde_json::Value) {
        self.preview_dark = value
            .get("preview_dark")
            .and_then(|v| v.as_bool())
            .unwrap_or(DEFAULT_PREVIEW_DARK);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slot::ToolView;

    #[test]
    fn persistable_options_are_preview_dark_only() {
        let mut view = MarkdownPreviewView::new();
        view.preview_dark = false;
        view.editor = "# secret".into();
        view.html = "<h1>secret</h1>".into();
        let (id, value) = view.persistable_options().unwrap();
        assert_eq!(id, ID);
        assert_eq!(value["preview_dark"], false);
        assert!(value.get("editor").is_none());
        assert!(value.get("html").is_none());
        assert!(!value.to_string().contains("secret"));
    }

    #[test]
    fn restore_light_preview_then_html_still_follows_editor() {
        let mut view = MarkdownPreviewView::new();
        view.restore_options(&serde_json::json!({ "preview_dark": false }));
        assert!(!view.preview_dark);
        view.on_data_received("[pic](https://example.com/a.png)");
        assert!(!view.preview_dark);
        assert!(
            view.html.contains("https://example.com/a.png"),
            "{}",
            view.html
        );
        assert!(view.document.blocks.iter().any(|block| matches!(
            block,
            Block::Paragraph { children } if children.iter().any(|inline| {
                matches!(inline, Inline::Link { dest, .. } if dest == "https://example.com/a.png")
            })
        )));
    }

    #[test]
    fn missing_preview_dark_defaults_true() {
        let mut view = MarkdownPreviewView::new();
        view.preview_dark = false;
        view.restore_options(&serde_json::json!({}));
        assert!(view.preview_dark);
    }

    #[test]
    fn test_markdown_preview_i18n_keys() {
        assert_eq!(devtoys_api::t("markdown.title"), "Markdown 预览");
        assert_eq!(devtoys_api::t("settings.theme_dark"), "深色");
        assert_eq!(devtoys_api::t("settings.theme_light"), "浅色");
    }

    fn test_png_bytes() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([12, 34, 56, 255]));
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    struct MockFetcher {
        responses: HashMap<String, Result<Vec<u8>, String>>,
    }

    impl ImageFetcher for MockFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            match self.responses.get(url) {
                Some(Ok(bytes)) => Ok(bytes.clone()),
                Some(Err(err)) => Err(err.clone()),
                None => Err(format!("unexpected url: {url}")),
            }
        }
    }

    fn run_frame(view: &mut MarkdownPreviewView, ctx: &egui::Context) {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            view.ui(ui);
        });
        output.textures_delta.clear();
    }

    #[test]
    fn test_async_image_loader_transitions_to_ready() {
        let url = "https://example.com/ok.png";
        let fetcher = Arc::new(MockFetcher {
            responses: HashMap::from([(url.to_string(), Ok(test_png_bytes()))]),
        });
        let mut view = MarkdownPreviewView::with_fetcher(fetcher);
        view.on_data_received(&format!("![ok]({url})"));

        let ctx = egui::Context::default();

        // Frame 1: triggers background load; image becomes Loading
        run_frame(&mut view, &ctx);
        assert!(view.images().contains_key(url));
        assert!(view.images()[url].is_loading());

        // Wait for background worker to complete
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            view.poll_image_results(&ctx);
            if view.images()[url].is_ready() {
                break;
            }
        }

        // Frame 2: should be Ready with valid texture
        assert!(
            view.images()[url].is_ready(),
            "Image status should transition to Ready"
        );

        // Run another frame, ensuring it renders without panic or re-triggering load
        run_frame(&mut view, &ctx);
        assert!(view.images()[url].is_ready());
    }

    #[test]
    fn test_async_image_loader_transitions_to_failed() {
        let url = "https://example.com/fail.png";
        let fetcher = Arc::new(MockFetcher {
            responses: HashMap::from([(url.to_string(), Err("404 Not Found".to_string()))]),
        });
        let mut view = MarkdownPreviewView::with_fetcher(fetcher);
        view.on_data_received(&format!("![fail]({url})"));

        let ctx = egui::Context::default();

        // Frame 1: triggers load; image becomes Loading
        run_frame(&mut view, &ctx);
        assert!(view.images().contains_key(url));
        assert!(view.images()[url].is_loading());

        // Wait for background worker
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            view.poll_image_results(&ctx);
            if view.images()[url].is_failed() {
                break;
            }
        }

        assert!(
            view.images()[url].is_failed(),
            "Image status should transition to Failed"
        );
        match &view.images()[url] {
            CachedImage::Failed(err) => assert_eq!(err, "无法读取图片"),
            _ => panic!("Expected Failed status"),
        }
    }

    #[test]
    fn test_stale_image_results_ignored_when_generation_advances() {
        let url = "https://example.com/stale.png";
        let mut view = MarkdownPreviewView::new();
        view.on_data_received(&format!("![stale]({url})"));
        let gen0 = view.doc_generation();

        let ctx = egui::Context::default();

        // Simulate document edit/change advancing generation
        view.on_data_received("# Edited content without image");
        let gen1 = view.doc_generation();
        assert!(gen1 > gen0);

        // A stale result from gen0 arrives via channel
        let tx = view.image_tx();
        tx.send(ImageLoadResult {
            url: url.to_string(),
            generation: gen0,
            result: Ok((1, 1, vec![255, 255, 255, 255])),
        })
        .unwrap();

        view.poll_image_results(&ctx);

        // Stale result should NOT be inserted into images
        assert!(!view.images().contains_key(url));
    }

    #[test]
    fn test_prune_images_removes_unreferenced_images() {
        let url_a = "https://example.com/a.png";
        let url_b = "https://example.com/b.png";
        let fetcher = Arc::new(MockFetcher {
            responses: HashMap::from([
                (url_a.to_string(), Ok(test_png_bytes())),
                (url_b.to_string(), Ok(test_png_bytes())),
            ]),
        });
        let mut view = MarkdownPreviewView::with_fetcher(fetcher);
        view.on_data_received(&format!("![a]({url_a})"));

        let ctx = egui::Context::default();
        run_frame(&mut view, &ctx);
        assert!(view.images().contains_key(url_a));

        // Switch to document referencing only b
        view.on_data_received(&format!("![b]({url_b})"));
        assert!(!view.images().contains_key(url_a));
    }

    struct CountingFetcher {
        call_count: std::sync::atomic::AtomicUsize,
        response: Result<Vec<u8>, String>,
    }

    impl ImageFetcher for CountingFetcher {
        fn fetch(&self, _url: &str) -> Result<Vec<u8>, String> {
            self.call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.response.clone()
        }
    }

    #[test]
    fn test_duplicate_requests_not_spawned_while_loading() {
        let fetcher = Arc::new(CountingFetcher {
            call_count: std::sync::atomic::AtomicUsize::new(0),
            response: Ok(test_png_bytes()),
        });
        let mut view = MarkdownPreviewView::with_fetcher(fetcher.clone());
        view.on_data_received("![pic](https://example.com/pic.png)");

        let ctx = egui::Context::default();
        run_frame(&mut view, &ctx);
        assert_eq!(
            fetcher.call_count.load(std::sync::atomic::Ordering::SeqCst),
            1
        );

        // Subsequent frames before result is polled should NOT spawn new fetches
        run_frame(&mut view, &ctx);
        run_frame(&mut view, &ctx);
        assert_eq!(
            fetcher.call_count.load(std::sync::atomic::Ordering::SeqCst),
            1
        );

        // Wait for background worker
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            view.poll_image_results(&ctx);
            if view.images()["https://example.com/pic.png"].is_ready() {
                break;
            }
        }
        assert!(view.images()["https://example.com/pic.png"].is_ready());

        // Further frames after Ready should NOT spawn new fetches
        run_frame(&mut view, &ctx);
        assert_eq!(
            fetcher.call_count.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }

    #[test]
    fn test_stale_result_ignored_when_image_still_in_doc_but_generation_advanced() {
        let url = "https://example.com/keep.png";
        let mut view = MarkdownPreviewView::new();
        view.on_data_received(&format!("![keep]({url})"));
        let gen0 = view.doc_generation();

        let ctx = egui::Context::default();
        run_frame(&mut view, &ctx);
        assert!(view.images()[url].is_loading());

        // Advance document generation (e.g. text edit while image is still in doc)
        view.on_data_received(&format!("Updated text\n\n![keep]({url})"));
        let gen1 = view.doc_generation();
        assert_eq!(gen1, gen0 + 1);

        // Old worker sends result with gen0
        let tx = view.image_tx();
        tx.send(ImageLoadResult {
            url: url.to_string(),
            generation: gen0,
            result: Ok((1, 1, vec![1, 2, 3, 255])),
        })
        .unwrap();

        view.poll_image_results(&ctx);

        // Result with gen0 was discarded! Image should still be Loading from gen0
        assert!(matches!(
            view.images()[url],
            CachedImage::Loading { generation } if generation == gen0
        ));

        // Run frame for gen1: updates image to Loading with gen1
        run_frame(&mut view, &ctx);
        assert!(matches!(
            view.images()[url],
            CachedImage::Loading { generation } if generation == gen1
        ));

        // Now send gen1 result and poll
        tx.send(ImageLoadResult {
            url: url.to_string(),
            generation: gen1,
            result: Ok((1, 1, vec![1, 2, 3, 255])),
        })
        .unwrap();

        view.poll_image_results(&ctx);
        assert!(view.images()[url].is_ready());
    }
}
