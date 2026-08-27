# DevToys-RS

基于 [GPUI](https://github.com/zed-industries/zed) 构建的跨平台开发者工具箱，灵感来自 [DevToys](https://github.com/veler/DevToys)，使用 Rust 实现。

## ✨ 特性

- 🖥️ **原生跨平台 GUI** — 基于 GPUI 高性能渲染（Windows / macOS / Linux）
- ⌨️ **CLI 支持** — 所有工具能力均可通过 `devtoys-cli` 命令行调用
- 🔍 **智能识别** — 粘贴文本自动检测数据类型并推荐工具
- 🧩 **插件化架构** — 工具以模块化方式注册，易于扩展
- 🌙 **主题支持** — 浅色 / 深色主题跟随系统或手动切换

## 🧰 内置工具

| 分类 | 工具 |
|------|------|
| 编解码 | Base64 文本/图片、URL、HTML 实体、转义字符、GZip |
| 加密校验 | MD5 / SHA1 / SHA256 哈希、HMAC、密码生成 |
| 格式化 | JSON / XML / SQL 格式化 |
| 转换 | JSON ⇄ YAML、日期时间、进制转换 |
| 解析器 | JWT、Cron 表达式、JSONPath、XSD 校验 |
| 图片 | 二维码生成/识别、图片格式转换、色觉模拟 |
| 文本 | 文本对比、列表对比、文本统计、Lorem Ipsum、Markdown 预览 |
| 其他 | X.509 证书解析、UUID 生成 |

## 📦 构建

```bash
# 克隆仓库
git clone https://github.com/JaminYe/devtoys-rs.git
cd devtoys-rs

# 构建 GUI 版本
cargo build --release -p devtoys

# 构建 CLI 版本
cargo build --release -p devtoys-cli
```

> Linux 需要安装 GPUI 相关依赖：libxkbcommon、libwayland、libfontconfig 等，详见 [.github/workflows/build.yml](.github/workflows/build.yml)

## 🚀 下载

前往 [Releases](https://github.com/JaminYe/devtoys-rs/releases) 页面下载对应平台的打包产物：

| 平台 | 架构 | 产物 |
|------|------|------|
| Windows | x86_64 | `devtoys-x86_64-pc-windows-msvc.zip` |
| macOS | Apple Silicon | `devtoys-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `devtoys-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 | `devtoys-x86_64-unknown-linux-gnu.tar.gz` |

## 🏗️ 项目结构

```
crates/
├── api/      # 公共类型：工具元数据、分组、设置、Detector trait
├── core/     # 核心逻辑：注册表、检测器调度、设置存储
├── tools/    # 全部业务工具实现
├── host/     # GPUI 桌面宿主：窗口、侧栏、设置界面
└── cli/      # 命令行入口
```

## 📄 License

MIT
