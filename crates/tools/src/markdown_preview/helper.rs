use std::path::PathBuf;
use std::sync::OnceLock;

use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, TYPE_TEXT};
use percent_encoding::percent_decode_str;
use pulldown_cmark::{html, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

pub const TYPE_MARKDOWN: &str = "Markdown";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkdownDocument {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: u8,
        children: Vec<Inline>,
    },
    Paragraph {
        children: Vec<Inline>,
    },
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    Table {
        alignments: Vec<TableAlignment>,
        header: TableRow,
        rows: Vec<TableRow>,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    BlockQuote {
        children: Vec<Block>,
    },
    ThematicBreak,
    Html {
        content: String,
    },
    FootnoteDefinition {
        label: String,
        children: Vec<Block>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub children: Vec<Block>,
}

pub type TableRow = Vec<Vec<Inline>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAlignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    Link {
        dest: String,
        title: Option<String>,
        children: Vec<Inline>,
    },
    Image {
        dest: String,
        alt: String,
        title: Option<String>,
    },
    SoftBreak,
    HardBreak,
    Html(String),
    FootnoteReference(String),
}

fn markdown_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options
}

pub fn markdown_to_html(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, markdown_options());
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

pub fn parse_markdown(input: &str) -> MarkdownDocument {
    let mut builder = DocumentBuilder::new();
    for event in Parser::new_ext(input, markdown_options()) {
        builder.push_event(event);
    }
    let doc = builder.finish();
    MarkdownDocument {
        blocks: rewrite_html_blocks(doc.blocks),
    }
}

enum Frame {
    Document(Vec<Block>),
    Quote(Vec<Block>),
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    Item {
        checked: Option<bool>,
        children: Vec<Block>,
    },
    FootnoteDef {
        label: String,
        children: Vec<Block>,
    },
    Heading {
        level: u8,
        children: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Link {
        dest: String,
        title: Option<String>,
        children: Vec<Inline>,
    },
    Image {
        dest: String,
        title: Option<String>,
        children: Vec<Inline>,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    HtmlBlock {
        content: String,
    },
    Table(TableBuilder),
    TableCell(Vec<Inline>),
    TransparentInlines(Vec<Inline>),
    TransparentBlocks(Vec<Block>),
    Sink,
}

struct TableBuilder {
    alignments: Vec<TableAlignment>,
    header: TableRow,
    rows: Vec<TableRow>,
    in_head: bool,
    current_row: TableRow,
}

struct DocumentBuilder {
    stack: Vec<Frame>,
}

impl DocumentBuilder {
    fn new() -> Self {
        Self {
            stack: vec![Frame::Document(Vec::new())],
        }
    }

    fn push_event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.push_text(&text),
            Event::Code(code) => self.push_inline(Inline::Code(code.to_string())),
            Event::Html(html) | Event::InlineHtml(html) => self.push_html(&html),
            Event::FootnoteReference(label) => {
                self.push_inline(Inline::FootnoteReference(label.to_string()))
            }
            Event::SoftBreak => self.push_inline(Inline::SoftBreak),
            Event::HardBreak => self.push_inline(Inline::HardBreak),
            Event::Rule => self.push_block(Block::ThematicBreak),
            Event::TaskListMarker(checked) => self.set_task_checked(checked),
            Event::InlineMath(math) | Event::DisplayMath(math) => {
                self.push_inline(Inline::Code(math.to_string()))
            }
        }
    }

