use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguagePreference {
    #[default]
    System,
    #[serde(rename = "zh_cn", alias = "zh-CN", alias = "zh")]
    ZhCn,
    #[serde(rename = "en_us", alias = "en-US", alias = "en")]
    EnUs,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    #[default]
    ZhCn,
    EnUs,
}

impl LanguagePreference {
    pub fn resolve(self) -> Language {
        match self {
            Self::ZhCn => Language::ZhCn,
            Self::EnUs => Language::EnUs,
            Self::System => detect_system_language(),
        }
    }

    pub fn resolve_with<FEnv, FWin>(self, get_env: FEnv, get_win: FWin) -> Language
    where
        FEnv: FnMut(&str) -> Option<String>,
        FWin: FnOnce() -> Option<String>,
    {
        match self {
            Self::ZhCn => Language::ZhCn,
            Self::EnUs => Language::EnUs,
            Self::System => detect_system_language_with(get_env, get_win),
        }
    }
}

#[cfg(windows)]
pub fn windows_system_language() -> Option<String> {
    use windows_sys::Win32::Globalization::{
        GetUserDefaultLocaleName, GetUserPreferredUILanguages, MUI_LANGUAGE_NAME,
    };

    const NAME_LENGTH: usize = 85;

    let mut names = [0u16; NAME_LENGTH * 8];
    let mut count = 0u32;
    let mut length = names.len() as u32;
    let preferred = unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            names.as_mut_ptr(),
            &mut length,
        )
    };
    if preferred != 0 && count > 0 {
        if let Some(first) = first_name(&names) {
            return Some(first);
        }
    }

    let mut name = [0u16; NAME_LENGTH];
    let read = unsafe { GetUserDefaultLocaleName(name.as_mut_ptr(), name.len() as i32) };
    (read > 0).then(|| first_name(&name)).flatten()
}

#[cfg(windows)]
fn first_name(names: &[u16]) -> Option<String> {
    let end = names.iter().position(|unit| *unit == 0)?;
    (end > 0).then(|| String::from_utf16_lossy(&names[..end]))
}

#[cfg(not(windows))]
pub fn windows_system_language() -> Option<String> {
    None
}

pub fn parse_language_tag(tag: &str) -> Option<Language> {
    let lower = tag.trim().to_ascii_lowercase();
    if lower.starts_with("zh") {
        Some(Language::ZhCn)
    } else if lower.starts_with("en") {
        Some(Language::EnUs)
    } else {
        None
    }
}

pub fn detect_system_language() -> Language {
    detect_system_language_with(
        |var| std::env::var(var).ok(),
        windows_system_language,
    )
}

pub fn detect_system_language_with<FEnv, FWin>(mut get_env: FEnv, get_win: FWin) -> Language
where
    FEnv: FnMut(&str) -> Option<String>,
    FWin: FnOnce() -> Option<String>,
{
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(val) = get_env(var) {
            if let Some(lang) = parse_language_tag(&val) {
                return lang;
            }
        }
    }
    if let Some(win_lang) = get_win() {
        if let Some(lang) = parse_language_tag(&win_lang) {
            return lang;
        }
    }
    Language::default()
}

