# Identity V Assistant

**语言 / Language:** 中文 | [English](README_EN.md)

Identity V Assistant 是 Windows 桌面端的《第五人格》外部视觉自动化助手。它通过窗口截图识别画面，并用普通 Windows 输入执行任务。

> 当前为 **0.1.0 开发版本**。黑杰克、自动监管者匹配、自动监管者排位和捉迷藏已通过提供的截图与逻辑检查；按约定未进行实际游戏对局验证。

## 功能

- React + TypeScript 中英文桌面界面，默认中文
- Tauri 2 Rust 进程监督与命令桥接
- Python 常驻引擎与 NDJSON IPC
- `ready`、心跳、日志、状态、动作、指标和结构化错误事件
- 可暂停、恢复、停止的“登录进大厅”“黑杰克”“自动监管者匹配”“自动监管者排位”和“捉迷藏”任务
- Pydantic 配置校验、加载与持久化
- Windows 窗口发现、客户区、响应状态和 DPI 实时诊断
- Windows 窗口级捕捉，支持被其他窗口遮挡或放在副屏的游戏窗口
- DXcam 与 MSS 兼容回退
- OpenCV 模板匹配与可配置置信度
- RapidOCR 限定区域识别对局计时
- 状态证据、历史确认、未知状态和恢复框架
- 登录任务使用窗口消息输入；对局任务恢复并置前游戏窗口，使用 SendInput 前台输入
- 十个固定像素识别归宿界面，八点匹配后点击书本
- “登录进大厅”任务可从任意已知登录画面继续，双帧确认后选择平台、关闭弹窗并等待归宿
- 黑杰克可从归宿、匹配、准备、对局或结算等已知阶段接管，识别角色后执行对应动作，结果页返回归宿
- 自动监管者匹配走“日记推理”；自动监管者排位走“排位模式 → 传统排位”。两者共用监管者对局逻辑，达到游戏显示时间六分钟后尝试投降
- 捉迷藏可从归宿、混沌模式、模式选择、匹配、准备、任意已知对局、淘汰或结算阶段接管；会选择“坏孩子”，并在固定状态下并行执行移动与视角循环
- 四种对局任务可分别设置无限循环；已识别的匹配、准备、淘汰和结算等待没有时限
- 窗口暂时未响应、丢失或截图失败时持续尝试恢复，连续 300 秒无法取得可用画面才暂停任务
- 可分别禁止系统休眠和自动熄屏；自动熄屏开关也维持会话活动，减少闲置自动锁屏
- 任务卡片的“编辑”可调整登录平台、登录超时、点击延迟与随机范围、视角转动幅度及循环方式；登录设置位于“登录进大厅”任务编辑器，更改自动保存
- 同时启用多个任务时按列表顺序运行；启动任务会自动启动尚未运行的自动化服务，启用状态在重启后恢复
- 支持 F3 全局暂停/恢复；暂停会释放已按下的输入并冻结任务计时
- 不可恢复的任务错误会暂停当前任务并保留队列，恢复时重新识别当前画面后继续；每个活动任务行都有“停止”，停止会取消当前任务并清空后续队列
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

自动捕捉默认通过 Windows Graphics Capture 按游戏窗口句柄读取完整客户区。登录任务可使用窗口消息输入；对局任务会把游戏置于前台，并在输入前重新检查窗口。识别素材和参考截图分别位于 [`assets/templates/blackjack`](assets/templates/blackjack)、[`assets/templates/hunter_match`](assets/templates/hunter_match)、[`assets/templates/hunter_rank`](assets/templates/hunter_rank) 和 [`assets/templates/hide_seek`](assets/templates/hide_seek)。普通匹配与排位共用已有的监管者对局图标素材。

## 使用任务

1. 启动第五人格与本应用，确保游戏窗口已打开。
2. 在“任务”中点击对应卡片的“编辑”。“登录进大厅”编辑器包含平台、最长等待时间和点击参数；对局任务编辑器包含无限循环及共享的输入参数。
3. 勾选要运行的任务，点击“启动已启用任务”。这会在服务尚未运行时自动启动服务；多个任务按页面列表顺序执行。
4. 按 F3 可暂停或恢复活动任务。暂停会释放已按下的输入并冻结任务计时；恢复会重新识别当前画面后继续。
5. 点击活动任务所在行的“停止”会停止该任务并清空其后队列。不可恢复的错误会保留当前任务和后续队列为暂停状态，待手动恢复。

页面左侧显示当前阶段、角色与完成循环数。默认完成一局并返回归宿后结束；“无限循环”默认关闭。任务启动和暂停恢复后会识别当前画面，从已知阶段继续。已识别的匹配、准备、淘汰和结算界面会持续等待；未知界面、持续加载或无法取得可用窗口画面仍以 300 秒为上限。捉迷藏的完整阶段与动作见 [状态机说明](docs/state-machine.md)。

“禁止自动熄屏和闲置锁屏”在应用开启期间维持屏幕亮起，并定期发送 F15 键以刷新 Windows 闲置计时；关闭开关或退出应用即停止。手动按 Win+L 仍会锁屏，设备管理策略也可能强制锁屏。Windows 的[电源请求](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-powersetrequest)本身只阻止屏幕因闲置熄灭；[自动锁屏策略](https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-10/security/threat-protection/security-policy-settings/interactive-logon-machine-inactivity-limit)按用户输入闲置时间执行。

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
uv sync --extra windows --extra ocr --extra dev
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
- 在真实游戏环境中验证对局与结算识别

项目不会加入进程内挂钩、内存读取、反作弊绕过、内核输入或网络协议操作。

## 许可证

本项目的组合分发采用 [GNU Affero General Public License v3.0 or later](LICENSE)（`AGPL-3.0-or-later`）。`engine/window/windows.py` 中与游戏窗口框架、置顶、尺寸和恢复相关的部分改编自 AhabAssistantLimbusCompany；来源、修订和版权说明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。分发修改版本时必须依照该许可证提供对应源代码并保留许可证与版权声明。