    fn start_tag(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Paragraph => self.stack.push(Frame::Paragraph(Vec::new())),
            Tag::Heading { level, .. } => self.stack.push(Frame::Heading {
                level: level as u8,
                children: Vec::new(),
            }),
            Tag::BlockQuote(_) => self.stack.push(Frame::Quote(Vec::new())),
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.into_string()),
                    _ => None,
                };
                self.stack.push(Frame::CodeBlock {
                    language,
                    content: String::new(),
                });
            }
            Tag::HtmlBlock => self.stack.push(Frame::HtmlBlock {
                content: String::new(),
            }),
            Tag::List(start) => self.stack.push(Frame::List {
                ordered: start.is_some(),
                start,
                items: Vec::new(),
            }),
            Tag::Item => self.stack.push(Frame::Item {
                checked: None,
                children: Vec::new(),
            }),
            Tag::FootnoteDefinition(label) => self.stack.push(Frame::FootnoteDef {
                label: label.into_string(),
                children: Vec::new(),
            }),
            Tag::DefinitionList | Tag::DefinitionListDefinition => {
                self.stack.push(Frame::TransparentBlocks(Vec::new()))
            }
            Tag::DefinitionListTitle => self.stack.push(Frame::Paragraph(Vec::new())),
            Tag::Table(alignments) => self.stack.push(Frame::Table(TableBuilder {
                alignments: alignments.into_iter().map(table_alignment).collect(),
                header: Vec::new(),
                rows: Vec::new(),
                in_head: false,
                current_row: Vec::new(),
            })),
            Tag::TableHead => {
                if let Some(Frame::Table(table)) = self.stack.last_mut() {
                    table.in_head = true;
                }
            }
            Tag::TableRow => {
                if let Some(Frame::Table(table)) = self.stack.last_mut() {
                    table.current_row.clear();
                }
            }
            Tag::TableCell => self.stack.push(Frame::TableCell(Vec::new())),
            Tag::Emphasis => self.stack.push(Frame::Emphasis(Vec::new())),
            Tag::Strong => self.stack.push(Frame::Strong(Vec::new())),
            Tag::Strikethrough => self.stack.push(Frame::Strikethrough(Vec::new())),
            Tag::Superscript | Tag::Subscript => {
                self.stack.push(Frame::TransparentInlines(Vec::new()))
            }
            Tag::Link {
                dest_url, title, ..
            } => self.stack.push(Frame::Link {
                dest: dest_url.into_string(),
                title: nonempty_meta(&title),
                children: Vec::new(),
            }),
            Tag::Image {
                dest_url, title, ..
            } => self.stack.push(Frame::Image {
                dest: dest_url.into_string(),
                title: nonempty_meta(&title),
                children: Vec::new(),
            }),
            Tag::MetadataBlock(_) => self.stack.push(Frame::Sink),
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::TableHead => {
                if let Some(Frame::Table(table)) = self.stack.last_mut() {
                    if !table.current_row.is_empty() {
                        table.header = std::mem::take(&mut table.current_row);
                    }
                    table.in_head = false;
                }
            }
            TagEnd::TableRow => {
                if let Some(Frame::Table(table)) = self.stack.last_mut() {
                    let row = std::mem::take(&mut table.current_row);
                    if table.in_head {
                        table.header = row;
                    } else {
                        table.rows.push(row);
                    }
                }
            }
            _ => {
                if let Some(frame) = self.pop_frame() {
                    self.integrate(frame);
                }
            }
        }
    }

    fn pop_frame(&mut self) -> Option<Frame> {
        if self.stack.len() <= 1 {
            return None;
        }
        self.stack.pop()
    }

    fn integrate(&mut self, frame: Frame) {
        match frame {
            Frame::Document(blocks) | Frame::TransparentBlocks(blocks) => {
                for block in blocks {
                    self.push_block(block);
                }
            }
            Frame::Quote(children) => self.push_block(Block::BlockQuote { children }),
            Frame::List {
                ordered,
                start,
                items,
            } => self.push_block(Block::List {
                ordered,
                start,
                items,
            }),
            Frame::Item { checked, children } => {
                if let Some(Frame::List { items, .. }) = self.stack.last_mut() {
                    items.push(ListItem { checked, children });
                }
            }
            Frame::FootnoteDef { label, children } => {
                self.push_block(Block::FootnoteDefinition { label, children })
            }
            Frame::Heading { level, children } => {
                self.push_block(Block::Heading { level, children })
            }
            Frame::Paragraph(children) => self.push_block(Block::Paragraph { children }),
            Frame::Strong(children) => self.push_inline(Inline::Strong(children)),
            Frame::Emphasis(children) => self.push_inline(Inline::Emphasis(children)),
            Frame::Strikethrough(children) => self.push_inline(Inline::Strikethrough(children)),
            Frame::Link {
                dest,
                title,
                children,
            } => self.push_inline(Inline::Link {
                dest,
                title,
                children,
            }),
            Frame::Image {
                dest,
                title,
                children,
            } => self.push_inline(Inline::Image {
                dest,
                alt: inline_plain_text(&children),
                title,
            }),
            Frame::CodeBlock { language, content } => {
                self.push_block(Block::CodeBlock { language, content })
            }
            Frame::HtmlBlock { content } => self.push_block(Block::Html { content }),
            Frame::Table(table) => self.push_block(Block::Table {
                alignments: table.alignments,
                header: table.header,
                rows: table.rows,
            }),
            Frame::TableCell(inlines) => {
                if let Some(Frame::Table(table)) = self.stack.last_mut() {
                    table.current_row.push(inlines);
                }
            }
            Frame::TransparentInlines(children) => {
                for inline in children {
                    self.push_inline(inline);
                }
            }
            Frame::Sink => {}
        }
    }

    fn push_text(&mut self, text: &str) {
        match self.stack.last_mut() {
            Some(Frame::CodeBlock { content, .. } | Frame::HtmlBlock { content }) => {
                content.push_str(text);
            }
            Some(Frame::Sink) => {}
            _ => self.push_inline(Inline::Text(text.to_string())),
        }
    }

    fn push_html(&mut self, html: &str) {
        match self.stack.last_mut() {
            Some(Frame::HtmlBlock { content } | Frame::CodeBlock { content, .. }) => {
                content.push_str(html);
            }
            Some(Frame::Sink) => {}
            _ => self.push_inline(Inline::Html(html.to_string())),
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(
                Frame::Heading { children, .. }
                | Frame::Paragraph(children)
                | Frame::Strong(children)
                | Frame::Emphasis(children)
                | Frame::Strikethrough(children)
                | Frame::Link { children, .. }
                | Frame::Image { children, .. }
                | Frame::TableCell(children)
                | Frame::TransparentInlines(children),
            ) => {
                children.push(inline);
                return;
            }
            Some(Frame::CodeBlock { content, .. } | Frame::HtmlBlock { content }) => {
                if let Inline::Text(text) | Inline::Html(text) | Inline::Code(text) = inline {
                    content.push_str(&text);
                }
                return;
            }
            Some(Frame::Sink) => return,
            _ => {}
        }
        match self.stack.last_mut() {
            Some(
                Frame::Document(blocks)
                | Frame::Quote(blocks)
                | Frame::Item {
                    children: blocks, ..
                }
                | Frame::FootnoteDef {
                    children: blocks, ..
                }
                | Frame::TransparentBlocks(blocks),
            ) => match blocks.last_mut() {
                Some(Block::Paragraph { children }) => children.push(inline),
                _ => blocks.push(Block::Paragraph {
                    children: vec![inline],
                }),
            },
            _ => self.stack.push(Frame::Paragraph(vec![inline])),
        }
    }

    fn push_block(&mut self, block: Block) {
        match self.stack.last_mut() {
            Some(
                Frame::Document(blocks)
                | Frame::Quote(blocks)
                | Frame::Item {
                    children: blocks, ..
                }
                | Frame::FootnoteDef {
                    children: blocks, ..
                }
                | Frame::TransparentBlocks(blocks),
            ) => blocks.push(block),
            Some(Frame::Sink) => {}
            _ => {
                if !matches!(self.stack.last(), Some(Frame::Document(_))) {
                    self.close_until_blocks();
                    self.push_block(block);
                }
            }
        }
    }

    fn close_until_blocks(&mut self) {
        while !matches!(
            self.stack.last(),
            Some(
                Frame::Document(_)
                    | Frame::Quote(_)
                    | Frame::Item { .. }
                    | Frame::FootnoteDef { .. }
                    | Frame::TransparentBlocks(_)
                    | Frame::Sink
            )
        ) {
            if let Some(frame) = self.pop_frame() {
                self.integrate(frame);
            } else {
                break;
            }
        }
    }

    fn set_task_checked(&mut self, checked: bool) {
        for frame in self.stack.iter_mut().rev() {
            if let Frame::Item { checked: slot, .. } = frame {
                *slot = Some(checked);
                return;
            }
        }
    }

    fn finish(mut self) -> MarkdownDocument {
        while self.stack.len() > 1 {
            if let Some(frame) = self.pop_frame() {
                self.integrate(frame);
            } else {
                break;
            }
        }
        match self.stack.pop() {
            Some(Frame::Document(blocks)) => MarkdownDocument { blocks },
            _ => MarkdownDocument::default(),
        }
    }
}

fn nonempty_meta(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn table_alignment(alignment: pulldown_cmark::Alignment) -> TableAlignment {
    match alignment {
        pulldown_cmark::Alignment::None => TableAlignment::None,
        pulldown_cmark::Alignment::Left => TableAlignment::Left,
        pulldown_cmark::Alignment::Center => TableAlignment::Center,
        pulldown_cmark::Alignment::Right => TableAlignment::Right,
    }
}

pub fn inline_plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    append_plain(inlines, &mut out);
    out
}

fn append_plain(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text(text) | Inline::Code(text) | Inline::Html(text) => out.push_str(text),
            Inline::Strong(children)
            | Inline::Emphasis(children)
            | Inline::Strikethrough(children)
            | Inline::Link { children, .. } => append_plain(children, out),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::SoftBreak => out.push(' '),
            Inline::HardBreak => out.push('\n'),
            Inline::FootnoteReference(_) => {}
        }
    }
}