/// Central translation dictionary lookup with fallback.
pub fn t(key: &str, lang: Language) -> &str {
    match (key, lang) {
        // --- Host Shell ---
        ("host.all_tools", Language::ZhCn) => "全部工具",
        ("host.all_tools", Language::EnUs) => "All tools",
        ("host.favorites", Language::ZhCn) => "收藏",
        ("host.favorites", Language::EnUs) => "Favorites",
        ("host.settings", Language::ZhCn) => "设置",
        ("host.settings", Language::EnUs) => "Settings",
        ("host.search_placeholder", Language::ZhCn) => "键入以搜索工具...",
        ("host.search_placeholder", Language::EnUs) => "Type to search for tools...",
        ("host.no_tools_found", Language::ZhCn) => "未找到相关工具",
        ("host.no_tools_found", Language::EnUs) => "No tools found",
        ("host.pip_overlay", Language::ZhCn) => "画中画",
        ("host.pip_overlay", Language::EnUs) => "Compact Overlay",
        ("host.pip_exit", Language::ZhCn) => "返回主窗",
        ("host.pip_exit", Language::EnUs) => "Exit Compact Overlay",
        ("host.pip_tooltip_disabled", Language::ZhCn) => "此工具布局较密集，暂不支持画中画小窗",
        ("host.select_tool_desc", Language::ZhCn) => "选择一个工具开始处理内容",
        ("host.select_tool_desc", Language::EnUs) => "Select a tool to start working",
        ("host.empty_group_desc", Language::ZhCn) => "可从左侧浏览其他分组，或使用搜索快速定位",
        ("host.empty_group_desc", Language::EnUs) => "Browse other groups on the left, or use search to locate tools",
        ("host.no_tools", Language::ZhCn) => "暂无工具",
        ("host.no_tools", Language::EnUs) => "No tools available",
        ("host.no_favorites", Language::ZhCn) => "暂无收藏",
        ("host.no_favorites", Language::EnUs) => "No favorites yet",
        ("host.tool_unavailable", Language::ZhCn) => "此工具暂不可打开",
        ("host.tool_unavailable", Language::EnUs) => "This tool is not openable",
        ("host.pinned_overlay", Language::ZhCn) => "置顶小窗",
        ("host.pinned_overlay", Language::EnUs) => "Pinned Compact Window",
        ("host.pip_tooltip_enabled", Language::ZhCn) => "在始终置顶的小窗口中打开此工具",
        ("host.pip_tooltip_enabled", Language::EnUs) => "Open this tool in an always-on-top compact window",
        ("host.favorite_add", Language::ZhCn) => "收藏",
        ("host.favorite_add", Language::EnUs) => "Favorite",
        ("host.favorite_remove", Language::ZhCn) => "取消收藏",
        ("host.favorite_remove", Language::EnUs) => "Unfavorite",
        ("host.pip_tooltip_disabled", Language::EnUs) => "This tool does not support Compact Overlay",

        // --- Groups ---
        ("group.converters", Language::ZhCn) => "转换器",
        ("group.converters", Language::EnUs) => "Converters",
        ("group.encoders_decoders", Language::ZhCn) => "编解码器",
        ("group.encoders_decoders", Language::EnUs) => "Encoders / Decoders",
        ("group.formatters", Language::ZhCn) => "格式化工具",
        ("group.formatters", Language::EnUs) => "Formatters",
        ("group.generators", Language::ZhCn) => "生成器",
        ("group.generators", Language::EnUs) => "Generators",
        ("group.graphic", Language::ZhCn) => "图像处理",
        ("group.graphic", Language::EnUs) => "Graphic",
        ("group.testers", Language::ZhCn) => "测试工具",
        ("group.testers", Language::EnUs) => "Testers",
        ("group.text", Language::ZhCn) => "文本处理",
        ("group.text", Language::EnUs) => "Text",

        // --- Settings Page ---
        ("settings.title", Language::ZhCn) => "设置",
        ("settings.title", Language::EnUs) => "Settings",
        ("settings.appearance", Language::ZhCn) => "外观",
        ("settings.appearance", Language::EnUs) => "Appearance",
        ("settings.theme", Language::ZhCn) => "应用主题",
        ("settings.theme", Language::EnUs) => "App theme",
        ("settings.theme_desc", Language::ZhCn) => "选择要在 DevToys 中显示的主题",
        ("settings.theme_desc", Language::EnUs) => "Select which app theme to display",
        ("settings.theme_system", Language::ZhCn) => "跟随系统",
        ("settings.theme_system", Language::EnUs) => "Use system settings",
        ("settings.theme_light", Language::ZhCn) => "浅色",
        ("settings.theme_light", Language::EnUs) => "Light",
        ("settings.theme_dark", Language::ZhCn) => "深色",
        ("settings.theme_dark", Language::EnUs) => "Dark",

        ("settings.language", Language::ZhCn) => "界面语言",
        ("settings.language", Language::EnUs) => "Language",
        ("settings.language_desc", Language::ZhCn) => "选择界面显示的语言",
        ("settings.language_desc", Language::EnUs) => "Select which language to display",
        ("settings.language_system", Language::ZhCn) => "跟随系统",
        ("settings.language_system", Language::EnUs) => "Use system settings",
        ("settings.language_zh_cn", Language::ZhCn) => "中文 (简体)",
        ("settings.language_zh_cn", Language::EnUs) => "Simplified Chinese",
        ("settings.language_en_us", Language::ZhCn) => "English",
        ("settings.language_en_us", Language::EnUs) => "English",

        ("settings.behavior", Language::ZhCn) => "行为",
        ("settings.behavior", Language::EnUs) => "Behavior",
        ("settings.smart_detection", Language::ZhCn) => "智能检测",
        ("settings.smart_detection_desc", Language::ZhCn) => "根据剪贴板内容自动推荐工具",
        ("settings.smart_detection", Language::EnUs) => "Smart detection",
        ("settings.smart_detection_desc", Language::EnUs) => "Automatically recommend tools based on clipboard content",
        ("settings.auto_paste", Language::ZhCn) => "自动粘贴",
        ("settings.auto_paste_desc", Language::ZhCn) => "切换到推荐工具时自动粘贴剪贴板文本",
        ("settings.auto_paste", Language::EnUs) => "Auto paste",
        ("settings.auto_paste_desc", Language::EnUs) => "Automatically paste clipboard content when navigating to a recommended tool",
        ("settings.about", Language::ZhCn) => "关于",
        ("settings.about", Language::EnUs) => "About",
        ("settings.version", Language::ZhCn) => "版本",
        ("settings.version", Language::EnUs) => "Version",

        // --- Common Tool Strings ---
        ("common.configuration", Language::ZhCn) => "配置",
        ("common.configuration", Language::EnUs) => "Configuration",
        ("common.input", Language::ZhCn) => "输入",
        ("common.input", Language::EnUs) => "Input",
        ("common.output", Language::ZhCn) => "输出",
        ("common.output", Language::EnUs) => "Output",
        ("common.copy", Language::ZhCn) => "复制",
        ("common.copy", Language::EnUs) => "Copy",
        ("common.copied", Language::ZhCn) => "已复制 ✓",
        ("common.copied", Language::EnUs) => "Copied ✓",
        ("common.match", Language::ZhCn) => "匹配",
        ("common.match", Language::EnUs) => "Match",
        ("common.mismatch", Language::ZhCn) => "不匹配",
        ("common.mismatch", Language::EnUs) => "Mismatch",
        ("common.paste", Language::ZhCn) => "粘贴",
        ("common.paste", Language::EnUs) => "Paste",
        ("common.clear", Language::ZhCn) => "清空",
        ("common.clear", Language::EnUs) => "Clear",
        ("common.load_file", Language::ZhCn) => "打开文件",
        ("common.load_file", Language::EnUs) => "Load file",
        ("common.save_file", Language::ZhCn) => "保存文件",
        ("common.save_file", Language::EnUs) => "Save file",
        ("common.indentation", Language::ZhCn) => "缩进",
        ("common.indentation", Language::EnUs) => "Indentation",
        ("common.two_spaces", Language::ZhCn) => "2 个空格",
        ("common.two_spaces", Language::EnUs) => "2 spaces",
        ("common.four_spaces", Language::ZhCn) => "4 个空格",
        ("common.four_spaces", Language::EnUs) => "4 spaces",
        ("common.one_tab", Language::ZhCn) => "1 个制表符",
        ("common.one_tab", Language::EnUs) => "1 tab",
        ("common.minified", Language::ZhCn) => "压缩",
        ("common.minified", Language::EnUs) => "Minified",

        // --- Issue 07: Converters & Formatters ---
        ("cron.title", Language::ZhCn) => "Cron 解析器",
        ("cron.title", Language::EnUs) => "Cron Parser",
        ("cron.expression", Language::ZhCn) => "Cron 表达式",
        ("cron.expression", Language::EnUs) => "Cron expression",
        ("cron.include_seconds", Language::ZhCn) => "包含秒字段",
        ("cron.include_seconds", Language::EnUs) => "Include seconds",
        ("cron.next_occurrences", Language::ZhCn) => "下次执行时刻",
        ("cron.next_occurrences", Language::EnUs) => "Next scheduled execution times",
        ("cron.description", Language::ZhCn) => "自然语言描述",
        ("cron.description", Language::EnUs) => "Human description",
        ("cron.invalid_expression", Language::ZhCn) => "无效的 Cron 表达式",
        ("cron.invalid_expression", Language::EnUs) => "Invalid Cron expression",

        ("date.title", Language::ZhCn) => "日期",
        ("date.title", Language::EnUs) => "Date Converter",
        ("date.format", Language::ZhCn) => "格式",
        ("date.format", Language::EnUs) => "Format",
        ("date.timezone", Language::ZhCn) => "时区",
        ("date.timezone", Language::EnUs) => "Timezone",
        ("date.now", Language::ZhCn) => "现在",
        ("date.now", Language::EnUs) => "Now",
        ("date.custom_epoch", Language::ZhCn) => "自定义纪元",
        ("date.custom_epoch", Language::EnUs) => "Custom Epoch",
        ("date.year", Language::ZhCn) => "年",
        ("date.year", Language::EnUs) => "Year",
        ("date.month", Language::ZhCn) => "月",
        ("date.month", Language::EnUs) => "Month",
        ("date.day", Language::ZhCn) => "日",
        ("date.day", Language::EnUs) => "Day",
        ("date.hour", Language::ZhCn) => "时",
        ("date.hour", Language::EnUs) => "Hour",
        ("date.minute", Language::ZhCn) => "分",
        ("date.minute", Language::EnUs) => "Minute",
        ("date.second", Language::ZhCn) => "秒",
        ("date.second", Language::EnUs) => "Second",
        ("date.invalid", Language::ZhCn) => "无效的日期或时间戳",
        ("date.invalid", Language::EnUs) => "Invalid date or timestamp",

        ("json_table.title", Language::ZhCn) => "JSON > 表格",
        ("json_table.title", Language::EnUs) => "JSON to Table",
        ("json_table.flatten", Language::ZhCn) => "扁平化嵌套对象",
        ("json_table.flatten", Language::EnUs) => "Flatten nested objects",

        ("json_yaml.title", Language::ZhCn) => "JSON <> YAML",
        ("json_yaml.title", Language::EnUs) => "JSON <> YAML",
        ("json_yaml.json_to_yaml", Language::ZhCn) => "JSON 转 YAML",
        ("json_yaml.json_to_yaml", Language::EnUs) => "JSON to YAML",
        ("json_yaml.yaml_to_json", Language::ZhCn) => "YAML 转 JSON",
        ("json_yaml.yaml_to_json", Language::EnUs) => "YAML to JSON",

        ("number_base.title", Language::ZhCn) => "进制转换",
        ("number_base.title", Language::EnUs) => "Number Base",
        ("number_base.decimal", Language::ZhCn) => "十进制",
        ("number_base.decimal", Language::EnUs) => "Decimal",
        ("number_base.hexadecimal", Language::ZhCn) => "十六进制",
        ("number_base.hexadecimal", Language::EnUs) => "Hexadecimal",
        ("number_base.octal", Language::ZhCn) => "八进制",
        ("number_base.octal", Language::EnUs) => "Octal",
        ("number_base.binary", Language::ZhCn) => "二进制",
        ("number_base.binary", Language::EnUs) => "Binary",
        ("number_base.signed", Language::ZhCn) => "有符号",
        ("number_base.signed", Language::EnUs) => "Signed",
        ("number_base.unsigned", Language::ZhCn) => "无符号",
        ("number_base.unsigned", Language::EnUs) => "Unsigned",
        ("number_base.format_thousands", Language::ZhCn) => "千分位",
        ("number_base.format_thousands", Language::EnUs) => "Format thousands",

        ("json.title", Language::ZhCn) => "JSON",
        ("json.title", Language::EnUs) => "JSON",
        ("json.sort_properties", Language::ZhCn) => "属性排序",
        ("json.sort_properties", Language::EnUs) => "Sort JSON properties alphabetically",
        ("json.invalid", Language::ZhCn) => "非法 JSON",
        ("json.invalid", Language::EnUs) => "Invalid JSON",

        ("sql.title", Language::ZhCn) => "SQL",
        ("sql.title", Language::EnUs) => "SQL",
        ("sql.language", Language::ZhCn) => "方言",
        ("sql.language", Language::EnUs) => "Language",
        ("sql.leading_comma", Language::ZhCn) => "前导逗号",
        ("sql.leading_comma", Language::EnUs) => "Leading comma",

        ("xml.title", Language::ZhCn) => "XML",
        ("xml.title", Language::EnUs) => "XML",
        ("xml.attributes_on_new_lines", Language::ZhCn) => "属性换行",
        ("xml.attributes_on_new_lines", Language::EnUs) => "Put attributes on new lines",
        ("xml.invalid", Language::ZhCn) => "非法 XML",
        ("xml.invalid", Language::EnUs) => "Invalid XML",

        // --- Issue 08: Encoders & Generators ---
        ("base64_text.title", Language::ZhCn) => "Base64 文本",
        ("base64_text.title", Language::EnUs) => "Base64 Text",
        ("base64_text.encode", Language::ZhCn) => "编码",
        ("base64_text.encode", Language::EnUs) => "Encode",
        ("base64_text.decode", Language::ZhCn) => "解码",
        ("base64_text.decode", Language::EnUs) => "Decode",
        ("base64_text.multiline", Language::ZhCn) => "多行",
        ("base64_text.multiline", Language::EnUs) => "Multiline",
        ("base64_text.invalid", Language::ZhCn) => "非法 Base64",
        ("base64_text.invalid", Language::EnUs) => "Invalid Base64",
        ("base64_text.not_ascii", Language::ZhCn) => "非 ASCII 文本",
        ("base64_text.not_ascii", Language::EnUs) => "Non-ASCII text",

        ("base64_image.title", Language::ZhCn) => "Base64 图片",
        ("base64_image.title", Language::EnUs) => "Base64 Image",
        ("base64_image.invalid", Language::ZhCn) => "非法图片",
        ("base64_image.invalid", Language::EnUs) => "Invalid image",

        ("jwt.title", Language::ZhCn) => "JWT 编解码",
        ("jwt.title", Language::EnUs) => "JWT",
        ("jwt.header", Language::ZhCn) => "头部 (Header)",
        ("jwt.header", Language::EnUs) => "Header",
        ("jwt.payload", Language::ZhCn) => "载荷 (Payload)",
        ("jwt.payload", Language::EnUs) => "Payload",
        ("jwt.signature", Language::ZhCn) => "签名 (Signature)",
        ("jwt.signature", Language::EnUs) => "Signature",
        ("jwt.validate", Language::ZhCn) => "验证签名",
        ("jwt.validate", Language::EnUs) => "Validate signature",
        ("jwt.valid", Language::ZhCn) => "签名有效",
        ("jwt.valid", Language::EnUs) => "Signature is valid",
        ("jwt.invalid", Language::ZhCn) => "签名无效",
        ("jwt.invalid", Language::EnUs) => "Signature is invalid",

        ("url.title", Language::ZhCn) => "URL 编解码",
        ("url.title", Language::EnUs) => "URL",
        ("url.encode", Language::ZhCn) => "编码",
        ("url.encode", Language::EnUs) => "Encode",
        ("url.decode", Language::ZhCn) => "解码",
        ("url.decode", Language::EnUs) => "Decode",
        ("url.invalid", Language::ZhCn) => "非法 URL 编码",
        ("url.invalid", Language::EnUs) => "Invalid URL encoding",

        ("hash.title", Language::ZhCn) => "哈希 / 校验和",
        ("hash.title", Language::EnUs) => "Hash / Checksum",
        ("hash.algorithm", Language::ZhCn) => "算法",
        ("hash.algorithm", Language::EnUs) => "Algorithm",
        ("hash.uppercase", Language::ZhCn) => "大写",
        ("hash.uppercase", Language::EnUs) => "Uppercase",
        ("hash.hmac", Language::ZhCn) => "HMAC 模式",
        ("hash.hmac", Language::EnUs) => "HMAC mode",
        ("hash.secret_key", Language::ZhCn) => "密钥",
        ("hash.secret_key", Language::EnUs) => "Secret key",

        ("password.title", Language::ZhCn) => "密码生成器",
        ("password.title", Language::EnUs) => "Password",
        ("password.length", Language::ZhCn) => "长度",
        ("password.length", Language::EnUs) => "Length",
        ("password.digits", Language::ZhCn) => "包含数字",
        ("password.digits", Language::EnUs) => "Include digits",
        ("password.uppercase", Language::ZhCn) => "包含大写字母",
        ("password.uppercase", Language::EnUs) => "Include uppercase letters",
        ("password.lowercase", Language::ZhCn) => "包含小写字母",
        ("password.lowercase", Language::EnUs) => "Include lowercase letters",
        ("password.special", Language::ZhCn) => "包含特殊字符",
        ("password.special", Language::EnUs) => "Include special characters",
        ("password.generate", Language::ZhCn) => "重新生成",
        ("password.generate", Language::EnUs) => "Regenerate",

        ("uuid.title", Language::ZhCn) => "UUID 生成器",
        ("uuid.title", Language::EnUs) => "UUID",
        ("uuid.count", Language::ZhCn) => "生成数量",
        ("uuid.count", Language::EnUs) => "Count",
        ("uuid.hyphens", Language::ZhCn) => "连字符",
        ("uuid.hyphens", Language::EnUs) => "Hyphens",
        ("uuid.uppercase", Language::ZhCn) => "大写",
        ("uuid.uppercase", Language::EnUs) => "Uppercase",
        ("uuid.version", Language::ZhCn) => "版本",
        ("uuid.version", Language::EnUs) => "Version",

        // --- Issue 09: Testers, Text, Graphic ---
        ("jsonpath.title", Language::ZhCn) => "JSONPath",
        ("jsonpath.title", Language::EnUs) => "JSONPath",
        ("jsonpath.expression", Language::ZhCn) => "JSONPath 表达式",
        ("jsonpath.expression", Language::EnUs) => "JSONPath expression",

        ("regex.title", Language::ZhCn) => "正则表达式",
        ("regex.title", Language::EnUs) => "Regex Tester",
        ("regex.pattern", Language::ZhCn) => "正则表达式",
        ("regex.pattern", Language::EnUs) => "Regular expression",
        ("regex.sample", Language::ZhCn) => "测试文本",
        ("regex.sample", Language::EnUs) => "Sample text",
        ("regex.substitution", Language::ZhCn) => "替换式",
        ("regex.substitution", Language::EnUs) => "Substitution",
        ("regex.ignore_case", Language::ZhCn) => "忽略大小写",
        ("regex.ignore_case", Language::EnUs) => "Ignore case",
        ("regex.multiline", Language::ZhCn) => "多行模式",
        ("regex.multiline", Language::EnUs) => "Multiline",
        ("regex.singleline", Language::ZhCn) => "单行模式",
        ("regex.singleline", Language::EnUs) => "Singleline",
        ("regex.matches", Language::ZhCn) => "匹配项",
        ("regex.matches", Language::EnUs) => "Matches",

        ("text_analyzer.title", Language::ZhCn) => "文本分析",
        ("text_analyzer.title", Language::EnUs) => "Text Analyzer",
        ("text_analyzer.characters", Language::ZhCn) => "字符数",
        ("text_analyzer.characters", Language::EnUs) => "Characters",
        ("text_analyzer.words", Language::ZhCn) => "词数",
        ("text_analyzer.words", Language::EnUs) => "Words",
        ("text_analyzer.lines", Language::ZhCn) => "行数",
        ("text_analyzer.lines", Language::EnUs) => "Lines",
        ("text_analyzer.bytes", Language::ZhCn) => "字节数",
        ("text_analyzer.bytes", Language::EnUs) => "Bytes",

        ("text_compare.title", Language::ZhCn) => "文本对比",
        ("text_compare.title", Language::EnUs) => "Text Compare",
        ("text_compare.original", Language::ZhCn) => "原始内容",
        ("text_compare.original", Language::EnUs) => "Original text",
        ("text_compare.modified", Language::ZhCn) => "修改后内容",
        ("text_compare.modified", Language::EnUs) => "Modified text",
        ("text_compare.side_by_side", Language::ZhCn) => "并排",
        ("text_compare.side_by_side", Language::EnUs) => "Side by side",
        ("text_compare.inline", Language::ZhCn) => "行内",
        ("text_compare.inline", Language::EnUs) => "Inline",

        ("escape.title", Language::ZhCn) => "转义 / 反转义",
        ("escape.title", Language::EnUs) => "Escape / Unescape",
        ("escape.escape", Language::ZhCn) => "转义",
        ("escape.escape", Language::EnUs) => "Escape",
        ("escape.unescape", Language::ZhCn) => "反转义",
        ("escape.unescape", Language::EnUs) => "Unescape",
        ("escape.invalid", Language::ZhCn) => "非法转义序列",
        ("escape.invalid", Language::EnUs) => "Invalid escape sequence",

        ("list_compare.title", Language::ZhCn) => "列表对比",
        ("list_compare.title", Language::EnUs) => "List Compare",
        ("list_compare.list_a", Language::ZhCn) => "列表 A",
        ("list_compare.list_a", Language::EnUs) => "List A",
        ("list_compare.list_b", Language::ZhCn) => "列表 B",
        ("list_compare.list_b", Language::EnUs) => "List B",
        ("list_compare.intersection", Language::ZhCn) => "交集 (A ∩ B)",
        ("list_compare.intersection", Language::EnUs) => "Intersection (A ∩ B)",
        ("list_compare.difference_a", Language::ZhCn) => "A 独有 (A - B)",
        ("list_compare.difference_a", Language::EnUs) => "Only in A (A - B)",
        ("list_compare.difference_b", Language::ZhCn) => "B 独有 (B - A)",
        ("list_compare.difference_b", Language::EnUs) => "Only in B (B - A)",

        ("markdown.title", Language::ZhCn) => "Markdown 预览",
        ("markdown.title", Language::EnUs) => "Markdown Preview",
        ("markdown.preview", Language::ZhCn) => "预览",
        ("markdown.preview", Language::EnUs) => "Preview",
        ("markdown.loading_image", Language::ZhCn) => "正在加载图片...",
        ("markdown.loading_image", Language::EnUs) => "Loading image...",
        ("markdown.cannot_load_image", Language::ZhCn) => "无法加载图片",
        ("markdown.cannot_load_image", Language::EnUs) => "Failed to load image",

        ("image_converter.title", Language::ZhCn) => "图片格式转换器",
        ("image_converter.title", Language::EnUs) => "Image Converter",
        ("image_converter.target_format", Language::ZhCn) => "目标格式",
        ("image_converter.target_format", Language::EnUs) => "Target format",
        ("image_converter.select_file", Language::ZhCn) => "选择图片",
        ("image_converter.select_file", Language::EnUs) => "Select image",

        // --- Fallback rule: if EnUs is missing, fallback to ZhCn; if ZhCn is missing, fallback to key ---
        (_, Language::EnUs) => {
            let zh = t(key, Language::ZhCn);
            if zh != key {
                zh
            } else {
                key
            }
        }
        _ => key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_shell_keys_have_both_translations() {
        let keys = [
            "host.all_tools",
            "host.favorites",
            "host.settings",
            "host.search_placeholder",
            "host.no_tools_found",
            "host.pip_overlay",
            "host.pip_exit",
            "group.converters",
            "group.encoders_decoders",
            "group.formatters",
            "group.generators",
            "group.graphic",
            "group.testers",
            "group.text",
            "settings.title",
            "settings.appearance",
            "settings.theme",
            "settings.language",
            "settings.behavior",
            "settings.smart_detection",
            "settings.auto_paste",
        ];
        for k in keys {
            let zh = t(k, Language::ZhCn);
            let en = t(k, Language::EnUs);
            assert!(!zh.is_empty(), "key {k} has empty zh");
            assert!(!en.is_empty(), "key {k} has empty en");
            assert_ne!(zh, k, "key {k} missing zh translation");
            assert_ne!(en, k, "key {k} missing en translation");
            assert_ne!(zh, en, "key {k} zh and en should differ");
        }
    }

    #[test]
    fn all_23_tools_keys_have_complete_translations() {
        let tool_keys = [
            // 07
            "cron.title", "cron.expression", "cron.include_seconds", "cron.next_occurrences",
            "date.title", "date.format", "date.timezone", "date.now", "date.custom_epoch",
            "json_table.title", "json_table.flatten",
            "json_yaml.title", "json_yaml.json_to_yaml", "json_yaml.yaml_to_json",
            "number_base.title", "number_base.decimal", "number_base.hexadecimal", "number_base.signed", "number_base.unsigned",
            "json.sort_properties", "json.invalid",
            "sql.language", "sql.leading_comma",
            "xml.attributes_on_new_lines", "xml.invalid",
            // 08
            "base64_text.title", "base64_text.encode", "base64_text.decode", "base64_text.multiline",
            "base64_text.invalid", "base64_text.not_ascii",
            "base64_image.title", "base64_image.invalid",
            "jwt.title", "jwt.header", "jwt.payload", "jwt.signature", "jwt.validate",
            "url.title", "url.encode", "url.decode", "url.invalid",
            "hash.title", "hash.algorithm", "hash.uppercase", "hash.hmac", "hash.secret_key",
            "password.title", "password.length", "password.digits", "password.generate",
            "uuid.title", "uuid.count", "uuid.hyphens", "uuid.uppercase",
            // 09
            "jsonpath.title", "jsonpath.expression",
            "regex.title", "regex.pattern", "regex.sample", "regex.substitution", "regex.ignore_case",
            "text_analyzer.title", "text_analyzer.characters", "text_analyzer.words", "text_analyzer.lines",
            "text_compare.title", "text_compare.original", "text_compare.modified", "text_compare.side_by_side", "text_compare.inline",
            "escape.title", "escape.escape", "escape.unescape", "escape.invalid",
            "list_compare.title", "list_compare.list_a", "list_compare.list_b", "list_compare.intersection",
            "markdown.title", "markdown.preview", "markdown.loading_image", "markdown.cannot_load_image",
            "image_converter.title", "image_converter.target_format", "image_converter.select_file",
            // Common
            "common.configuration", "common.input", "common.output", "common.copy", "common.copied",
            "common.paste", "common.clear", "common.match", "common.mismatch",
            "common.indentation", "common.two_spaces", "common.four_spaces", "common.one_tab", "common.minified",
        ];
        for k in tool_keys {
            let zh = t(k, Language::ZhCn);
            let en = t(k, Language::EnUs);
            assert!(!zh.is_empty(), "key {k} has empty zh");
            assert!(!en.is_empty(), "key {k} has empty en");
            assert_ne!(zh, k, "key {k} missing zh translation");
            assert_ne!(en, k, "key {k} missing en translation");
        }
    }

    #[test]
    fn missing_key_falls_back_cleanly() {
        assert_eq!(t("unknown.key.xyz", Language::ZhCn), "unknown.key.xyz");
        assert_eq!(t("unknown.key.xyz", Language::EnUs), "unknown.key.xyz");
    }

    #[test]
    fn language_preference_roundtrip() {
        let json = serde_json::to_string(&LanguagePreference::ZhCn).unwrap();
        assert_eq!(json, "\"zh_cn\"");
        let pref: LanguagePreference = serde_json::from_str("\"zh-CN\"").unwrap();
        assert_eq!(pref, LanguagePreference::ZhCn);
        let pref: LanguagePreference = serde_json::from_str("\"en-US\"").unwrap();
        assert_eq!(pref, LanguagePreference::EnUs);
        let pref: LanguagePreference = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(pref, LanguagePreference::System);
    }

    #[test]
    fn test_system_language_detection_and_precedence() {
        // 1. LC_ALL takes precedence over LC_MESSAGES and LANG
        let detected = detect_system_language_with(
            |var| match var {
                "LC_ALL" => Some("en_US.UTF-8".into()),
                "LC_MESSAGES" => Some("zh_CN".into()),
                "LANG" => Some("zh_CN".into()),
                _ => None,
            },
            || Some("zh-CN".into()),
        );
        assert_eq!(detected, Language::EnUs);

        // 2. LC_MESSAGES takes precedence when LC_ALL is None
        let detected = detect_system_language_with(
            |var| match var {
                "LC_ALL" => None,
                "LC_MESSAGES" => Some("zh_CN.UTF-8".into()),
                "LANG" => Some("en_US".into()),
                _ => None,
            },
            || Some("en-US".into()),
        );
        assert_eq!(detected, Language::ZhCn);

        // 3. LANG takes precedence when LC_ALL and LC_MESSAGES are None
        let detected = detect_system_language_with(
            |var| match var {
                "LANG" => Some("en_GB".into()),
                _ => None,
            },
            || Some("zh-CN".into()),
        );
        assert_eq!(detected, Language::EnUs);

        // 4. Unsupported env vars fall through to next env var or Windows UI language
        let detected = detect_system_language_with(
            |var| match var {
                "LC_ALL" => Some("fr_FR".into()),
                "LC_MESSAGES" => Some("de_DE".into()),
                "LANG" => Some("ja_JP".into()),
                _ => None,
            },
            || Some("en-US".into()),
        );
        assert_eq!(detected, Language::EnUs);

        // 5. No env vars: Windows UI language Chinese -> ZhCn
        let detected = detect_system_language_with(|_| None, || Some("zh-Hans-CN".into()));
        assert_eq!(detected, Language::ZhCn);

        // 6. No env vars: Windows UI language English -> EnUs
        let detected = detect_system_language_with(|_| None, || Some("en-US".into()));
        assert_eq!(detected, Language::EnUs);

        // 7. No env vars: Unsupported Windows UI language -> falls back to default (ZhCn)
        let detected = detect_system_language_with(|_| None, || Some("ko-KR".into()));
        assert_eq!(detected, Language::ZhCn);

        // 8. No env vars: Windows UI language unavailable -> falls back to default (ZhCn)
        let detected = detect_system_language_with(|_| None, || None);
        assert_eq!(detected, Language::ZhCn);
    }

    #[test]
    fn test_language_preference_precedence() {
        // Explicit choice ZhCn overrides any system language or env vars
        let pref = LanguagePreference::ZhCn;
        assert_eq!(
            pref.resolve_with(|_| Some("en_US".into()), || Some("en-US".into())),
            Language::ZhCn
        );

        // Explicit choice EnUs overrides any system language or env vars
        let pref = LanguagePreference::EnUs;
        assert_eq!(
            pref.resolve_with(|_| Some("zh_CN".into()), || Some("zh-CN".into())),
            Language::EnUs
        );

        // System preference delegates to detection
        let pref = LanguagePreference::System;
        assert_eq!(
            pref.resolve_with(|_| None, || Some("en-US".into())),
            Language::EnUs
        );
    }

    #[test]
    fn test_specific_parity_keys_differ() {
        let parity_keys = [
            ("common.copy", "复制", "Copy"),
            ("common.copied", "已复制 ✓", "Copied ✓"),
            ("json.invalid", "非法 JSON", "Invalid JSON"),
            ("base64_text.multiline", "多行", "Multiline"),
            ("base64_text.invalid", "非法 Base64", "Invalid Base64"),
            ("base64_text.not_ascii", "非 ASCII 文本", "Non-ASCII text"),
            ("xml.invalid", "非法 XML", "Invalid XML"),
            ("escape.invalid", "非法转义序列", "Invalid escape sequence"),
            ("url.invalid", "非法 URL 编码", "Invalid URL encoding"),
        ];
        for (k, expected_zh, expected_en) in parity_keys {
            assert_eq!(t(k, Language::ZhCn), expected_zh);
            assert_eq!(t(k, Language::EnUs), expected_en);
            assert_ne!(t(k, Language::ZhCn), t(k, Language::EnUs));
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_system_language_smoke() {
        let lang = windows_system_language();
        assert!(lang.is_some(), "Windows UI language should be retrievable on Windows");
        let detected = detect_system_language();
        assert!(detected == Language::ZhCn || detected == Language::EnUs);
    }
}
