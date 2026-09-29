# Identity V Assistant

**语言 / Language:** 中文 | [English](README_EN.md)

Identity V Assistant 是 Windows 桌面端的《第五人格》外部视觉自动化助手框架。它通过可见画面识别、状态机和普通 Windows 输入组织自动化流程，不读取或修改游戏进程，不注入 DLL，也不处理网络协议。

> 当前处于 **0.1.0 框架阶段**。完整对局、每周刷取、角色逻辑和路径规划均未实现。

## 功能

- React + TypeScript 中文桌面工具界面
- Tauri 2 Rust 进程监督与命令桥接
- Python 常驻引擎与 NDJSON IPC
- `ready`、心跳、日志、状态、动作、指标和结构化错误事件
- 可暂停、恢复、停止的 MockTask
- Pydantic 配置校验、加载与持久化
- Windows 窗口发现、客户区和 DPI 诊断框架
- Windows 窗口级捕捉，支持被其他窗口遮挡或放在副屏的游戏窗口
- DXcam 与 MSS 兼容回退
- OpenCV 模板匹配与可配置置信度
- 可选 RapidOCR 后端的惰性 OCR 接口
- 状态证据、历史确认、未知状态和恢复框架
- 全局 dry run、后台窗口坐标输入和输入释放
- Debug、日志过滤、设置和开发者输入测试界面
- Python 与 Rust 核心测试基础

## 架构

```text
React / TypeScript GUI
        |
        | Tauri commands + events
        v
Rust lifecycle supervisor
        |
        | NDJSON over stdin/stdout
        v
Python visual automation engine
        |
        v
window surface + client-area Windows input
```

自动捕捉会优先按游戏窗口句柄读取完整客户区，不依赖窗口是否处于最前方，也不依赖主屏位置。后台点击使用游戏客户区坐标发送给目标窗口；开发者输入测试默认仍受模拟模式保护。

职责边界见 [docs/architecture.md](docs/architecture.md)，协议见 [docs/ipc.md](docs/ipc.md)。

## 环境要求

- Windows 11 x86_64
- Node.js LTS
- pnpm 11+
- Rust stable（MSVC）
- Python 3.12+
- uv（推荐）
- WebView2 Runtime

## 安装依赖

```powershell
pnpm install
Set-Location engine
uv sync --extra windows --extra dev
Set-Location ..
```

## 运行方法

```powershell
pnpm tauri dev
```

只运行前端：

```powershell
pnpm dev
```

只运行 Python 引擎：

```powershell
Set-Location engine
uv run python main.py
```

## 验证方法

```powershell
pnpm typecheck
pnpm build

Set-Location engine
uv run pytest

Set-Location ..\src-tauri
cargo fmt --check
cargo test
```

## 项目结构

```text
frontend/       React GUI
src-tauri/      Tauri shell and Python process supervisor
engine/         Python automation engine
assets/         Template and icon registry content
config/         Default configuration
docs/           Architecture and development documentation
scripts/        Windows development helpers
engine/tests/fixtures/ Generated test images
```

## 后续计划

- 收集合法的实际游戏截图和模板
- 为各游戏状态实现并校准检测器
- 完成每周任务的真实语义动作和验证流程
- 将 Python 引擎打包为独立可执行文件
- 增加 OCR 实现与真实游戏环境集成测试

项目不会加入进程内挂钩、内存读取、反作弊绕过、内核输入或网络协议操作。

## 许可证

本项目采用 [GNU General Public License v3.0 only](LICENSE)（`GPL-3.0-only`）。分发修改版本时必须依照该许可证提供对应源代码并保留许可证与版权声明。