/// Preview click dest. Load success vs fail does not change dest; tests assert URLs without a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewClickNode<'a> {
    LinkedImage { loaded: bool, link_dest: &'a str },
    PlainImage { loaded: bool },
    TextLink { dest: &'a str },
}

pub fn preview_click_dest(node: PreviewClickNode<'_>) -> Option<&str> {
    match node {
        PreviewClickNode::LinkedImage { link_dest, .. } => nonempty_click_dest(link_dest),
        PreviewClickNode::PlainImage { .. } => None,
        PreviewClickNode::TextLink { dest } => nonempty_click_dest(dest),
    }
}

fn nonempty_click_dest(dest: &str) -> Option<&str> {
    if dest.is_empty() {
        None
    } else {
        Some(dest)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ImagePreviewError {
    #[error("图片地址为空")]
    EmptyDest,
    #[error("不支持远程图片")]
    Remote,
    #[error("无法读取图片")]
    Unreadable,
    #[error("无法解码图片")]
    Decode,
}

pub trait ImageFetcher: Send + Sync {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String>;
}

pub struct UreqFetcher;

impl ImageFetcher for UreqFetcher {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
        use std::io::Read;
        let mut bytes = Vec::new();
        ureq::get(url)
            .timeout(std::time::Duration::from_secs(5))
            .call()
            .map_err(|err| err.to_string())?
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|err| err.to_string())?;
        Ok(bytes)
    }
}

pub fn load_preview_image(dest: &str) -> Result<(u32, u32, Vec<u8>), ImagePreviewError> {
    load_preview_image_with(dest, None)
}

pub fn load_preview_image_with(
    dest: &str,
    fetcher: Option<&dyn ImageFetcher>,
) -> Result<(u32, u32, Vec<u8>), ImagePreviewError> {
    let dest = dest.trim();
    if dest.is_empty() {
        return Err(ImagePreviewError::EmptyDest);
    }
    let bytes = if is_http_url(dest) {
        let Some(fetcher) = fetcher else {
            return Err(ImagePreviewError::Remote);
        };
        fetcher
            .fetch(dest)
            .map_err(|_| ImagePreviewError::Unreadable)?
    } else {
        let path = local_image_path(dest)?;
        std::fs::read(&path).map_err(|_| ImagePreviewError::Unreadable)?
    };
    decode_image_bytes(&bytes)
}

fn decode_image_bytes(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), ImagePreviewError> {
    let img = image::load_from_memory(bytes).map_err(|_| ImagePreviewError::Decode)?;
    let rgba = img.into_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((width, height, rgba.into_raw()))
}

fn is_http_url(dest: &str) -> bool {
    match dest.find("://") {
        Some(idx) => {
            let scheme = &dest[..idx];
            scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
        }
        None => false,
    }
}

fn local_image_path(dest: &str) -> Result<PathBuf, ImagePreviewError> {
    let dest = dest.trim();
    if dest.is_empty() {
        return Err(ImagePreviewError::EmptyDest);
    }
    if let Some(scheme_end) = dest.find("://") {
        let scheme = &dest[..scheme_end];
        if scheme.eq_ignore_ascii_case("file") {
            return file_url_to_path(dest);
        }
        if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
            return Err(ImagePreviewError::Remote);
        }
        return Err(ImagePreviewError::Unreadable);
    }
    Ok(PathBuf::from(dest))
}

fn file_url_to_path(url: &str) -> Result<PathBuf, ImagePreviewError> {
    let Some(body) = url.get(5..) else {
        return Err(ImagePreviewError::Unreadable);
    };
    let raw = if let Some(after) = body.strip_prefix("//") {
        match after.find('/') {
            Some(slash) => {
                let host = &after[..slash];
                if !host.is_empty() && !host.eq_ignore_ascii_case("localhost") {
                    return Err(ImagePreviewError::Unreadable);
                }
                &after[slash..]
            }
            None => after,
        }
    } else {
        body
    };
    let decoded = percent_decode_str(raw)
        .decode_utf8()
        .map_err(|_| ImagePreviewError::Unreadable)?;
    Ok(native_file_path(decoded.as_ref()))
}

fn native_file_path(path: &str) -> PathBuf {
    #[cfg(windows)]
    {
        let bytes = path.as_bytes();
        if bytes.len() >= 3 && bytes[0] == b'/' && bytes[2] == b':' {
            return PathBuf::from(&path[1..]);
        }
    }
    PathBuf::from(path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub text: String,
    pub color: (u8, u8, u8),
}

const PLAIN_HIGHLIGHT: (u8, u8, u8) = (0x24, 0x29, 0x2f);

pub fn highlight_code(lang: Option<&str>, src: &str) -> Vec<HighlightSpan> {
    let lang = lang.map(str::trim).filter(|token| !token.is_empty());
    let Some(lang) = lang else {
        return plain_highlight(src);
    };
    let Some(syntax) = syntax_for(lang) else {
        return plain_highlight(src);
    };
    let ss = syntax_set();
    let mut highlighter = syntect::easy::HighlightLines::new(syntax, highlight_theme());
    let mut spans = Vec::new();
    for line in syntect::util::LinesWithEndings::from(src) {
        match highlighter.highlight_line(line, ss) {
            Ok(ranges) => {
                for (style, text) in ranges {
                    if text.is_empty() {
                        continue;
                    }
                    let fg = style.foreground;
                    spans.push(HighlightSpan {
                        text: text.to_string(),
                        color: (fg.r, fg.g, fg.b),
                    });
                }
            }
            Err(_) => spans.push(HighlightSpan {
                text: line.to_string(),
                color: PLAIN_HIGHLIGHT,
            }),
        }
    }
    if spans.is_empty() {
        plain_highlight(src)
    } else {
        spans
    }
}

fn plain_highlight(src: &str) -> Vec<HighlightSpan> {
    vec![HighlightSpan {
        text: src.to_string(),
        color: PLAIN_HIGHLIGHT,
    }]
}

fn syntax_set() -> &'static syntect::parsing::SyntaxSet {
    static SET: OnceLock<syntect::parsing::SyntaxSet> = OnceLock::new();
    SET.get_or_init(syntect::parsing::SyntaxSet::load_defaults_newlines)
}

fn highlight_theme() -> &'static syntect::highlighting::Theme {
    static THEME: OnceLock<syntect::highlighting::Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let mut themes = syntect::highlighting::ThemeSet::load_defaults();
        if let Some(theme) = themes.themes.remove("InspiredGitHub") {
            theme
        } else {
            themes
                .themes
                .into_values()
                .next()
                .expect("syntect default themes")
        }
    })
}

fn syntax_for(lang: &str) -> Option<&'static syntect::parsing::SyntaxReference> {
    let token = lang.split_whitespace().next().unwrap_or(lang);
    syntax_set().find_syntax_by_token(token)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenKind {
    Strong,
    Emphasis,
}

enum HtmlToken {
    Open(String),
    Close(String),
    Void(String),
    Text(String),
}

