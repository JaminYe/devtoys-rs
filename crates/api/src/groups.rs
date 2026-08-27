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
}
