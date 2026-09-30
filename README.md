# Identity V Assistant

**语言 / Language:** 中文 | [English](README_EN.md)

Identity V Assistant 是 Windows 桌面端的《第五人格》外部视觉自动化助手。它通过窗口截图识别画面，并用普通 Windows 输入执行任务。

> 当前为 **0.1.0 开发版本**。“黑杰克”流程已实现并通过提供的截图与定时逻辑检查，尚未在实际游戏中完成整局验证。

## 功能

- React + TypeScript 中英文桌面界面，默认中文
- Tauri 2 Rust 进程监督与命令桥接
- Python 常驻引擎与 NDJSON IPC
- `ready`、心跳、日志、状态、动作、指标和结构化错误事件
- 可暂停、恢复、停止的“登录进大厅”和“黑杰克”任务
- Pydantic 配置校验、加载与持久化
- Windows 窗口发现、客户区、响应状态和 DPI 实时诊断
- Windows 窗口级捕捉，支持被其他窗口遮挡或放在副屏的游戏窗口
- DXcam 与 MSS 兼容回退
- OpenCV 模板匹配与可配置置信度
- 可选 RapidOCR 后端的惰性 OCR 接口
- 状态证据、历史确认、未知状态和恢复框架
- 登录任务使用窗口消息输入；黑杰克任务恢复并置前游戏窗口，使用 SendInput 前台输入
- 十个固定像素识别归宿界面，八点匹配后点击书本
- “登录进大厅”任务可从任意已知登录画面继续，双帧确认后选择平台、关闭弹窗并等待归宿
- “黑杰克”从归宿打开书本、选择模式、匹配与准备，识别角色后执行对局动作，结果页返回归宿；可选择无限循环
- 任务卡片的“编辑”可调整登录平台、超时、点击延迟与随机范围、视角转动幅度及循环方式，更改自动保存
- 同时启用两个任务时按列表顺序运行；启用状态在重启后恢复
- 可选内存保护：系统内存超过 90% 时清理未使用对象
- 面向普通用户开放的捕捉诊断、精简日志和窗口设置；日志支持搜索、筛选、分页和跳页
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
window capture + Windows input
```

自动捕捉默认通过 Windows Graphics Capture 按游戏窗口句柄读取完整客户区。登录任务可使用窗口消息输入；黑杰克任务会把游戏置于前台，并在输入前重新检查窗口。识别素材和提供的参考截图集中在 [`assets/templates/blackjack`](assets/templates/blackjack)。

## 使用任务

1. 启动第五人格与本应用，确保游戏窗口已打开。
2. 在“任务”中点击“编辑”，为登录任务选择 Android 或 iOS，并按需调整黑杰克参数。
3. 勾选要运行的任务，点击“启动已启用任务”。同时勾选时先登录进大厅，再运行黑杰克。
4. 黑杰克运行期间游戏保持前台。暂停或停止会释放已按下的按键。

页面左侧显示当前阶段、角色与完成循环数。默认完成一局并返回归宿后结束；“无限循环”默认关闭。加载动画不会计入普通阶段等待，连续加载超过 300 秒会超时。

## 本地数据

配置和运行数据保存在 `%LOCALAPPDATA%\IdentityV-Assistant\`：

```text
config.json                  设置与启用任务
logs\YYYY-MM-DD.jsonl       按日期保存的完整事件
debug\captures\            手动捕捉的调试截图
```

日志文件只保留最近 30 天，启动和跨日写入时自动清理过期文件。日志页面只显示本次应用会话，最多保留 99,000 条、每页 1,000 条；清空页面不删除日期日志。详细调试开关只改变页面显示，磁盘日志仍保存完整事件。

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

- 在实际游戏中校准各阶段识别与输入参数
- 将 Python 引擎打包为独立可执行文件
- 增加 OCR 实现与真实游戏环境集成测试

项目不会加入进程内挂钩、内存读取、反作弊绕过、内核输入或网络协议操作。

## 许可证

本项目采用 [GNU General Public License v3.0 only](LICENSE)（`GPL-3.0-only`）。分发修改版本时必须依照该许可证提供对应源代码并保留许可证与版权声明。