fn rewrite_html_blocks(blocks: Vec<Block>) -> Vec<Block> {
    let mut out = Vec::with_capacity(blocks.len());
    for block in blocks {
        match block {
            Block::Html { content } => out.extend(html_to_blocks(&content)),
            Block::Paragraph { children } => out.push(Block::Paragraph {
                children: rewrite_inlines(children),
            }),
            Block::Heading { level, children } => out.push(Block::Heading {
                level,
                children: rewrite_inlines(children),
            }),
            Block::List {
                ordered,
                start,
                items,
            } => out.push(Block::List {
                ordered,
                start,
                items: items
                    .into_iter()
                    .map(|item| ListItem {
                        checked: item.checked,
                        children: rewrite_html_blocks(item.children),
                    })
                    .collect(),
            }),
            Block::Table {
                alignments,
                header,
                rows,
            } => out.push(Block::Table {
                alignments,
                header: header.into_iter().map(rewrite_inlines).collect(),
                rows: rows
                    .into_iter()
                    .map(|row| row.into_iter().map(rewrite_inlines).collect())
                    .collect(),
            }),
            Block::BlockQuote { children } => out.push(Block::BlockQuote {
                children: rewrite_html_blocks(children),
            }),
            Block::FootnoteDefinition { label, children } => out.push(Block::FootnoteDefinition {
                label,
                children: rewrite_html_blocks(children),
            }),
            other => out.push(other),
        }
    }
    out
}

fn rewrite_inlines(inlines: Vec<Inline>) -> Vec<Inline> {
    let mut buf = InlineBuf::new();
    for inline in inlines {
        match rewrite_inline_node(inline) {
            Inline::Html(html) => {
                for token in tokenize_html(&html) {
                    buf.apply_token(token);
                }
            }
            other => buf.push_inline(other),
        }
    }
    buf.take()
}

fn rewrite_inline_node(inline: Inline) -> Inline {
    match inline {
        Inline::Strong(children) => Inline::Strong(rewrite_inlines(children)),
        Inline::Emphasis(children) => Inline::Emphasis(rewrite_inlines(children)),
        Inline::Strikethrough(children) => Inline::Strikethrough(rewrite_inlines(children)),
        Inline::Link {
            dest,
            title,
            children,
        } => Inline::Link {
            dest,
            title,
            children: rewrite_inlines(children),
        },
        other => other,
    }
}

fn html_to_blocks(html: &str) -> Vec<Block> {
    let mut doc = HtmlDoc {
        out: Vec::new(),
        lists: Vec::new(),
        inlines: InlineBuf::new(),
    };
    for token in tokenize_html(html) {
        doc.push_token(token);
    }
    doc.finish()
}

struct HtmlDoc {
    out: Vec<Block>,
    lists: Vec<HtmlList>,
    inlines: InlineBuf,
}

struct HtmlList {
    ordered: bool,
    items: Vec<ListItem>,
    current: Option<Vec<Block>>,
}

impl HtmlDoc {
    fn push_token(&mut self, token: HtmlToken) {
        if self.inlines.skip.is_some() {
            self.inlines.apply_token(token);
            return;
        }
        match token {
            HtmlToken::Open(name) if name == "p" => self.flush_inlines(),
            HtmlToken::Close(name) if name == "p" => self.flush_inlines(),
            HtmlToken::Open(name) if name == "ul" || name == "ol" => {
                self.flush_inlines();
                self.lists.push(HtmlList {
                    ordered: name == "ol",
                    items: Vec::new(),
                    current: None,
                });
            }
            HtmlToken::Close(name) if name == "ul" || name == "ol" => {
                self.flush_inlines();
                self.close_li();
                if let Some(list) = self.lists.pop() {
                    let ordered = list.ordered;
                    self.push_block(Block::List {
                        ordered,
                        start: ordered.then_some(1),
                        items: list.items,
                    });
                }
            }
            HtmlToken::Open(name) if name == "li" => {
                self.flush_inlines();
                if self.lists.is_empty() {
                    self.lists.push(HtmlList {
                        ordered: false,
                        items: Vec::new(),
                        current: None,
                    });
                }
                self.close_li();
                if let Some(list) = self.lists.last_mut() {
                    list.current = Some(Vec::new());
                }
            }
            HtmlToken::Close(name) if name == "li" => {
                self.flush_inlines();
                self.close_li();
            }
            HtmlToken::Void(name) if name == "hr" => {
                self.flush_inlines();
                self.push_block(Block::ThematicBreak);
            }
            other => self.inlines.apply_token(other),
        }
    }

    fn flush_inlines(&mut self) {
        let children = self.inlines.take();
        if children.is_empty() {
            return;
        }
        self.push_block(Block::Paragraph { children });
    }

    fn close_li(&mut self) {
        if let Some(list) = self.lists.last_mut() {
            if let Some(children) = list.current.take() {
                list.items.push(ListItem {
                    checked: None,
                    children,
                });
            }
        }
    }

    fn push_block(&mut self, block: Block) {
        if let Some(list) = self.lists.last_mut() {
            if let Some(children) = &mut list.current {
                children.push(block);
                return;
            }
        }
        self.out.push(block);
    }

    fn finish(mut self) -> Vec<Block> {
        self.flush_inlines();
        while !self.lists.is_empty() {
            self.close_li();
            let list = self.lists.pop().expect("list stack is not empty");
            let ordered = list.ordered;
            self.push_block(Block::List {
                ordered,
                start: ordered.then_some(1),
                items: list.items,
            });
        }
        self.out
    }
}

struct InlineBuf {
    stack: Vec<(Option<OpenKind>, Vec<Inline>)>,
    skip: Option<String>,
}

impl InlineBuf {
    fn new() -> Self {
        Self {
            stack: vec![(None, Vec::new())],
            skip: None,
        }
    }

    fn push_inline(&mut self, inline: Inline) {
        if self.skip.is_some() {
            return;
        }
        self.stack.last_mut().expect("inline stack").1.push(inline);
    }

    fn current_is_empty(&self) -> bool {
        self.stack.len() == 1 && self.stack[0].1.is_empty()
    }

    fn apply_token(&mut self, token: HtmlToken) {
        if let Some(skip) = self.skip.as_deref() {
            if let HtmlToken::Close(name) = &token {
                if name == skip {
                    self.skip = None;
                }
            }
            return;
        }
        match token {
            HtmlToken::Open(name) if name == "script" || name == "style" => {
                self.skip = Some(name);
            }
            HtmlToken::Open(name) if name == "b" || name == "strong" => {
                self.stack.push((Some(OpenKind::Strong), Vec::new()));
            }
            HtmlToken::Open(name) if name == "i" || name == "em" => {
                self.stack.push((Some(OpenKind::Emphasis), Vec::new()));
            }
            HtmlToken::Close(name) if name == "b" || name == "strong" => {
                self.close_kind(OpenKind::Strong);
            }
            HtmlToken::Close(name) if name == "i" || name == "em" => {
                self.close_kind(OpenKind::Emphasis);
            }
            HtmlToken::Void(name) if name == "br" => self.push_inline(Inline::HardBreak),
            HtmlToken::Text(text) => {
                if text.is_empty() {
                    return;
                }
                if text.chars().all(char::is_whitespace) && self.current_is_empty() {
                    return;
                }
                self.push_inline(Inline::Text(text));
            }
            _ => {}
        }
    }

    fn close_kind(&mut self, want: OpenKind) {
        if !self.stack.iter().any(|(kind, _)| *kind == Some(want)) {
            return;
        }
        while self.stack.len() > 1 {
            let kind = self.stack.last().and_then(|(kind, _)| *kind);
            self.close_top();
            if kind == Some(want) {
                break;
            }
        }
    }

