# 多多解密

基于 **Tauri 2 + Vue 3 + TypeScript** 的桌面加解密工具骨架。当前阶段只搭框架与扩展点，加解密算法和完整插件运行时后续迭代。

目标平台：Windows / macOS / Linux。

## 功能骨架

- 悬浮角标：可拖动；左键打开主界面；右键弹出菜单
- 系统托盘：左键打开主界面；右键菜单与角标一致
- 独立功能窗口：打开主界面只显示加解密；功能设置、意见反馈、插件管理、关于各自单独成窗（无侧栏切换）
- 剪贴板监听：识别文本变化后提示是否立即加密/解密（可在设置中关闭）
- 主题：跟随系统深浅色，也可强制浅色/深色
- 插件：扫描 `plugin.json` 清单并注册贡献点（编辑器 / 拖尾特效 / 算法预设）

## 开发启动

前置条件：

- Node.js 18+
- Rust（[rustup](https://www.rust-lang.org/learn/get-started)，仓库通过 `rust-toolchain.toml` 固定 `stable` channel；Windows 需 `stable-x86_64-pc-windows-msvc` 工具链）
- Windows：Visual Studio 2022（含“使用 C++ 的桌面开发”）。`npm run tauri` 经 `scripts/run-tauri.mjs` 调用 `scripts/tauri.cmd`，自动加载 MSVC 环境，避免 Git Bash 误用 GNU/`dlltool`
- 各平台系统依赖见 [Tauri prerequisites](https://tauri.app/start/prerequisites/)

国内网络下载 crates 较慢时，项目已在 `src-tauri/.cargo/config.toml` 配置 rsproxy 镜像。

```bash
npm install
npm run tauri dev
```

浏览器预览前端（无托盘/角标原生能力）：

```bash
npm run dev
# 加解密：http://localhost:1420
# 角标：  http://localhost:1420/?window=badge
# 关于：  http://localhost:1420/?window=about
# 设置：  http://localhost:1420/?window=settings
# 反馈：  http://localhost:1420/?window=feedback
# 插件：  http://localhost:1420/?window=plugins
```

打包：

```bash
# Windows（NSIS 安装包）
npm run tauri build
```

## macOS 打包

**说明**：DMG 只能在 macOS 上构建，无法在 Windows 上交叉编译。Gitee 无免费 macOS Runner，需 Apple Silicon Mac 本机或自建 Mac 构建机。

### 环境（在 Mac 上执行）

- Apple Silicon Mac（M 系列），macOS 12+
- Xcode Command Line Tools：`xcode-select --install`
- Node.js 18+
- Rust stable（`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`）

### 命令

```bash
git clone <gitee-repo>
cd jdd-crypto
npm install
npm run build:mac
# 或
npm run build && npx tauri build
```

### 产物

- `src-tauri/target/release/bundle/dmg/多多解密_<version>_aarch64.dmg`
- `src-tauri/target/release/bundle/macos/多多解密.app`

`build-mac.sh` 会将 dmg 复制/对齐为内网商店命名：`多多解密_{version}_aarch64.dmg`。

### 上传内网商店

与 Windows 同目录，文件名区分：

- Windows: `多多解密_{version}_x64-setup.exe`
- macOS: `多多解密_{version}_aarch64.dmg`

Gitee 流水线接入说明见 [`docs/gitee-mac-build.md`](docs/gitee-mac-build.md)。

## 目录约定

```text
src/                         Vue 前端
  windows/                   badge / main / feature 窗口根组件
  views/                     各独立窗口页面
  stores/                    Pinia（theme / clipboard / plugins / app）
  composables/               主题、剪贴板提示、窗口拖动
  plugins-runtime/           JS 插件加载器与沙箱扩展点
  services/crypto.ts         加解密 invoke 接口（Rust stub）
src-tauri/                   Rust 核心
  src/windows.rs             窗口显示、按需创建功能窗、角标尺寸
  src/tray.rs                托盘与统一菜单
  src/clipboard.rs           剪贴板轮询与候选事件
  src/plugin_host.rs         插件目录扫描
  src/commands.rs            前端可调用命令
  plugins/<id>/plugin.json   开发期示例插件
```

### 插件包

每个插件一个目录：

```text
plugins/<id>/
  plugin.json
  index.js
```

`plugin.json` 关键字段：`id`、`name`、`version`、`entry`、`permissions`、`contributes`。

搜索路径：

1. 应用资源目录 `plugins/`
2. 用户数据目录 `plugins/`
3. 开发期 `src-tauri/plugins/`

安全底线：只加载 JS/WASM + 宿主 API，不加载任意 `.dll/.so`。

## 后续迭代

1. 角标位置持久化、主窗口深链与提示 UI 打磨
2. Rust 加解密实现与密钥管理
3. iframe 沙箱真正执行插件；落地 monaco / 鼠标拖尾 / crypto-presets
