# DevToys 需求说明文档

| 项 | 内容 |
|---|---|
| 产品 | DevToys 2.0 — 开发者瑞士军刀 |
| 文档性质 | 基于当前代码与默认工具仓的 **As-Is 产品需求**（描述已实现能力，而非规划新功能） |
| 宿主仓库 | 本工作区 `DevToys/`（窗口、导航、设置、扩展管理、Smart Detection、UI 契约） |
| 默认工具仓 | [DevToys.Tools](https://github.com/DevToys-app/DevToys.Tools)（30 个默认工具，以插件加载） |
| 平台 | Windows / macOS / Linux 桌面；配套 CLI |
| 版本基线 | DevToys 2.0 扩展化架构 |
| 本仓接入结论 | **已接入 2 个 Footer GUI 工具**；**未接入** 30 个默认业务工具与任何 CLI 工具（它们在独立插件仓，运行时靠 `Plugins` 加载） |

---

## 1. 问题陈述

开发者日常需要完成大量细碎任务：JSON/YAML 互转、Base64 编解码、JWT 查看、正则试跑、哈希校验、图片格式转换等。这些任务往往要打开多个不可信网页，数据会离开本机。

用户需要：

- 一套 **离线、可信、即开即用** 的小工具合集
- **剪贴板智能识别**，自动推荐合适工具
- **可扩展**：官方默认 30 工具不够时，能装社区扩展或自研
- **跨平台桌面 + CLI**，同一套工具能力尽量复用

## 2. 解决方案

提供桌面宿主应用 DevToys：

1. 宿主负责窗口、侧栏分组、搜索/收藏/最近使用、设置、扩展安装、Smart Detection、通用 UI 控件。
2. 默认 30 个工具以 **插件** 形式装入 `Plugins`，通过 MEF 发现 `IGuiTool` / `ICommandLineTool`。
3. 剪贴板内容经类型检测树匹配工具；用户点进推荐工具时可自动填入数据。
4. 更多工具通过 Extension Manager 安装 nupkg；第三方可基于 `DevToys.Api` 自研。

---

## 3. 当前已接入清单（本工作区实测）

判定规则：源码里出现 `[Export(typeof(IGuiTool))]` / `[Export(typeof(ICommandLineTool))]` 且未被注释，即视为 **已接入宿主**。仅有图标、resx 或未 Export 的类不算接入。

### 3.1 一句话

| 类别 | 数量 | 状态 |
|---|---|---|
| 本仓库 GUI 工具 | **2** | ✅ 已接入：设置、扩展管理 |
| 本仓库 CLI 工具 | **0** | ❌ 宿主只提供 CLI 框架，无业务命令 |
| 本仓库工具分组 | **7** | ✅ 侧栏骨架已接入，分组内默认工具未编进本仓 |
| 本仓库 Smart Detection | **13** 个检测器 | ✅ 已接入通用类型 |
| 官方默认业务工具 | **30** | ⚠️ **规格属于产品，代码不在本仓**；发行包通过 DevToys.Tools 插件接入 |
| 未完成 / 未启用 | **1** | ❌ 「支持开发」Export 已注释 |
| 本工作区 `Plugins` | 无随仓插件 | 仅编译本仓时侧栏 **看不到** JSON/Base64 等 30 工具 |

### 3.2 本仓库已接入的 GUI 工具（2）

| 状态 | 类 | 内部名 `[Name]` | 菜单位置 | 平台 | 说明 |
|---|---|---|---|---|---|
| ✅ 已接入 | `SettingsGuiTool` | `Settings` | Footer | 全平台 | 外观 / 行为 / 编辑器 / 关于 |
| ✅ 已接入 | `ExtensionsManagerGuiTool` | `Extensions Manager` | Footer（排在设置前） | 仅 Windows、Linux、macOS | 安装卸载 nupkg |
| ❌ 未接入 | `SupportDevelopmentGuidTools` | `SupportDevelopment` | 本应 Footer | — | `//[Export(typeof(IGuiTool))]`，未完成 |

测试用 Mock（`MockIGuiTool` ×4）只存在于单元测试，不算产品接入。

### 3.3 本仓库已接入的分组（7，仅骨架）

| 状态 | 类 | Name | 中文 |
|---|---|---|---|
| ✅ | `ConvertersGroup` | Converters | 转换器 |
| ✅ | `EncodersDecodersGroup` | Encoders / Decoders | 编解码器 |
| ✅ | `FormattersGroup` | Formatters | 格式化工具 |
| ✅ | `GeneratorsGroup` | Generators | 生成器 |
| ✅ | `GraphicGroup` | Graphic | 图像处理 |
| ✅ | `TestersGroup` | Testers | 测试工具 |
| ✅ | `TextGroup` | Text | 文本处理 |

另有导航预留组（非 `GuiToolGroup` 导出）：`AllTools`、`FavoriteTools`。

### 3.4 本仓库已接入的数据类型检测器（13）

| 状态 | 类 | 类型名 | 父类型 |
|---|---|---|---|
| ✅ | `TextDataTypeDetector` | text | — |
| ✅ | `JsonDataTypeDetector` | json | text |
| ✅ | `JsonArrayDataTypeDetector` | jsonArray | json |
| ✅ | `XmlDataTypeDetector` | xml | text |
| ✅ | `XsdDataTypeDetector` | xsd | xml |
| ✅ | `Base64TextDataTypeDetector` | base64Text | text |
| ✅ | `Base64ImageDataTypeDetector` | base64Image | text |
| ✅ | `GZipTextDataTypeDetector` | gzip | text |
| ✅ | `DateDataTypeDetector` | date | text |
| ✅ | `ImageDataTypeDetector` | image | — |
| ✅ | `FilesDataTypeDetector` | files | — |
| ✅ | `FileDataTypeDetector` | file | files |
| ✅ | `ImageFileDataTypeDetector` | imageFile | file |

### 3.5 官方默认 30 工具：产品已规划，本仓未编入

实现与 MEF Export 均在 [DevToys.Tools](https://github.com/DevToys-app/DevToys.Tools)。正式安装包把该仓打成插件放进 `Plugins` 后，运行时才「接入」宿主。

清点：GUI **30**（与侧栏项 1:1）；CLI **26**（JWT、正则、文本比较、Markdown 预览无 CLI）。功能规格见 [§9](#9-默认工具功能需求30)。

| 分组 | 中文名 | `[Name]` | CLI | 本仓源码 |
|---|---|---|---|---|
| 转换器 | Cron 解析器 | `CronParser` | `cronparser` / `cron` | ❌ |
| 转换器 | 日期 | `DateConverter` | `date` | ❌ |
| 转换器 | JSON > 表格 | `JsonTableConverter` | `jsonToTable` | ❌ |
| 转换器 | JSON <> YAML | `JsonYamlConverter` | `jsonToYaml` | ❌ |
| 转换器 | 数字进制 | `NumberBaseConverter` | `numberbase` / `nb` | ❌ |
| 编解码 | Base64 文本 | `Base64TextEncoderDecoder` | `base64` / `b64` | ❌ |
| 编解码 | Base64 图片 | `Base64ImageEncoderDecoder` | `base64img` / `b64i` | ❌ |
| 编解码 | 证书 | `CertificateDecoder` | `certificate` / `cert` | ❌ |
| 编解码 | GZip | `GZipEncoderDecoder` | `gzip` | ❌ |
| 编解码 | HTML | `HtmlEncoderDecoder` | `html` | ❌ |
| 编解码 | JWT | `JsonWebTokenEncoderDecoder` | 无 | ❌ |
| 编解码 | 二维码 | `QRCodeEncoderDecoder` | `qrcode` | ❌ |
| 编解码 | URL | `UrlEncoderDecoder` | `url` | ❌ |
| 格式化 | JSON | `JsonFormatter` | `JsonFormatter` / `Jsonf` | ❌ |
| 格式化 | SQL | `SqlFormatter` | `sqlFormatter` / `sqlf` | ❌ |
| 格式化 | XML | `XmlFormatter` | `xmlFormatter` / `xmlf` | ❌ |
| 生成器 | 哈希 / 校验和 | `HashAndChecksumGenerator` | `checksum` / `hash` | ❌ |
| 生成器 | 乱数假文 | `LoremIpsumGenerator` | `loremipsum` / `li` | ❌ |
| 生成器 | 密码 | `PasswordGenerator` | `password` / `pwd` | ❌ |
| 生成器 | UUID | `UUIDGenerator` | `uuid` / `guid` | ❌ |
| 图像 | 色盲模拟器 | `ColorBlindnessSimulator` | `colorblindsimulator` / `cbs` | ❌ |
| 图像 | 图片格式转换器 | `ImageConverter` | `imageconverter` / `imgconv` | ❌ |
| 测试 | JSONPath | `JSONPathTester` | `jsonpathtester` / `jpt` | ❌ |
| 测试 | 正则表达式 | `RegExTester` | 无 | ❌ |
| 测试 | XML / XSD | `XMLTester` | `xmltester` / `xsd` | ❌ |
| 文本 | 文本分析和实用工具 | `TextAnalyzerAndUtilities` | `textutilities` / `txt` | ❌ |
| 文本 | 比较 | `TextCompare` | 无 | ❌ |
| 文本 | 转义 / 反转义 | `EscapeUnescape` | `escape` / `esc` | ❌ |
| 文本 | 列表比对 | `ListCompare` | `listcompare` / `lc` | ❌ |
| 文本 | Markdown 预览 | `MarkdownPreview` | 无 | ❌ |

运行时如何出现：一律靠 `Plugins`。本仓 **无** `[Export(typeof(ICommandLineTool))]`。

### 3.6 未接入（易误判）

| 项 | 为何不像已接入 |
|---|---|
| Color Picker | 仅 `assets/font/ColorPicker.svg` 等图标槽，无 `IGuiTool` |
| PNG/JPEG Compressor | 同上；2.0 起为第三方扩展，不在默认 30 |
| Support Development | 类存在，Export 注释掉 |
| 社区扩展（JSON to C#、PNG Compressor、Msgpack…） | changelog 提及商店/扩展目录，不在本仓 |

### 3.7 本地只编宿主时的可见结果

侧栏预期：

- 有：全部工具、收藏、7 个空分组、Footer「扩展管理」「设置」
- 无：JSON Formatter、Base64 等 30 个默认工具（除非另行按 DevToys.Tools 文档把插件拷入 `Plugins`）

---

## 4. 范围

### 4.1 范围内

- 宿主壳：Win / macOS / Linux GUI，以及 CLI 宿主
- 底部内置工具：设置、扩展管理
- 7 个工具分组骨架 + 全部工具 / 收藏 / 最近使用
- Smart Detection 与 13 个宿主内置数据类型
- 默认 30 工具的功能规格（实现在 DevToys.Tools）
- 扩展加载契约、设置项、无障碍与本地化约束

### 4.2 范围外

- 社区第三方扩展的具体功能（JSON to C#、PNG 压缩器等）
- 未启用的「支持开发」Footer 工具（`SupportDevelopmentGuidTools`，`[Export]` 已注释）
- 1.x 遗留能力且 2.0 默认集已移除者（如 Color Picker；图标资源仍可能残留）
- 新工具的设计与实现
- 云同步、账号体系、遥测上报用户输入内容
- WASM / 浏览器版产品化（API 上可有 `Platform.WASM` 标记，本需求不覆盖上线）

---

## 5. 角色

| 角色 | 说明 |
|---|---|
| 桌面开发者 | 主用户。在本机处理文本、结构化数据、证书、图片、哈希等 |
| CLI 用户 | 用命令行调用同一批工具，做脚本化转换 |
| 扩展开发者 | 按 `IGuiTool` / `ICommandLineTool` 契约发布 nupkg |
| 首次使用者 | 需要首次启动引导、更新后「新功能」提示 |
| 读屏用户 | 依赖 AccessibleName、分组/工具标题可读 |

---

## 6. 总体架构需求

### 6.1 两仓分工

| 仓 | 必须提供 | 不得包含 |
|---|---|---|
| 宿主（本仓库） | 窗口、MEF 组合、分组、设置、扩展管理、通用检测器、UI 控件实现 | JSON/Base64 等业务工具实现 |
| DevToys.Tools | 30 个默认 GUI/CLI 工具、工具侧检测器 | 窗口与导航壳 |

本地仅编译宿主时，侧栏只有分组骨架 + Footer（设置、扩展管理），**没有** JSON Formatter 等默认工具。带默认工具运行须按 DevToys.Tools 的 CONTRIBUTING 把插件拷入 `Plugins`。

### 6.2 发现与加载

- 用 MEF 扫描：
  - 应用目录下 `Plugins`
  - 用户缓存目录下 `Plugins`（各平台 `Constants.PluginInstallationFolder`）
- 每个 GUI 工具导出 `IGuiTool`，带 `[Name]`、`[ToolDisplayInformation]`
- 每个 CLI 工具导出 `ICommandLineTool`，带 `[CommandName]`
- 分组导出 `GuiToolGroup`，`[Name]` 必须等于工具的 `GroupName`
- 可选：`[TargetPlatform]`、`[Order]`、`[MenuPlacement]`、`[NotSearchable]`、`[NotFavorable]`、`[NoCompactOverlaySupport]`、`[AcceptedDataTypeName]`

### 6.3 工具契约（宿主对扩展的接口）

**GUI 工具 `IGuiTool`**

- 必须提供 `UIToolView View`：用宿主 `IUI*` 控件声明界面，不写 Razor
- 必须实现 `OnDataReceived(dataTypeName, parsedData)`：Smart Detection 命中并导航过来时，把解析结果填进对应输入控件
- 不得在工具内直接 new 平台依赖（剪贴板、文件、设置、字体须通过注入的 `IClipboard` / `IFileStorage` / `ISettingsProvider` / `IFontProvider`）

**CLI 工具 `ICommandLineTool`**

- `InvokeAsync(ILogger, CancellationToken)` 返回进程退出码
- 选项用 `[CommandLineOption]` 声明
- Logger：可报错误与性能；**禁止**把用户输入当作遥测内容上报

**分组 `GuiToolGroup`**

- 提供图标字体名、Glyph、显示标题、AccessibleName

---

## 7. 宿主功能需求

### 7.1 主窗口与导航

| ID | 需求 | 优先级 |
|---|---|---|
| H-NAV-01 | 侧栏展示：全部工具、收藏、最近使用（可关）、7 个业务分组、Footer 工具 | 必须 |
| H-NAV-02 | 预留组名 `AllTools`、`FavoriteTools` 不可被扩展占用 | 必须 |
| H-NAV-03 | 业务分组固定 7 个：转换器、编解码器、格式化工具、生成器、图像处理、测试工具、文本处理 | 必须 |
| H-NAV-04 | 工具可收藏；标记 `[NotFavorable]` 的不得出现收藏入口 | 必须 |
| H-NAV-05 | 记录最近 3 个工具（`RecentTool1..3`）；设置可关闭「最近使用」区块 | 必须 |
| H-NAV-06 | 搜索：模糊匹配显示名与 SearchKeywords；`[NotSearchable]` 不进搜索 | 必须 |
| H-NAV-07 | 无搜索结果时展示「无结果」占位项 | 必须 |
| H-NAV-08 | Footer 顺序：扩展管理在设置之前（若该平台有扩展管理） | 必须 |
| H-NAV-09 | 扩展管理仅 Win/Linux/macOS；设置全平台 | 必须 |
| H-NAV-10 | 记住主窗口位置/大小与最大化状态 | 必须 |
| H-NAV-11 | Compact Overlay：工具未标 `[NoCompactOverlaySupport]` 才支持小窗置顶 | 应该 |

### 7.2 首次启动与更新

| ID | 需求 |
|---|---|
| H-LIFE-01 | `IsFirstStart` 为真时展示首次启动对话框 |
| H-LIFE-02 | `LastVersionRan` 与当前版本不同时，可展示 What’s New |
| H-LIFE-03 | 设置可开关自动检查更新（默认关） |

### 7.3 设置工具（`SettingsGuiTool`，内部名 `Settings`）

Footer、不可收藏、不可搜索、不支持 Compact Overlay。

**外观**

- 界面语言：系统默认或已安装语言；提供「帮助翻译」外链
- 主题：跟随系统 / 浅色 / 深色
- 紧凑模式

**行为**

- 自动检查更新
- 显示最近使用的工具
- Smart Detection 总开关；关闭后「推荐工具自动粘贴」不可用
- Smart Detection 命中并打开工具时，是否自动粘贴剪贴板（默认开）

**编辑器（Monaco 类文本框全局默认）**

- 字体：系统已装字体，优先 Fira Code、Source Code Pro、Cascadia、Menlo、Consolas 等开发者等宽字体
- 自动换行、行号（默认开）、高亮当前行（默认开）、渲染空白、EOL（自动 / LF / CRLF）
- 粘贴是否清空原文本（默认是）
- 设置页内提供 JSON 样例预览

**关于**

- 显示应用版本，可一键复制
- 致谢入口（图标设计、DevToys Mac 作者等）

### 7.4 扩展管理（`ExtensionsManagerGuiTool`，内部名 `Extensions Manager`）

| ID | 需求 |
|---|---|
| H-EXT-01 | 列出已安装扩展（名称、大小等） |
| H-EXT-02 | 从本地 nupkg 安装；安装后提示重启 |
| H-EXT-03 | 卸载扩展；卸载后提示重启 |
| H-EXT-04 | 安装须遵守扩展条款；危险操作前警告 |
| H-EXT-05 | 仅桌面三平台导出；CLI/非桌面不出现该 Footer |

### 7.5 通用文本输入行为

工具内多行/单行文本框必须遵循全局编辑器设置，并提供统一包装能力：从剪贴板粘贴、从文件打开、复制输出、清空。粘贴策略受 `TextEditorPasteClearsText` 控制。

### 7.6 UI 控件清单（工具只能用这些声明界面）

布局：Stack、Grid、SplitGrid、Wrap、Card  
输入：单行/多行文本、Diff 文本、数字、密码、文件选择、开关、下拉  
展示：Label、Icon、InfoBar、进度条/环、DataGrid、List、ImageViewer、WebView、Setting / SettingGroup

工具不得绕过该层直接操作 DOM/平台控件（宿主实现除外）。

---

## 8. Smart Detection 需求

### 8.1 行为

| ID | 需求 |
|---|---|
| SD-01 | 设置开启时，监视剪贴板；检测须离开 UI 线程，单次检测可被 2 秒级取消 |
| SD-02 | 检测器按 `baseName` 组成类型树；子类型比父类型更具体、优先级更高 |
| SD-03 | `strict=true`：只返回最贴合叶子类型的工具 |
| SD-04 | `strict=false`：返回叶子及其直接父类型工具；不返回更远的祖先（例：JWT-Header > JSON > Text，命中 JWT-Header 时给 JWT 与 JSON 工具，不给纯 Text 工具） |
| SD-05 | 非严格模式下，当前已打开的工具不进入推荐列表 |
| SD-06 | 推荐工具在 UI 上有明确指示（如灯泡）；用户点选后导航到该工具并调用 `OnDataReceived` |
| SD-07 | `SmartDetectionPaste` 为真时自动把解析数据填入工具；为假时只推荐不自动填 |
| SD-08 | 总开关关闭时不做检测、不提示 |

### 8.2 宿主内置数据类型

| 类型名 | 含义 | 父类型 |
|---|---|---|
| `text` | 普通文本 | — |
| `json` | 合法 JSON（排除纯长整型误判） | text |
| `jsonArray` | JSON 数组 | json |
| `xml` | XML | text |
| `xsd` | XSD | xml |
| `base64Text` | Base64 文本 | text |
| `base64Image` | Base64 图片 | text |
| `gzip` | GZip 文本载荷 | text |
| `date` | 日期/时间戳文本 | text |
| `image` | 内存图像 | — |
| `files` | 多文件 | — |
| `file` | 单文件 | files |
| `imageFile` | 图像文件（扩展名在支持列表内） | file |

工具仓另导出 8 个领域类型（详见 [§9.8](#98-工具仓追加的检测器8)）：`Yaml`、`Certificate`、`TextWithEscapedCharacters`、`Markdown`、`NumberBase`、`StaticImageFile`、`StaticImageFiles`、`Base64ImageFile`。工具用 `[AcceptedDataTypeName]` 声明接受哪些类型。

---

## 9. 默认工具功能需求（30）

**接入状态：本仓库未编入。** 见 [§3.5](#35-官方默认-30-工具产品已规划本仓未编入)。下列按 [DevToys.Tools](https://github.com/DevToys-app/DevToys.Tools) 当前源码整理（30 个 `[Export(typeof(IGuiTool))]`，26 个 `[Export(typeof(ICommandLineTool))]`）。发行包把该仓打成插件放入 `Plugins` 后，侧栏才出现这些工具。

宿主只保证：7 个分组存在、通用检测器可把数据交给这些工具（若插件已加载）。

下列每条均默认：离线工作、输入不上传、输出可复制。CLI 输入多为 `OneOf<FileInfo, string>`（文件或内联文本），`-o` 写文件，缺省打印到 stdout。

**计数**：转换器 5 + 编解码 8 + 格式化 3 + 生成器 4 + 图像 2 + 测试 3 + 文本 5 = **30**。JWT 侧栏是 **1** 个工具（内部 Encode/Decode 子视图），不拆成两个默认工具。

**无 CLI 的 4 个**：JWT、正则表达式测试、文本比较、Markdown 预览（仅 GUI）。

### 9.0 总表

| ID | 分组 | 中文名 | `[Name]` | GUI 类 | CLI 命令 / 别名 | Smart Detection |
|---|---|---|---|---|---|---|
| T-CV-01 | 转换器 | Cron 解析器 | `CronParser` | `CronParserGuiTool` | `cronparser` / `cron` | 无 |
| T-CV-02 | 转换器 | 日期 | `DateConverter` | `DateConverterGuiTool` | `date` | `date` |
| T-CV-03 | 转换器 | JSON > 表格 | `JsonTableConverter` | `JsonTableConverterGuiTool` | `jsonToTable` | `jsonArray` |
| T-CV-04 | 转换器 | JSON <> YAML | `JsonYamlConverter` | `JsonYamlConverterGuiTool` | `jsonToYaml` | `json`、`Yaml` |
| T-CV-05 | 转换器 | 数字进制 | `NumberBaseConverter` | `NumberBaseConverterGuiTool` | `numberbase` / `nb` | `NumberBase` |
| T-ED-01 | 编解码 | Base64 文本 | `Base64TextEncoderDecoder` | `Base64TextEncoderDecoderGuiTool` | `base64` / `b64` | `base64Text`、`text` |
| T-ED-02 | 编解码 | Base64 图片 | `Base64ImageEncoderDecoder` | `Base64ImageEncoderDecoderGuiTool` | `base64img` / `b64i` | `base64Image`、`image`、`Base64ImageFile` |
| T-ED-03 | 编解码 | 证书 | `CertificateDecoder` | `CertificateDecoderGuiTool` | `certificate` / `cert` | `Certificate` |
| T-ED-04 | 编解码 | GZip | `GZipEncoderDecoder` | `GZipEncoderDecoderGuiTool` | `gzip` | `gzip` |
| T-ED-05 | 编解码 | HTML | `HtmlEncoderDecoder` | `HtmlEncoderDecoderGuiTool` | `html` | 未接线 |
| T-ED-06 | 编解码 | JWT | `JsonWebTokenEncoderDecoder` | `JsonWebTokenEncoderDecoderGuiTool` | 无 CLI | 未接线 |
| T-ED-07 | 编解码 | 二维码 | `QRCodeEncoderDecoder` | `QRCodeEncoderDecoderGuiTool` | `qrcode` | `image`、`text` |
| T-ED-08 | 编解码 | URL | `UrlEncoderDecoder` | `UrlEncoderDecoderGuiTool` | `url` | 未接线 |
| T-FM-01 | 格式化 | JSON | `JsonFormatter` | `JsonFormatterGuiTool` | `JsonFormatter` / `Jsonf` | `json` |
| T-FM-02 | 格式化 | SQL | `SqlFormatter` | `SqlFormatterGuiTool` | `sqlFormatter` / `sqlf` | 无 |
| T-FM-03 | 格式化 | XML | `XmlFormatter` | `XmlFormatterGuiTool` | `xmlFormatter` / `xmlf` | `xml` |
| T-GN-01 | 生成器 | 哈希 / 校验和 | `HashAndChecksumGenerator` | `HashAndChecksumGeneratorGuiTool` | `checksum` / `hash` | `text`、`file` |
| T-GN-02 | 生成器 | 乱数假文 | `LoremIpsumGenerator` | `LoremIpsumGeneratorGuiTool` | `loremipsum` / `li` | 无 |
| T-GN-03 | 生成器 | 密码 | `PasswordGenerator` | `PasswordGeneratorGuidTool` | `password` / `pwd` | 无 |
| T-GN-04 | 生成器 | UUID | `UUIDGenerator` | `UUIDGeneratorGuidTool` | `uuid` / `guid` | 无 |
| T-GR-01 | 图像 | 色盲模拟器 | `ColorBlindnessSimulator` | `ColorBlindnessSimulatorGuiTool` | `colorblindsimulator` / `cbs` | `image`、`StaticImageFile` |
| T-GR-02 | 图像 | 图片格式转换器 | `ImageConverter` | `ImageConverterGuiTool` | `imageconverter` / `imgconv` | `image`、`StaticImageFile`、`StaticImageFiles` |
| T-TS-01 | 测试 | JSONPath | `JSONPathTester` | `JsonPathTesterGuiTool` | `jsonpathtester` / `jpt` | `json` |
| T-TS-02 | 测试 | 正则表达式 | `RegExTester` | `RegExTesterGuiTool` | 无 CLI | `text` |
| T-TS-03 | 测试 | XML / XSD | `XMLTester` | `XMLTesterGuiTool` | `xmltester` / `xsd` | `xml`、`xsd` |
| T-TX-01 | 文本 | 文本分析和实用工具 | `TextAnalyzerAndUtilities` | `AnalyzerAndUtilitiesGuiTool` | `textutilities` / `txt` | `text` |
| T-TX-02 | 文本 | 比较 | `TextCompare` | `TextCompareGuiTool` | 无 CLI | 无 |
| T-TX-03 | 文本 | 转义 / 反转义 | `EscapeUnescape` | `EscapeUnescapeGuiTool` | `escape` / `esc` | `TextWithEscapedCharacters` |
| T-TX-04 | 文本 | 列表比对 | `ListCompare` | `ListCompareGuiTool` | `listcompare` / `lc` | 无 |
| T-TX-05 | 文本 | Markdown 预览 | `MarkdownPreview` | `MarkdownPreviewGuiTool` | 无 CLI | `Markdown` |

「未接线」= GUI 有 `OnDataReceived`，但实现为 `throw new NotImplementedException()`，剪贴板命中后不会自动填入。

### 9.1 转换器（Converters）

#### T-CV-01 Cron 解析器

- **目的**：解析 Cron 表达式，给出人类可读说明与即将触发的时间列表
- **输入**：Cron 表达式；日期格式字符串（默认 `yyyy-MM-dd ddd HH:mm:ss`）
- **输出**：语义说明 + 下次执行时刻列表
- **GUI 选项**：
  - 包含秒字段（默认开；开则默认 `* * * * * *`，关则 `* * * * *`）
  - 预览条数：5 / 10 / 25 / 50 / 100（默认 5）
- **CLI**：`cronparser -e <表达式> [-s] [-c <条数>] [-d <日期格式>]`
- **Smart Detection**：无对应检测器；`OnDataReceived` 空实现
- **错误**：非法表达式走 InfoBar，不静默吞掉

#### T-CV-02 日期 / 时间戳

- **目的**：Unix 时间戳 ↔ 可读日期时间
- **输入**：时间戳或日期字符串；检测器给出 `DateTimeOffset`
- **输出**：按所选格式/时区同步更新对向结果
- **GUI 选项**：
  - 格式：`Ticks` / `Seconds`（默认）/ `Milliseconds`
  - 时区：系统时区 ID（默认本机）
  - 自定义 Epoch（默认关；默认 Epoch 为 Unix 纪元）
- **CLI**：`date -i <时间戳或日期> [-e <epoch>] [-tz <时区>] [-f Ticks|Seconds|Milliseconds]`
- **Smart Detection**：宿主 `date`；命中后填入日期控件

#### T-CV-03 JSON 数组 → 表格 / CSV

- **目的**：把 JSON 对象数组转成表格，可复制或导出
- **输入**：JSON 数组（须为对象数组，否则无法成表）
- **输出**：DataGrid；导出格式 TSV / CSV / FSV（分号分隔，法式 CSV）
- **GUI**：输入区 + 表格；可复制到剪贴板或保存文件
- **CLI**：`jsonToTable -i <文件或文本> [-o <文件>] [--format TSV|CSV|FSV]`
- **Smart Detection**：宿主 `jsonArray`

#### T-CV-04 JSON ↔ YAML

- **目的**：JSON 与 YAML 互转
- **输入**：JSON 或 YAML 文本
- **输出**：另一侧格式；非法输入报错，不静默损坏数据
- **GUI 选项**：
  - 方向：JSON→YAML（默认）/ YAML→JSON；Smart Detection 命中后自动切方向并设语言高亮
  - 缩进：两空格（默认）/ 四空格 / Tab / Minified
- **CLI**：`jsonToYaml -i <输入> -c JsonToYaml|YamlToJson [-o <文件>] [--indentation …]`
- **Smart Detection**：宿主 `json`；工具仓 `Yaml`（父类型 `text`）

#### T-CV-05 数字进制

- **目的**：数字在不同进制/编码间转换
- **基础模式**（默认；Smart Detection 命中后强制切回基础模式）：十六进制、十进制、八进制、二进制四框同步；可选千分位格式化
- **高级模式**：输入/输出字典可选 RFC 4648 Base16 / Base32 / Base32 Extended Hex / Base64 / Base64 URL，或自定义字符表
- **CLI**：仅基础四进制。`numberbase -i <值> [-b Decimal|Octal|Hexadecimal|Binary] [-o <输出进制>]`
- **Smart Detection**：工具仓 `NumberBase`（父类型 `text`）

### 9.2 编解码器（Encoders / Decoders）

#### T-ED-01 Base64 文本

- **目的**：文本 ↔ Base64（RFC 4648）
- **输入 / 输出**：两侧文本框；编/解码开关
- **GUI 选项**：Encode / Decode；字符集 UTF-8（默认）或 ASCII；多行模式
- **CLI**：`base64 -i <输入> [-c Encode|Decode] [-e Utf8|Ascii] [-o <文件>]`
- **Smart Detection**：`base64Text` → 自动切 Decode；普通 `text` → 切 Encode

#### T-ED-02 Base64 图片

- **目的**：图片文件/像素 ↔ Base64 字符串（含 data URI 场景）
- **输入**：Base64 文本、剪贴板图像、或图片文件
- **输出**：对向的图像预览或 Base64 文本
- **CLI**：`base64img -i <文件或 Base64> [-o <文件>]`
- **Smart Detection**：`base64Image`、`image`、`Base64ImageFile`（工具仓，父类型 `file`）

#### T-ED-03 证书

- **目的**：解码证书，展示主题、颁发者、有效期、指纹等可读信息
- **输入**：PEM / CER / CRT 文本或文件；PFX 可带密码（CLI `-p`）
- **输出**：解码后的证书文本
- **CLI**：`certificate -i <输入> [-p <密码>] [-o <文件>]`
- **Smart Detection**：工具仓 `Certificate`（父类型 `text`，PEM 等无密码可识别）

#### T-ED-04 GZip

- **目的**：文本压缩/解压为 GZip（传输形态常见为压缩后再 Base64）
- **GUI 选项**：Compress / Decompress；命中 `gzip` 时自动切解压
- **CLI**：`gzip -i <文本> [-m Compress|Decompress]`
- **Smart Detection**：宿主 `gzip`

#### T-ED-05 HTML

- **目的**：字符 ↔ HTML 实体编解码
- **GUI 选项**：Encode / Decode
- **CLI**：`html -i <文本> [-c Encode|Decode]`
- **Smart Detection**：未声明 `AcceptedDataTypeName`；`OnDataReceived` 抛 `NotImplementedException`

#### T-ED-06 JWT（JsonWebToken）

- **目的**：编码或解码 JSON Web Token。侧栏 **一个** 工具，开关切换 Encode / Decode 子视图（`JwtMode`，默认 Decode）
- **解码子视图**（`JsonWebTokenDecoderGuiTool`，不单独 Export）：
  - 输入 Token，输出 Header / Payload / Signature
  - 可选校验：签名密钥、Issuer、Audience、Lifetime、Actor；密钥可标为 Base64
- **编码子视图**（`JsonWebTokenEncoderGuiTool`，不单独 Export）：
  - 算法默认 HS256；另支持 HS384/512、RS256/384/512、ES256/384/512、PS256/384/512
  - 可选 Issuer、Audience、默认时间声明、过期时间、Base64 密钥
- **CLI**：无
- **Smart Detection**：父工具未声明接受类型；`OnDataReceived` 抛 `NotImplementedException`。Payload 虽为 JSON，不会因此自动推荐本工具

#### T-ED-07 二维码

- **目的**：文本生成二维码；从图像读取二维码。可导出 SVG
- **输入**：文本（编码）或图像（解码）
- **输出**：二维码预览图或解码文本
- **CLI**：`qrcode -i <文本或图像文件> [-o <文件>]`
- **Smart Detection**：`text` 填入输入框编码；`image` 走文件选择解码

#### T-ED-08 URL

- **目的**：字符 ↔ URL percent-encoding
- **GUI 选项**：Encode / Decode；多行模式
- **CLI**：`url -i <文本> [-c Encode|Decode]`
- **Smart Detection**：同 HTML，未接线

### 9.3 格式化工具（Formatters）

缩进枚举共用：`TwoSpaces`（默认）/ `FourSpaces` / `OneTab` / `Minified`。

#### T-FM-01 JSON Formatter

- **目的**：美化或压缩 JSON；可选按属性名排序
- **非法 JSON 报错**，不静默损坏数据
- **CLI**：`JsonFormatter -i <输入> [-o <文件>] [--indentation …] [--sortProperties]`
- **Smart Detection**：`json`

#### T-FM-02 SQL Formatter

- **目的**：按方言美化 SQL
- **GUI 选项**：缩进；方言默认 `Sql`，另有 Tsql / Spark / RedShift / PostgreSql / PlSql / N1ql / MySql / MariaDb / Db2；前导逗号（默认关）
- **CLI**：`sqlFormatter -i <输入> [-o <文件>] [--indentation …] [--language …] [--leadingComma]`
- **Smart Detection**：无

#### T-FM-03 XML Formatter

- **目的**：美化或压缩 XML
- **GUI 选项**：缩进；属性换行（默认关）
- **CLI**：`xmlFormatter -i <输入> [-o <文件>] [--indentation …] [--newLineOnAttributes]`
- **Smart Detection**：`xml`

### 9.4 生成器（Generators）

#### T-GN-01 哈希 / 校验和

- **目的**：对文本或文件计算哈希；可 HMAC；可与期望校验和比对
- **算法**：MD5（默认）/ SHA1 / SHA256 / SHA384 / SHA512；可选 HMAC 密钥；输出大小写（默认小写）
- **文件**：走 `IFileStorage` / `SandboxedFileReader`，不绕过权限
- **CLI**：`checksum -i <文本或文件> [-a Md5|Sha1|Sha256|Sha384|Sha512] [-u] [-m <hmac>] [-c <期望校验和>] [-s]`
- **Smart Detection**：`text`、`file`

#### T-GN-02 乱数假文

- **目的**：生成占位文案
- **选项**：语料（默认 LoremIpsum，另有 ChildHarold、Decameron、Faust 等 12 种）；单位 Paragraphs / Sentences / Words / Characters；长度
- **CLI**：`loremipsum [-c <语料>] [-t <单位>] [-l <长度>]`
- **Smart Detection**：无

#### T-GN-03 密码

- **目的**：按字符集生成随机密码
- **默认**：大写、小写、数字、特殊字符均开；长度 30；一次生成 1 条；可排除指定字符
- **CLI**：`password [-l <长度>] [-u] [-m 小写] [-d] [-s 特殊] [-e <排除>]`
- **Smart Detection**：无

#### T-GN-04 UUID

- **目的**：生成 UUID v1 / v4（默认）/ v7
- **选项**：带连字符（默认开）；大写（默认关）；批量条数（默认 1）
- **CLI**：`uuid [-v One|Four|Seven] [-h] [-u]`
- **Smart Detection**：无

### 9.5 图像处理（Graphic）

静态图扩展名（工具仓 `StaticImageFile`）：`.bmp` `.jpeg` `.jpg` `.pbm` `.png` `.tiff` `.tga` `.webp`。

#### T-GR-01 色盲模拟器

- **目的**：对输入图模拟色盲，输出对照预览
- **输出四宫格**：原图、红色盲（Protanopia）、绿色盲（Deuteranopia）、黄蓝色盲（Tritanopia）；Brettel 1997 模型，severity=1
- **CLI**：`colorblindsimulator -i <图像> [-o <输出目录>] [-s]`
- **Smart Detection**：`image`、`StaticImageFile`

#### T-GR-02 图片格式转换器

- **目的**：无损转换图片格式；支持多文件
- **目标格式**：BMP / JPEG / PBM / PNG / TGA / TIFF / WEBP（GUI 记忆上次选择）
- **CLI**：`imageconverter -i <文件或目录> -t <格式> [-o <输出>]`
- **Smart Detection**：`image`、`StaticImageFile`、`StaticImageFiles`

> 1.x / 官网早期文案中的 PNG/JPEG Compressor **不是** 2.0 默认 30 工具，属第三方扩展，本文件不写功能规格。

### 9.6 测试工具（Testers）

#### T-TS-01 JSONPath

- **目的**：对 JSON 执行 JSONPath，列出匹配
- **输入**：JSON + JSONPath 表达式
- **CLI**：`jsonpathtester -j <JSON> -p <path> [-o <文件>]`
- **Smart Detection**：`json` 填入 JSON 框

#### T-TS-02 正则表达式

- **目的**：表达式 + 样例文本；展示匹配与分组；内置速查表
- **选项**（均可记忆）：全部匹配、ECMAScript、区域固定、忽略大小写、忽略空白、Singleline、Multiline、从右向左
- **约束**：`[NoCompactOverlaySupport]`（界面过挤）；非法正则报错，检测有超时，不得卡死 UI
- **CLI**：无
- **Smart Detection**：`text` 填入样例文本

#### T-TS-03 XML / XSD

- **目的**：用 XSD 校验 XML，列出错误位置与级别（Success / Warning / Error）
- **CLI**：`xmltester -s <XSD> -x <XML>`
- **Smart Detection**：`xml` 填 XML 框；`xsd` 填 XSD 框

### 9.7 文本处理（Text）

#### T-TX-01 文本分析和实用工具

- **目的**：统计 + 常见文本变换
- **统计**：字节、字符、词、句、段、行、EOL 类型、字符/词频；选区长度与行列
- **变换**：
  - 换行：LF / CRLF
  - 大小写：lower / UPPER / Sentence / Title / camel / Pascal / snake / CONSTANT / kebab / COBOL / Train / aLtErNaTiNg / Inverse / Random
  - 行：字母序 / 倒序字母序 / 按末词 / 按末词倒序 / 反转行 / 打乱行
- **CLI**：`textutilities -i <输入> -a <一个或多个 OperationType> [-o <文件>]`
- **Smart Detection**：`text`

#### T-TX-02 文本比较

- **目的**：两侧文本 Diff 高亮（宿主 `IUIDiffTextInput`）
- **选项**：并排（默认）或行内（inline）
- **CLI**：无
- **Smart Detection**：无（`OnDataReceived` 空）

#### T-TX-03 转义 / 反转义

- **目的**：转义或反转义字符串，去掉会阻碍解析的字符
- **GUI 选项**：Encode / Decode；命中转义文本时自动切 Unescape
- **CLI**：`escape -i <输入> [-c Encode|Decode] [-o <文件>]`
- **Smart Detection**：工具仓 `TextWithEscapedCharacters`（父类型 `text`）

#### T-TX-04 列表比对

- **目的**：两列表求交 / 并 / 差
- **模式**：`AInterB`（交，默认）/ `AUnionB`（并）/ `AOnly`（仅 A）/ `BOnly`（仅 B）；可选大小写敏感
- **CLI**：`listcompare -a <文件A> -b <文件B> [-cm AInterB|AUnionB|AOnly|BOnly] [-cs] [-o <文件>]`
- **Smart Detection**：无

#### T-TX-05 Markdown 预览

- **目的**：用类似 GitHub 的渲染器预览 Markdown
- **选项**：预览主题 Dark / Light
- **CLI**：无
- **Smart Detection**：工具仓 `Markdown`（父类型 `text`；标题行 `#` 等启发式）

### 9.8 工具仓追加的检测器（8）

相对宿主 13 个通用类型，DevToys.Tools 再导出：

| 类型名 | 父类型 | 服务工具 |
|---|---|---|
| `Yaml` | text | JSON ↔ YAML |
| `Certificate` | text | 证书 |
| `TextWithEscapedCharacters` | text | 转义 / 反转义 |
| `Markdown` | text | Markdown 预览 |
| `NumberBase` | text | 数字进制 |
| `StaticImageFile` | file | 色盲模拟、图片转换 |
| `StaticImageFiles` | files | 图片转换（多文件） |
| `Base64ImageFile` | file | Base64 图片 |

---

## 10. CLI 需求

| ID | 需求 |
|---|---|
| CLI-01 | `DevToys.CLI` 加载同一套 Plugins，把 `ICommandLineTool` 映射为子命令 |
| CLI-02 | 命令名/别名来自 `[CommandName]`；选项来自 `[CommandLineOption]` |
| CLI-03 | 退出码：0 成功，非 0 失败；尊重取消 |
| CLI-04 | 不把用户文件内容写入日志 |
| CLI-05 | 无 GUI 时不加载 Settings / Extensions Manager |
| CLI-06 | 默认 30 工具中 26 个提供 CLI；JWT、正则测试、文本比较、Markdown 预览仅 GUI |

---

## 11. 非功能需求

| ID | 类别 | 需求 |
|---|---|---|
| NFR-01 | 隐私 | 默认工具在本地完成；不把剪贴板/文件发往网络。扩展安装、检查更新除外 |
| NFR-02 | 安全 | 扩展安装有警告与条款；文件读取走沙箱 `SandboxedFileReader` |
| NFR-03 | 性能 | Smart Detection 不得阻塞 UI；检测可取消 |
| NFR-04 | 本地化 | 宿主与默认工具字符串走 resx；设置可切语言 |
| NFR-05 | 无障碍 | 分组与工具提供 AccessibleName；读屏可导航侧栏 |
| NFR-06 | 跨平台 | 同一工具逻辑在 Win/macOS/Linux 行为一致；平台差异只用 `[TargetPlatform]` 收口 |
| NFR-07 | 扩展隔离 | 扩展失败不得拖垮宿主启动（记录日志，跳过坏插件） |
| NFR-08 | 主题 | 浅色/深色/跟随系统；工具 UI 走宿主控件，自动跟主题 |

---

## 12. 用户故事

1. 作为桌面开发者，我想在本机完成 JSON/YAML 转换，以免把数据贴到陌生网站。
2. 作为桌面开发者，我想复制一段 JSON 后被推荐 JSON 格式化工具，以便少点几次导航。
3. 作为桌面开发者，我想关闭 Smart Detection，以便在处理敏感剪贴板时完全手动选工具。
4. 作为桌面开发者，我想让推荐工具自动填入剪贴板内容，以便打开即可改。
5. 作为桌面开发者，我想只推荐工具但不自动粘贴，以便先确认再填入。
6. 作为桌面开发者，我想收藏常用工具，以便侧栏置顶。
7. 作为桌面开发者，我不想把设置或扩展管理加入收藏，以免收藏列表被系统项污染。
8. 作为桌面开发者，我想看到最近 3 个用过的工具，以便重复操作。
9. 作为桌面开发者，我想关掉「最近使用」，以便侧栏更干净。
10. 作为桌面开发者，我想按名字或关键词搜索工具，以便记不清分组时仍能打开。
11. 作为桌面开发者，当搜索无结果时，我想看到明确空态，以免误以为程序坏了。
12. 作为桌面开发者，我想在浅色/深色/跟随系统间切换主题，以便配合 OS。
13. 作为桌面开发者，我想把 UI 改成我的语言，以便阅读控件标签。
14. 作为桌面开发者，我想用紧凑模式，以便小屏或小窗塞下更多控件。
15. 作为桌面开发者，我想为所有代码框统一字体（如 Cascadia / Fira Code），以便阅读。
16. 作为桌面开发者，我想开关行号、当前行高亮、空白渲染、自动换行，以便对照不同任务。
17. 作为桌面开发者，我想强制 LF 或 CRLF，以免跨平台 diff 噪声。
18. 作为桌面开发者，我想粘贴时默认覆盖而非追加，以免新旧文本混在一起。
19. 作为桌面开发者，我想在设置页预览编辑器效果，以便改设置前看到结果。
20. 作为桌面开发者，我想一键复制版本号，以便报 bug。
21. 作为桌面开发者，我想安装社区扩展 nupkg，以便获得默认 30 工具之外的能力。
22. 作为桌面开发者，我想卸载扩展，以便去掉不用的工具。
23. 作为桌面开发者，安装/卸载扩展后我想被明确告知需要重启，以免以为没生效。
24. 作为桌面开发者，我不想在 macOS/Windows 以外的不支持平台看到扩展管理。
25. 作为 CLI 用户，我想用子命令做 Base64/JSON 等转换，以便写进脚本。
26. 作为 CLI 用户，我想用短别名调用命令，以便少打字。
27. 作为 CLI 用户，我想用退出码判断成败，以便 CI 失败即停。
28. 作为扩展开发者，我想只实现 `IGuiTool` + 元数据就被侧栏收录，以便不改宿主。
29. 作为扩展开发者，我想声明 `AcceptedDataTypeName`，以便剪贴板命中我的工具。
30. 作为扩展开发者，我想用 `IUI*` 声明界面，以便自动跟主题和无障碍。
31. 作为扩展开发者，我想同时提供 CLI 命令，以便 GUI 与脚本共用逻辑。
32. 作为扩展开发者，我想限制工具只在某些 OS 出现，以免调用不存在的 API。
33. 作为首次用户，我想看到简短首次启动说明，以便理解 Smart Detection。
34. 作为升级用户，我想看到 What’s New，以便知道新扩展或修复。
35. 作为读屏用户，我想听到分组与工具的 AccessibleName，以便不用猜图标。
36. 作为桌面开发者，我想把 JWT 贴进去就看到 header/payload，以便调试 API。
37. 作为桌面开发者，我想编解码 URL/HTML 实体，以便处理接口数据。
38. 作为桌面开发者，我想编解码 Base64 文本和图片，以便看嵌入资源。
39. 作为桌面开发者，我想解 GZip 载荷，以便看压缩传输内容。
40. 作为桌面开发者，我想解析证书 PEM，以便核对过期与主题。
41. 作为桌面开发者，我想从文本生成二维码，以便分享配置或 URL。
42. 作为桌面开发者，我想美化或压缩 JSON/XML/SQL，以便阅读或减小体积。
43. 作为桌面开发者，我想对 JSON 跑 JSONPath，以便抽取嵌套字段。
44. 作为桌面开发者，我想试正则并看分组，以便写对表达式。
45. 作为桌面开发者，我想用 XSD 校验 XML，以便提交前发现 schema 错误。
46. 作为桌面开发者，我想把 JSON 数组变成表并导出 CSV，以便给非开发同事。
47. 作为桌面开发者，我想转换数字进制，以便对齐底层协议。
48. 作为桌面开发者，我想转换 Unix 时间戳，以便对日志。
49. 作为桌面开发者，我想解析 Cron，以便确认调度时间。
50. 作为桌面开发者，我想生成 UUID/密码/Lorem，以便造测试数据。
51. 作为桌面开发者，我想对文件做 SHA256 校验，以便核对下载完整性。
52. 作为桌面开发者，我想对比两段文本，以便看配置差异。
53. 作为桌面开发者，我想对比两个列表的交差集，以便对账号/ID。
54. 作为桌面开发者，我想预览 Markdown，以便发文档前看效果。
55. 作为桌面开发者，我想统计字数并做大小写变换，以便改命名。
56. 作为桌面开发者，我想转义/反转义字符串，以便塞进 JSON 或代码。
57. 作为桌面开发者，我想转换图片格式，以便资源管线统一。
58. 作为桌面开发者，我想模拟色盲看图，以便检查对比度。
59. 作为桌面开发者，我想把主窗口缩成小窗置顶（工具支持时），以便对照 IDE。
60. 作为桌面开发者，设置和扩展管理我不想进搜索，以免搜 “json” 时被系统项干扰。
61. 作为安全敏感用户，我不希望日志里出现我粘贴的密钥或文件内容。
62. 作为 Windows 用户，我想从任务栏/跳转列表进常用工具（若平台已接 Jump List）。
63. 作为 macOS 用户，我想用原生菜单栏完成关于/窗口操作，同时工具区仍是同一套 Blazor UI。
64. 作为插件损坏时的用户，我想应用仍能启动并跳过坏扩展，以便至少用设置卸掉它。

---

## 13. 实现决策

1. **宿主与工具分仓**：默认工具不得编译进宿主程序集；以 nupkg/Plugins 加载。
2. **唯一对外工具接口**：GUI 为 `IGuiTool`，CLI 为 `ICommandLineTool`。测试与调用方都走这两处，不穿透到具体工具类。
3. **分组由宿主内置**：7 个 `GuiToolGroup` 在宿主注册，工具只引用 `PredefinedCommonToolGroupNames` 的字符串，避免插件各定义一套分组。
4. **Footer 系统工具留在宿主**：设置、扩展管理依赖宿主服务（主题、NuGet 安装路径），不适合下放到 Tools 仓。
5. **Smart Detection 分层**：通用类型（text/json/xml/base64/image/file…）在宿主；领域类型（yaml/jwt/证书/markdown）在工具仓。
6. **检测树而非扁平列表**：用 `DataTypeName(baseName)` 表达特化关系，严格/非严格推荐共用一棵树。
7. **UI 声明式控件**：工具只组 `IUI*`，宿主 Blazor 负责渲染。这样 Win/macOS/Linux WebView 共用同一工具代码。
8. **设置集中在 `PredefinedSettings`**：工具可自带 `SettingDefinition<T>`，但全局外观/编辑器/检测开关只放宿主。
9. **平台过滤用特性而非运行时 if 散落**：`[TargetPlatform]` 在元数据层剔除。
10. **文件访问沙箱化**：工具拿 `SandboxedFileReader` / `IFileStorage`，不直接 `File.ReadAllText` 用户路径（平台适配器除外）。
11. **「支持开发」暂不交付**：代码保留但未 Export，本需求不要求上线。
12. **图标字体 `DevToys-Tools-Icons` 由宿主嵌入**：分组图标一致；工具也可引用 FluentSystemIcons。

### 建议测试接缝（Seam）

优先 **一个最高接缝**：`IGuiTool`（外加 CLI 的 `ICommandLineTool`）。

- 宿主：对 `GuiToolProvider`、`SmartDetectionService` 用假工具/假检测器测导航、推荐、分组
- 默认工具：对每个工具的 **纯逻辑 Helper** 测转换对错；GUI 只测 `OnDataReceived` 是否把数据送进输入控件
- 不要为每个 Razor 控件写实现细节测试

若不同意「GUI 只测 `IGuiTool` 接缝、算法测 Helper」，需在实现前改本段。

---

## 14. 测试决策

**好测试只断言对外行为**：给定输入，工具输出/退出码/检测结果是什么。不断言私有字段、控件树内部 ID（除非 ID 就是契约的一部分，如设置控件名）。

| 模块 | 测什么 | 仓库内既有参考 |
|---|---|---|
| `SmartDetectionService` | 类型树、strict、排除当前工具 | `DevToys.UnitTests` 中 MEF 测试基类 `MefBasedTest` |
| `GuiToolProvider` | 分组、Footer、搜索过滤、NotSearchable | `Mocks/Tools/MockIGuiTool*` |
| 各 `IDataTypeDetector` | 正例/反例（JSON vs 纯数字、假 Base64） | `BuiltInDataTypeDetectors` + UnitTests |
| 默认工具 Helper | 编解码往返、非法输入错误 | DevToys.Tools.UnitTests（工具仓） |
| `ICommandLineTool` | 选项绑定、退出码 | CLI + Mock |
| 设置 | 开关写入 `ISettingsProvider` 后读回 | 用假 `ISettingsStorage` |

不要求：对本需求做全量 UI 浏览器/E2E（宿主是 WebView，成本高）。回归以单元/MEF 组合测试为主。

---

## 15. 需求追踪对照

| 用户可见能力 | 实现落点 | 本仓是否已接入 |
|---|---|---|
| 设置 | 宿主 `SettingsGuiTool` + `PredefinedSettings` | ✅ 已接入 |
| 装扩展 | 宿主 `ExtensionsManagerGuiTool` | ✅ 已接入（仅桌面三平台） |
| 侧栏 7 分组 | 宿主 `BuiltInGroups` | ✅ 已接入（空骨架） |
| 通用 Smart Detection | 宿主 13 个 `IDataTypeDetector` | ✅ 已接入 |
| 30 个默认 GUI 工具 | DevToys.Tools `Tools/{Group}/{Tool}` 的 `IGuiTool` | ❌ 源码不在本仓；靠 Plugins |
| 工具仓 8 个领域检测器 | DevToys.Tools `SmartDetection/` | ❌ 源码不在本仓；靠 Plugins |
| 26 个默认 CLI 命令 | 工具仓 `ICommandLineTool` + 宿主 `DevToys.CLI` | ❌ 本仓无 CLI 业务命令；JWT/正则/文本比较/Markdown 无 CLI |
| 支持开发 | `SupportDevelopmentGuidTools` | ❌ Export 已注释 |
| 主题/多语言 | 宿主 + resx（Crowdin） | ✅ 宿主能力 |

默认 30 工具清点：转换器 5 + 编解码 8 + 格式化 3 + 生成器 4 + 图像 2 + 测试 3 + 文本 5 = **30**（JWT 侧栏 1 项，Encode/Decode 为子视图）。Tools 仓 `IGuiTool` 30、`ICommandLineTool` 26。

本仓 `Export(typeof(IGuiTool))`：**2**（Settings、Extensions Manager）。`Export(typeof(ICommandLineTool))`：**0**。

---

## 16. 附录：不要当成默认工具的资源

`assets/font/` 中存在 ColorPicker、PngJpgCompressor 等 SVG，仅表示图标字体历史槽位，**不构成** 2.0 默认工具需求。

第三方扩展示例（changelog / 官网，非本需求）：JSON to Python、Geo、Msgpack、C# to TypeScript、PNG Compressor、JSON Schema 等。

---

## 17. 修订记录

| 日期 | 说明 |
|---|---|
| 2026-08-26 | 初稿。根据本仓库代码与 DevToys.Tools 目录、README、官网默认工具列表整理 |
| 2026-08-26 | 增补 §3「当前已接入清单」：本仓 2 个 GUI 工具、7 分组、13 检测器；30 个默认工具未编入本仓 |
| 2026-08-26 | §9 按 DevToys.Tools 源码展开 30 个默认业务工具：目的、I/O、选项、GUI/CLI 类与命令、Smart Detection；§3.5 / §8.2 / §10 / §15 同步 |
