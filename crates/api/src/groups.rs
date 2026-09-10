use crate::i18n::Language;

/// Fixed business groups. Display names are Chinese and must stay in sync
/// with `DevToys-需求说明.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GroupId {
    Converters,
    EncodersDecoders,
    Formatters,
    Generators,
    Graphic,
    Testers,
    Text,
}

pub const ALL_TOOLS_ID: &str = "AllTools";
pub const ALL_TOOLS_LABEL: &str = "全部工具";
pub const FAVORITES_ID: &str = "FavoriteTools";
pub const FAVORITES_LABEL: &str = "收藏";

impl GroupId {
    pub const ALL: [GroupId; 7] = [
        GroupId::Converters,
        GroupId::EncodersDecoders,
        GroupId::Formatters,
        GroupId::Generators,
        GroupId::Graphic,
        GroupId::Testers,
        GroupId::Text,
    ];

    pub fn key(self) -> &'static str {
        match self {
            GroupId::Converters => "Converters",
            GroupId::EncodersDecoders => "EncodersDecoders",
            GroupId::Formatters => "Formatters",
            GroupId::Generators => "Generators",
            GroupId::Graphic => "Graphic",
            GroupId::Testers => "Testers",
            GroupId::Text => "Text",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            GroupId::Converters => "转换器",
            GroupId::EncodersDecoders => "编解码器",
            GroupId::Formatters => "格式化工具",
            GroupId::Generators => "生成器",
            GroupId::Graphic => "图像处理",
            GroupId::Testers => "测试工具",
            GroupId::Text => "文本处理",
        }
    }

    pub fn localized_name(self, lang: Language) -> &'static str {
        match (self, lang) {
            (GroupId::Converters, Language::ZhCn) => "转换器",
            (GroupId::Converters, Language::EnUs) => "Converters",
            (GroupId::EncodersDecoders, Language::ZhCn) => "编解码器",
            (GroupId::EncodersDecoders, Language::EnUs) => "Encoders / Decoders",
            (GroupId::Formatters, Language::ZhCn) => "格式化工具",
            (GroupId::Formatters, Language::EnUs) => "Formatters",
            (GroupId::Generators, Language::ZhCn) => "生成器",
            (GroupId::Generators, Language::EnUs) => "Generators",
            (GroupId::Graphic, Language::ZhCn) => "图像处理",
            (GroupId::Graphic, Language::EnUs) => "Graphic",
            (GroupId::Testers, Language::ZhCn) => "测试工具",
            (GroupId::Testers, Language::EnUs) => "Testers",
            (GroupId::Text, Language::ZhCn) => "文本处理",
            (GroupId::Text, Language::EnUs) => "Text",
        }
    }
}