    fn close_top(&mut self) {
        let (kind, children) = self.stack.pop().expect("inline stack");
        let dest = &mut self.stack.last_mut().expect("inline stack").1;
        match kind {
            Some(OpenKind::Strong) => dest.push(Inline::Strong(children)),
            Some(OpenKind::Emphasis) => dest.push(Inline::Emphasis(children)),
            None => dest.extend(children),
        }
    }

    fn take(&mut self) -> Vec<Inline> {
        while self.stack.len() > 1 {
            self.close_top();
        }
        std::mem::take(&mut self.stack[0].1)
    }
}

fn tokenize_html(input: &str) -> Vec<HtmlToken> {
    let mut out = Vec::new();
    let mut rest = input;
    while !rest.is_empty() {
        if let Some(stripped) = rest.strip_prefix("<!--") {
            if let Some(end) = stripped.find("-->") {
                rest = &stripped[end + 3..];
                continue;
            }
        }
        if rest.starts_with('<') {
            if let Some(end) = rest.find('>') {
                let inner = &rest[1..end];
                rest = &rest[end + 1..];
                if let Some(token) = classify_tag(inner) {
                    out.push(token);
                }
                continue;
            }
            out.push(HtmlToken::Text(rest.to_string()));
            break;
        }
        let split = rest.find('<').unwrap_or(rest.len());
        let (text, next) = rest.split_at(split);
        out.push(HtmlToken::Text(text.to_string()));
        rest = next;
    }
    out
}

fn classify_tag(inner: &str) -> Option<HtmlToken> {
    let s = inner.trim();
    if s.is_empty() || s.starts_with('!') || s.starts_with('?') {
        return None;
    }
    let is_close = s.starts_with('/');
    let s = if is_close { s[1..].trim_start() } else { s };
    let is_void_slash = s.ends_with('/');
    let s = if is_void_slash {
        s[..s.len() - 1].trim_end()
    } else {
        s
    };
    let name_end = s
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(s.len());
    if name_end == 0 {
        return None;
    }
    let name = s[..name_end].to_ascii_lowercase();
    if is_close {
        Some(HtmlToken::Close(name))
    } else if is_void_slash || is_void_element(&name) {
        Some(HtmlToken::Void(name))
    } else {
        Some(HtmlToken::Open(name))
    }
}

fn is_void_element(name: &str) -> bool {
    matches!(name, "br" | "hr" | "img" | "input" | "meta" | "link")
}

