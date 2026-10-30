# Identity V Assistant

**语言 / Language:** 中文 | [English](README_EN.md)

Windows 桌面端的《第五人格》助手，提供游戏启动、视觉自动化任务和加页手记地图浏览。界面支持中文与英文，配置和地图保存在本机。

<p align="center">
  <img src="docs/images/overview.png" alt="Identity V Assistant 主界面" width="850">
</p>

## 功能

- **游戏启动**：定位正式服 `dwrg.exe`，通过独立的 [idv-login](https://github.com/KKeygen/idv-login) 登录组件直接启动游戏；支持组件配置、下载、更新确认和代理状态展示。
- **视觉任务**：登录进大厅、黑杰克、监管者匹配、监管者排位和捉迷藏；支持按顺序运行、暂停、恢复、停止及对局循环。
- **加页手记**：多选 ZIP 导入地图包，按赛季、难度、门型、楼层及纯净／速刷图筛选，支持详情与窗口内预览。
- **窗口与输入**：按窗口捕捉画面，使用经过前台检查的 Windows 输入；显式开启引擎后应用窗口设置，关闭时尝试恢复原窗口。
- **日常使用**：全局引擎快捷键、关闭到托盘、捕捉诊断、可筛选日志，以及可选的睡眠和熄屏控制。
- **更新**：默认 GitHub 直连，支持三个手动代理；显示下载速度、预计剩余时间与 Markdown 更新日志；下载前确认，安装前校验签名、版本、SHA-256 与文件大小。

行为逻辑、任务恢复、组件职责和地图格式见 [功能逻辑说明](docs/README_LOGIC.md)。构建与检查见 [开发说明](docs/development_zh.md)，迁移与发布见 [更新说明](docs/updates.md)。

## 环境要求

- Windows 11 x64 与 Microsoft Edge WebView2 Runtime。
- 已安装《第五人格》正式服客户端；自动化输入时游戏需要处于前台。
- 游戏启动页的登录代理功能使用独立的官方 idv-login 组件；首次登录或凭证过期时仍需本人操作。

源码构建还需要 Node.js 24、仓库声明版本的 pnpm、Rust MSVC、Python 3.12、uv 及 Windows C++ 构建工具。

## 安装与开始使用

1. 在 [Releases](https://github.com/WaltomAdaam2/IdentityV-Assistant/releases) 下载 `IdentityV-Assistant-<版本>-Windows-x64-Setup.exe` 并安装。
2. 使用“游戏启动”定位游戏，在齿轮面板配置登录组件；也可自行启动游戏后使用视觉任务。
3. 在“任务”中编辑选项，勾选任务后点击“启动已启用任务”。多个任务按列表顺序运行。
4. 默认 **F3** 开启／关闭自动化引擎；暂停与恢复使用任务行按钮。停止任务会清空后续队列。

**任务会发送真实键鼠输入，没有默认 dry-run 模式。** 普通游戏启动不会套用引擎窗口布局。登录组件就绪不等于账号已登录；离线测试和截图识别通过也不代表真实对局成功。

## 地图与本地数据

地图包导入支持 ZIP 多选，单包失败不会阻止其他包；顶部提示显示成功数量和失败原因，并写入运行日志。地图筛选静默保存，楼层每次进入默认 1F。

![加页手记地图浏览](docs/images/journal.png)

数据目录为 `%LOCALAPPDATA%\IdentityV-Assistant\`，包括 `config.json`、按日期归档的 `logs\`、手动截图和 `journal\packs\`。升级保留此目录；完整字段和地图包格式见 [功能逻辑说明](docs/README_LOGIC.md)。

## 构建方法

```powershell
pnpm install --frozen-lockfile
uv sync --project engine --extra windows --extra ocr --extra dev --extra build
pnpm tauri build
```

正式签名构建需要原有更新签名私钥。第三方构建可自行配置签名，但不能替代官方更新链；具体要求见 [开发说明](docs/development_zh.md)。安装包输出至 `src-tauri/target/release/bundle/nsis/`。

## 更新与仓库迁移

从 0.1.7 起，源码、下载和后续更新统一使用本仓库。旧 `desktop-releases` 仓库提供一次 0.1.7 过渡更新；旧版是否能使用它取决于其更新器能力。旧仓库删除后，尚未迁移的客户端需从本仓库手动安装。

程序不会未经确认下载或安装更新。代理由第三方提供，失败时可切换来源；所有来源均保留完整性校验。详见 [更新与发布](docs/updates.md)。

## 项目结构

```text
frontend/   React 界面、配置编辑和状态展示
src-tauri/  Rust 桌面壳、进程生命周期、导入与更新
engine/     Python 识别、任务判断、捕捉和输入
assets/     识别素材与参考截图
scripts/    构建、打包、许可收集与发布
docs/       用户说明、开发文档与架构决策
```

## Credits

- [KKeygen/idv-login](https://github.com/KKeygen/idv-login)：独立登录组件；保留上游实现与许可。
- [Tauri](https://github.com/tauri-apps/tauri)、[React](https://github.com/facebook/react)：桌面壳与界面。
- [OpenCV](https://github.com/opencv/opencv)、[RapidOCR](https://github.com/RapidAI/RapidOCR)、[Windows Capture](https://github.com/NiiightmareXD/windows-capture)：图像处理、文字识别与窗口捕捉。
- [Lucide](https://github.com/lucide-icons/lucide) / [Iconify](https://github.com/iconify/icon-sets)：界面图标。

项目由 [WaltomAdaam2](https://github.com/WaltomAdaam2) 维护。第三方许可与随包源码见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)；游戏图像、商标及导入地图保留各自权利，来源见 [素材说明](frontend/public/launcher/SOURCES.md)。

## License

当前自有源码及 0.1.7 起的版本采用 **GNU GPL v3.0 only（GPL-3.0-only）**。Copyright (C) 2026 WaltomAdaam2。完整条款见 [LICENSE](LICENSE)。历史安装包保留其内含许可，第三方材料不因本项目许可变更而被重新授权。
