# AGENTS.md

基于 Rust + egui / eframe 的跨平台开发者工具箱，提供原生桌面 GUI 与无头 CLI 命令行支持。

## Project map

- `crates/api/` - 公共契约与核心类型（ToolMetadata, GroupId, Detector trait, AppSettings）
- `crates/core/` - 调度引擎与状态持久化（ToolRegistry, DetectionEngine, SettingsStore）
- `crates/tools/` - 30+ 业务工具实现（纯函数 helper、CLI 子命令、egui 视图与类型探测器）
- `crates/host/` - egui 桌面 GUI 宿主（窗口外壳、侧栏导航、全局设置与剪贴板监听）
- `crates/cli/` - 命令行程序入口（子命令路由与输入输出管道）
- `docs/agents/` - 智能体工程协作规范（Issue 追踪、Triage 标签、Domain 文档规则）
- `.scratch/` - 本地特性规划与分解工单（Spec 规格与结构化 Issue）

<important if="you need to run commands to build, test, lint, or generate code">

从仓库根目录执行 Cargo 命令：

| Command | What it does |
|---|---|
| `cargo build` | 编译工作区所有 crate |
| `cargo build -p devtoys` | 编译桌面 GUI 客户端 |
| `cargo build -p devtoys-cli` | 编译 CLI 命令行可执行文件 |
| `cargo test` | 运行工作区全部单元测试与集成测试 |
| `cargo test -p <package>` | 运行指定 crate 测试（如 `devtoys-core`, `devtoys-tools`） |
| `cargo clippy` | 执行代码静态代码分析与检查 |
| `cargo fmt --check` | 检查 Rust 代码格式规范 |
| `cargo run -p devtoys` | 启动桌面客户端进行交互调试 |
| `cargo run -p devtoys-cli -- <command>` | 运行指定工具的命令行模式（如 `JsonFormatter -i "{}"`） |
</important>

<important if="you are adding, modifying, or registering a tool">
- 每个工具位于 `crates/tools/src/<tool_name>/`，包含 `mod.rs`（元数据与 `Tool` trait 实现）、`helper.rs`（核心算法）、`view.rs`（egui 视图），有命令行子命令的工具还包含 `cli.rs`（CLI 适配器）
- 业务逻辑与数据转换算法必须放在 `helper.rs`，严禁在 `view.rs` 或 `cli.rs` 中内联算法逻辑
- GUI 视图必须实现 `ToolView` trait，并正确处理 `on_data_received` 回调以支持智能粘贴
- 新增或重命名工具必须实现 `Tool` trait，并在 `crates/tools/src/catalog.rs` 的 `default_catalog()` 中完成单点注册
</important>

<important if="you are implementing or modifying Smart Detection detectors">
- 探测器必须实现 `devtoys_api::Detector` trait，通过 `DataTypeSpec` 声明自身数据类型及父级关系
- 探测器必须是纯计算逻辑，严禁调用窗口 API、系统剪贴板或执行阻塞 I/O
- 探测测试参照 `crates/core/tests/detection.rs`
</important>

<important if="you are developing GUI views, widgets, or theme styling">
- 遵循 egui 立即模式渲染规范，通用组件与辅助函数参见 `crates/tools/src/ui.rs`
- 颜色统一取自 `Palette`（见 `crates/host/src/theme.rs`）或 `ui::danger` / `ui::success`，支持浅色与深色自适应
- 文本转换类工具优先使用 `ui::split_2` 与 `ui::labeled_code` 进行双栏排版，剪贴板导出调用 `ui::copy_text`
</important>

<important if="you are writing or running tests">
- 工具核心转换与算法单测写在对应工具的 `helper.rs` 内置测试模块中
- 工具注册与契约集成测试位于 `crates/tools/tests/catalog.rs`
- 设置存储与序列化测试位于 `crates/core/tests/settings_store.rs`
- 状态机测试位于 `crates/core/tests/app_state.rs`
</important>

<important if="you are reading, creating, or tracking issues and tickets">
- 本地工单遵循 `docs/agents/issue-tracker.md` 规范，存放于 `.scratch/<feature-slug>/`
- 特性规格为 `spec.md`，实现工单为 `issues/<NN>-<slug>.md`
- 工单状态流转使用 `docs/agents/triage-labels.md` 中定义的 5 个标准分类标签
</important>

<important if="you are designing architecture, defining seams, or modifying domain models">
- 遵循单一上下文（single-context）规则：通用领域词汇记录于根目录 `CONTEXT.md`，架构决策记录于 `docs/adr/`
- 严格遵循 `skill://codebase-design` 架构词汇（Module, Interface, Implementation, Depth, Seam, Adapter, Leverage, Locality）
</important>


## Agent skills

### Issue tracker

Issues live as markdown under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Default role strings: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

single-context: root `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.