pub fn looks_like_markdown(text: &str) -> bool {
    text.lines().any(|line| {
        let t = line.trim_start();
        t.starts_with("# ") || t.starts_with("```") || t.starts_with("- ")
    })
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownDetector;

impl Detector for MarkdownDetector {
    fn data_type(&self) -> DataTypeSpec {
        DataTypeSpec {
            name: TYPE_MARKDOWN,
            parent: Some(TYPE_TEXT),
        }
    }

    fn detect(&self, _raw: &RawData, parent: Option<&DetectedPayload>) -> Option<DetectedPayload> {
        let parent = parent?;
        let value = parent.value.as_str();
        if !looks_like_markdown(value) {
            return None;
        }
        Some(DetectedPayload::new(TYPE_MARKDOWN, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parent(text: &str) -> DetectedPayload {
        DetectedPayload::new(TYPE_TEXT, text)
    }

    fn paragraph_inlines(doc: &MarkdownDocument) -> &[Inline] {
        match doc.blocks.as_slice() {
            [Block::Paragraph { children }] => children,
            other => panic!("expected a single paragraph, got {other:?}"),
        }
    }

    fn find_inline(inlines: &[Inline], pred: impl Fn(&Inline) -> bool + Copy) -> bool {
        inlines.iter().any(|inline| {
            if pred(inline) {
                return true;
            }
            match inline {
                Inline::Strong(children)
                | Inline::Emphasis(children)
                | Inline::Strikethrough(children)
                | Inline::Link { children, .. } => find_inline(children, pred),
                _ => false,
            }
        })
    }

    #[test]
    fn heading_html_contains_h1() {
        let html = markdown_to_html("# Hi");
        assert!(html.contains("<h1"), "html was {html}");
    }

    #[test]
    fn parse_heading_level_1_text() {
        let doc = parse_markdown("# Hi");
        match doc.blocks.as_slice() {
            [Block::Heading { level, children }] => {
                assert_eq!(*level, 1);
                assert_eq!(inline_plain_text(children), "Hi");
            }
            other => panic!("expected h1, got {other:?}"),
        }
    }

    #[test]
    fn parse_bold_italic_strike_and_code() {
        let bold = parse_markdown("**bold**");
        match paragraph_inlines(&bold) {
            [Inline::Strong(children)] => assert_eq!(inline_plain_text(children), "bold"),
            other => panic!("expected strong, got {other:?}"),
        }

        let em = parse_markdown("*em*");
        match paragraph_inlines(&em) {
            [Inline::Emphasis(children)] => assert_eq!(inline_plain_text(children), "em"),
            other => panic!("expected emphasis, got {other:?}"),
        }

        let strike = parse_markdown("~~strike~~");
        match paragraph_inlines(&strike) {
            [Inline::Strikethrough(children)] => {
                assert_eq!(inline_plain_text(children), "strike")
            }
            other => panic!("expected strikethrough, got {other:?}"),
        }

        let code = parse_markdown("`code`");
        match paragraph_inlines(&code) {
            [Inline::Code(text)] => assert_eq!(text, "code"),
            other => panic!("expected code, got {other:?}"),
        }
    }

    #[test]
    fn parse_nested_bold_and_italic() {
        let doc = parse_markdown("**bold *and* italic**");
        match paragraph_inlines(&doc) {
            [Inline::Strong(children)] => {
                assert!(find_inline(children, |i| matches!(i, Inline::Emphasis(_))));
                assert_eq!(inline_plain_text(children), "bold and italic");
            }
            other => panic!("expected strong wrapping nested emphasis, got {other:?}"),
        }
    }

    #[test]
    fn parse_link_keeps_dest_and_text() {
        let doc = parse_markdown("[txt](https://example.com)");
        match paragraph_inlines(&doc) {
            [Inline::Link {
                dest,
                title,
                children,
            }] => {
                assert_eq!(dest, "https://example.com");
                assert_eq!(title, &None);
                assert_eq!(inline_plain_text(children), "txt");
            }
            other => panic!("expected link, got {other:?}"),
        }
    }

    #[test]
    fn parse_multi_paragraph_yields_multiple_blocks() {
        let doc = parse_markdown("one\n\ntwo");
        let texts: Vec<String> = doc
            .blocks
            .iter()
            .map(|block| match block {
                Block::Paragraph { children } => inline_plain_text(children),
                other => panic!("expected paragraphs, got {other:?}"),
            })
            .collect();
        assert_eq!(texts, ["one", "two"]);
    }

    #[test]
    fn parse_common_inline_nesting() {
        let combo = parse_markdown("***bold italic***");
        let inlines = paragraph_inlines(&combo);
        assert!(find_inline(inlines, |i| matches!(i, Inline::Strong(_))));
        assert!(find_inline(inlines, |i| matches!(i, Inline::Emphasis(_))));
        assert_eq!(inline_plain_text(inlines), "bold italic");

        let code = parse_markdown("**bold `code`**");
        match paragraph_inlines(&code) {
            [Inline::Strong(children)] => {
                assert!(children
                    .iter()
                    .any(|inline| matches!(inline, Inline::Code(text) if text == "code")));
                assert_eq!(inline_plain_text(children), "bold code");
            }
            other => panic!("expected code nested in strong, got {other:?}"),
        }
    }

    fn walk_blocks(blocks: &[Block], pred: impl Fn(&Block) -> bool + Copy) -> bool {
        blocks.iter().any(|block| {
            if pred(block) {
                return true;
            }
            match block {
                Block::List { items, .. } => {
                    items.iter().any(|item| walk_blocks(&item.children, pred))
                }
                Block::BlockQuote { children } | Block::FootnoteDefinition { children, .. } => {
                    walk_blocks(children, pred)
                }
                _ => false,
            }
        })
    }

    fn walk_inlines(blocks: &[Block], pred: impl Fn(&Inline) -> bool + Copy) -> bool {
        fn inlines(items: &[Inline], pred: impl Fn(&Inline) -> bool + Copy) -> bool {
            items.iter().any(|inline| {
                if pred(inline) {
                    return true;
                }
                match inline {
                    Inline::Strong(children)
                    | Inline::Emphasis(children)
                    | Inline::Strikethrough(children)
                    | Inline::Link { children, .. } => inlines(children, pred),
                    _ => false,
                }
            })
        }
        walk_blocks(blocks, |block| match block {
            Block::Heading { children, .. } | Block::Paragraph { children } => {
                inlines(children, pred)
            }
            Block::Table { header, rows, .. } => header
                .iter()
                .chain(rows.iter().flatten())
                .any(|cell| inlines(cell, pred)),
            _ => false,
        })
    }

    #[test]
    fn parse_two_list_items() {
        let doc = parse_markdown("- a\n- b");
        match doc.blocks.as_slice() {
            [Block::List { items, ordered, .. }] => {
                assert!(!*ordered);
                assert_eq!(items.len(), 2, "{items:?}");
                assert_eq!(
                    items
                        .iter()
                        .map(|item| item
                            .children
                            .iter()
                            .map(|block| match block {
                                Block::Paragraph { children } => inline_plain_text(children),
                                other => panic!("expected paragraph in item, got {other:?}"),
                            })
                            .collect::<Vec<_>>()
                            .join(""))
                        .collect::<Vec<_>>(),
                    ["a", "b"]
                );
            }
            other => panic!("expected list, got {other:?}"),
        }
    }

    #[test]
    fn parse_list_table_code_image_for_ticket_07() {
        let doc = parse_markdown(
            "- a\n- b\n\n| h |\n| --- |\n| c |\n\n```\ncode\n```\n\n![alt](https://ex.com/x.png)",
        );
        assert!(
            walk_blocks(
                &doc.blocks,
                |block| matches!(block, Block::List { items, .. } if items.len() == 2)
            ),
            "expected two-item list: {:?}",
            doc.blocks
        );
        assert!(
            walk_blocks(&doc.blocks, |block| matches!(
                block,
                Block::Table { header, rows, .. }
                    if inline_plain_text(&header.first().cloned().unwrap_or_default()) == "h"
                        && rows.len() == 1
            )),
            "expected table: {:?}",
            doc.blocks
        );
        assert!(
            walk_blocks(&doc.blocks, |block| matches!(
                block,
                Block::CodeBlock { content, .. } if content.contains("code")
            )),
            "expected code block: {:?}",
            doc.blocks
        );
        assert!(
            walk_inlines(&doc.blocks, |inline| matches!(
                inline,
                Inline::Image { dest, alt, .. }
                    if dest == "https://ex.com/x.png" && alt == "alt"
            )),
            "expected image: {:?}",
            doc.blocks
        );
    }

    fn list_item_text(item: &ListItem) -> String {
        item.children
            .iter()
            .filter_map(|block| match block {
                Block::Paragraph { children } => Some(inline_plain_text(children)),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn nested_list(item: &ListItem) -> &Block {
        item.children
            .iter()
            .find(|block| matches!(block, Block::List { .. }))
            .unwrap_or_else(|| panic!("expected nested list in {item:?}"))
    }

    #[test]
    fn parse_table_two_by_two_cells() {
        let doc = parse_markdown("| A | B |\n| --- | --- |\n| C | D |\n");
        match doc.blocks.as_slice() {
            [Block::Table { header, rows, .. }] => {
                assert_eq!(header.len(), 2, "{header:?}");
                assert_eq!(inline_plain_text(&header[0]), "A");
                assert_eq!(inline_plain_text(&header[1]), "B");
                assert_eq!(rows.len(), 1, "{rows:?}");
                assert_eq!(rows[0].len(), 2, "{rows:?}");
                assert_eq!(inline_plain_text(&rows[0][0]), "C");
                assert_eq!(inline_plain_text(&rows[0][1]), "D");
            }
            other => panic!("expected 2x2 table, got {other:?}"),
        }
    }

    #[test]
    fn parse_nested_list_outer_contains_inner() {
        let doc = parse_markdown("- outer\n  1. inner\n");
        match doc.blocks.as_slice() {
            [Block::List { ordered, items, .. }] => {
                assert!(!*ordered);
                assert_eq!(items.len(), 1, "{items:?}");
                assert_eq!(list_item_text(&items[0]), "outer");
                match nested_list(&items[0]) {
                    Block::List {
                        ordered,
                        start,
                        items,
                    } => {
                        assert!(*ordered);
                        assert_eq!(*start, Some(1));
                        assert_eq!(items.len(), 1, "{items:?}");
                        assert_eq!(list_item_text(&items[0]), "inner");
                    }
                    other => panic!("expected inner list, got {other:?}"),
                }
            }
            other => panic!("expected nested list, got {other:?}"),
        }
    }

    #[test]
    fn parse_task_list_checked_states() {
        let doc = parse_markdown("- [x] done\n- [ ] todo\n");
        match doc.blocks.as_slice() {
            [Block::List { items, .. }] => {
                assert_eq!(items.len(), 2, "{items:?}");
                assert_eq!(items[0].checked, Some(true));
                assert_eq!(list_item_text(&items[0]), "done");
                assert_eq!(items[1].checked, Some(false));
                assert_eq!(list_item_text(&items[1]), "todo");
            }
            other => panic!("expected task list, got {other:?}"),
        }
    }

    #[test]
    fn parse_code_block_keeps_indent_and_raw_markers() {
        let doc = parse_markdown("```\n  **not strong**\n```\n");
        match doc.blocks.as_slice() {
            [Block::CodeBlock { content, .. }] => {
                assert_eq!(content.trim_end(), "  **not strong**", "{content:?}");
                assert!(content.starts_with("  "), "{content:?}");
            }
            other => panic!("expected code block, got {other:?}"),
        }
        assert!(
            !walk_inlines(&doc.blocks, |inline| matches!(inline, Inline::Strong(_))),
            "code content must not become Strong: {:?}",
            doc.blocks
        );
    }

    #[test]
    fn parse_image_dest_and_alt() {
        let doc = parse_markdown("![logo](./pic.png \"title\")");
        match paragraph_inlines(&doc) {
            [Inline::Image { dest, alt, title }] => {
                assert_eq!(dest, "./pic.png");
                assert_eq!(alt, "logo");
                assert_eq!(title.as_deref(), Some("title"));
            }
            other => panic!("expected image, got {other:?}"),
        }
    }

    #[test]
    fn linked_image_binds_outer_url_on_success_and_fallback() {
        let dest = "https://example.com";
        assert_eq!(
            preview_click_dest(PreviewClickNode::LinkedImage {
                loaded: true,
                link_dest: dest,
            }),
            Some(dest)
        );
        assert_eq!(
            preview_click_dest(PreviewClickNode::LinkedImage {
                loaded: false,
                link_dest: dest,
            }),
            Some(dest)
        );
    }

    #[test]
    fn plain_image_has_no_click_dest() {
        assert_eq!(
            preview_click_dest(PreviewClickNode::PlainImage { loaded: true }),
            None
        );
        assert_eq!(
            preview_click_dest(PreviewClickNode::PlainImage { loaded: false }),
            None
        );
    }

    #[test]
    fn text_link_binds_its_dest() {
        assert_eq!(
            preview_click_dest(PreviewClickNode::TextLink {
                dest: "https://example.com",
            }),
            Some("https://example.com")
        );
        assert_eq!(
            preview_click_dest(PreviewClickNode::TextLink { dest: "" }),
            None
        );
    }

    fn first_image(inlines: &[Inline]) -> Option<&Inline> {
        inlines.iter().find_map(|inline| match inline {
            Inline::Image { .. } => Some(inline),
            Inline::Strong(children)
            | Inline::Emphasis(children)
            | Inline::Strikethrough(children)
            | Inline::Link { children, .. } => first_image(children),
            _ => None,
        })
    }

    fn linked_image(inlines: &[Inline]) -> Option<(&str, &Inline)> {
        inlines.iter().find_map(|inline| match inline {
            Inline::Link { dest, children, .. } => {
                first_image(children).map(|image| (dest.as_str(), image))
            }
            Inline::Strong(children)
            | Inline::Emphasis(children)
            | Inline::Strikethrough(children) => linked_image(children),
            _ => None,
        })
    }

    #[test]
    fn nested_image_link_keeps_outer_dest_for_click() {
        let dest = "https://example.com";
        for markdown in [
            format!("[![alt](./pic.png)]({dest})"),
            format!("[**![alt](./pic.png)**]({dest})"),
        ] {
            let doc = parse_markdown(&markdown);
            let (link_dest, image) = linked_image(paragraph_inlines(&doc))
                .unwrap_or_else(|| panic!("expected linked image, got {:?}", doc.blocks));
            assert_eq!(link_dest, dest);
            match image {
                Inline::Image { dest: img, alt, .. } => {
                    assert_eq!(img, "./pic.png");
                    assert_eq!(alt, "alt");
                }
                other => panic!("expected image, got {other:?}"),
            }
            assert_eq!(
                preview_click_dest(PreviewClickNode::LinkedImage {
                    loaded: true,
                    link_dest,
                }),
                Some(dest)
            );
            assert_eq!(
                preview_click_dest(PreviewClickNode::LinkedImage {
                    loaded: false,
                    link_dest,
                }),
                Some(dest)
            );
        }
    }

    fn write_temp_png() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "devtoys-md-preview-{}-{}.png",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([10, 20, 30, 255]));
        img.save(&path).unwrap();
        path
    }

    fn path_to_file_url(path: &std::path::Path) -> String {
        let mut unix = path.to_string_lossy().replace('\\', "/");
        if !unix.starts_with('/') {
            unix.insert(0, '/');
        }
        format!("file://{unix}")
    }

    #[test]
    fn load_preview_image_reads_local_png() {
        let path = write_temp_png();
        let dest = path.to_str().expect("temp png path is utf-8");
        let (width, height, pixels) = load_preview_image(dest).unwrap();
        assert_eq!((width, height), (1, 1));
        assert_eq!(pixels, [10, 20, 30, 255]);

        let (width, height, pixels) = load_preview_image(&path_to_file_url(&path)).unwrap();
        assert_eq!((width, height), (1, 1));
        assert_eq!(pixels, [10, 20, 30, 255]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_preview_image_missing_path_is_err() {
        let path = std::env::temp_dir().join("devtoys-md-preview-missing-no-such-file.png");
        let _ = std::fs::remove_file(&path);
        let err = load_preview_image(path.to_str().unwrap()).unwrap_err();
        assert_eq!(err, ImagePreviewError::Unreadable);
        assert!(load_preview_image("").is_err());
        assert_eq!(
            load_preview_image("https://example.com/x.png").unwrap_err(),
            ImagePreviewError::Remote
        );
    }

    #[test]
    fn load_preview_image_error_display_omits_binary() {
        let path = std::env::temp_dir().join(format!(
            "devtoys-md-preview-not-image-{}.bin",
            std::process::id()
        ));
        let blob = vec![0u8; 64 * 1024];
        std::fs::write(&path, &blob).unwrap();
        let err = load_preview_image(path.to_str().unwrap()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.len() < 80, "{msg}");
        assert!(!msg.as_bytes().contains(&0), "{msg:?}");
        assert!(!msg.contains(&blob.len().to_string()));
        let _ = std::fs::remove_file(&path);

        let huge = "a".repeat(8 * 1024);
        let msg = load_preview_image(&huge).unwrap_err().to_string();
        assert!(msg.len() < 80, "{msg}");
        assert!(!msg.contains(&huge[..32]));
    }

    #[test]
    fn detector_accepts_title_rejects_json() {
        assert!(MarkdownDetector
            .detect(&RawData::text("# Title"), Some(&parent("# Title")))
            .is_some());
        assert!(MarkdownDetector
            .detect(&RawData::text(r#"{"a":1}"#), Some(&parent(r#"{"a":1}"#)))
            .is_none());
    }

    fn tiny_png_bytes() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([10, 20, 30, 255]));
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    struct MapFetcher {
        map: std::collections::HashMap<String, Result<Vec<u8>, String>>,
    }

    impl ImageFetcher for MapFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            match self.map.get(url) {
                Some(Ok(bytes)) => Ok(bytes.clone()),
                Some(Err(err)) => Err(err.clone()),
                None => Err(format!("unexpected url {url}")),
            }
        }
    }

    struct PanicFetcher;

    impl ImageFetcher for PanicFetcher {
        fn fetch(&self, url: &str) -> Result<Vec<u8>, String> {
            panic!("local dest must not fetch: {url}");
        }
    }

    #[test]
    fn remote_image_success_via_injected_fetcher() {
        let url = "https://example.com/x.png";
        let fetcher = MapFetcher {
            map: std::collections::HashMap::from([(url.to_string(), Ok(tiny_png_bytes()))]),
        };
        let (width, height, pixels) = load_preview_image_with(url, Some(&fetcher)).unwrap();
        assert_eq!((width, height), (1, 1));
        assert_eq!(pixels, [10, 20, 30, 255]);
    }

    #[test]
    fn remote_image_failure_via_injected_fetcher() {
        let url = "http://example.com/missing.png";
        let fetcher = MapFetcher {
            map: std::collections::HashMap::from([(url.to_string(), Err("not found".to_string()))]),
        };
        assert_eq!(
            load_preview_image_with(url, Some(&fetcher)).unwrap_err(),
            ImagePreviewError::Unreadable
        );
    }

    #[test]
    fn remote_image_without_fetcher_stays_remote_error() {
        assert_eq!(
            load_preview_image("https://example.com/x.png").unwrap_err(),
            ImagePreviewError::Remote
        );
    }

    #[test]
    fn local_image_does_not_call_fetcher() {
        let path = write_temp_png();
        let dest = path.to_str().expect("temp png path is utf-8");
        let (width, height, pixels) = load_preview_image_with(dest, Some(&PanicFetcher)).unwrap();
        assert_eq!((width, height), (1, 1));
        assert_eq!(pixels, [10, 20, 30, 255]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn linked_remote_image_keeps_click_dest_on_load_success_and_fail() {
        let doc = parse_markdown("[![x](https://example.com/x.png)](https://example.com)");
        let (link_dest, image) = linked_image(paragraph_inlines(&doc))
            .unwrap_or_else(|| panic!("expected linked image, got {:?}", doc.blocks));
        assert_eq!(link_dest, "https://example.com");
        match image {
            Inline::Image { dest, alt, .. } => {
                assert_eq!(dest, "https://example.com/x.png");
                assert_eq!(alt, "x");
            }
            other => panic!("expected image, got {other:?}"),
        }
        assert_eq!(
            preview_click_dest(PreviewClickNode::LinkedImage {
                loaded: true,
                link_dest,
            }),
            Some("https://example.com")
        );
        assert_eq!(
            preview_click_dest(PreviewClickNode::LinkedImage {
                loaded: false,
                link_dest,
            }),
            Some("https://example.com")
        );
    }

    #[test]
    fn highlight_json_colors_or_splits_spans() {
        let src = r#"{"a":1}"#;
        let spans = highlight_code(Some("json"), src);
        let joined: String = spans.iter().map(|span| span.text.as_str()).collect();
        assert_eq!(joined, src, "{spans:?}");
        let color_of = |needle: &str| {
            spans
                .iter()
                .find(|span| span.text.contains(needle))
                .map(|span| span.color)
        };
        let differs = match (
            color_of("{").or_else(|| color_of("}")),
            color_of("1").or_else(|| color_of("a")),
        ) {
            (Some(brace), Some(lit)) => brace != lit,
            _ => false,
        };
        assert!(
            spans.len() > 1 || differs,
            "expected json highlighting, got {spans:?}"
        );
    }

    #[test]
    fn highlight_no_language_is_one_plain_span() {
        let src = r#"{"a":1}"#;
        let spans = highlight_code(None, src);
        assert_eq!(spans.len(), 1, "{spans:?}");
        assert_eq!(spans[0].text, src);
        assert_eq!(spans[0].color, PLAIN_HIGHLIGHT);
    }

    #[test]
    fn highlight_unknown_language_does_not_panic() {
        let src = "hello { world }";
        let spans = highlight_code(Some("not-a-real-lang-xyz"), src);
        assert_eq!(spans.len(), 1, "{spans:?}");
        assert_eq!(spans[0].text, src);
    }

    #[test]
    fn parse_inline_html_bold_is_strong() {
        let doc = parse_markdown("hello <b>x</b>");
        let inlines = paragraph_inlines(&doc);
        assert!(
            find_inline(inlines, |inline| matches!(
                inline,
                Inline::Strong(children) if inline_plain_text(children) == "x"
            )),
            "expected Strong containing x, got {inlines:?}"
        );
        assert!(
            !find_inline(inlines, |inline| match inline {
                Inline::Text(text) | Inline::Html(text) => {
                    text.contains("<b>") || text.contains("</b>") || text.contains("<b>x</b>")
                }
                _ => false,
            }),
            "tags must not remain as text: {inlines:?}"
        );
    }

    #[test]
    fn parse_html_br_is_hard_break() {
        let doc = parse_markdown("<br>");
        assert!(
            walk_inlines(&doc.blocks, |inline| matches!(inline, Inline::HardBreak)),
            "expected HardBreak, got {:?}",
            doc.blocks
        );
    }

    #[test]
    fn parse_heading_still_heading_after_html_rewrite() {
        let doc = parse_markdown("# title");
        match doc.blocks.as_slice() {
            [Block::Heading { level, children }] => {
                assert_eq!(*level, 1);
                assert_eq!(inline_plain_text(children), "title");
            }
            other => panic!("expected heading, got {other:?}"),
        }
    }

    #[test]
    fn parse_html_paragraph_and_list() {
        let doc = parse_markdown("<p>hi</p><ul><li>a</li></ul>");
        assert!(
            walk_blocks(
                &doc.blocks,
                |block| matches!(block, Block::Paragraph { children } if inline_plain_text(children) == "hi")
            ),
            "expected paragraph hi: {:?}",
            doc.blocks
        );
        assert!(
            walk_blocks(&doc.blocks, |block| match block {
                Block::List { items, ordered, .. } => {
                    !*ordered
                        && items
                            .iter()
                            .any(|item| list_item_text(item) == "a" || {
                                item.children.iter().any(|child| {
                                    matches!(child, Block::Paragraph { children } if inline_plain_text(children) == "a")
                                })
                            })
                }
                _ => false,
            }),
            "expected ul/li a: {:?}",
            doc.blocks
        );
    }

    #[test]
    fn parse_html_script_is_not_shown() {
        let doc = parse_markdown("<script>alert(1)</script>");
        let dumped = format!("{doc:?}");
        assert!(!dumped.contains("alert"), "{dumped}");
        assert!(!dumped.to_lowercase().contains("script"), "{dumped}");
    }

    #[test]
    fn parse_html_strong_and_em_aliases() {
        let strong = parse_markdown("n<strong>x</strong>");
        assert!(
            find_inline(paragraph_inlines(&strong), |inline| matches!(
                inline,
                Inline::Strong(children) if inline_plain_text(children) == "x"
            )),
            "{:?}",
            strong.blocks
        );
        let em = parse_markdown("n<em>x</em>");
        assert!(
            find_inline(paragraph_inlines(&em), |inline| matches!(
                inline,
                Inline::Emphasis(children) if inline_plain_text(children) == "x"
            )),
            "{:?}",
            em.blocks
        );
    }
}
