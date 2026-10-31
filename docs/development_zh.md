# 开发说明

**语言 / Language:** 中文 | [English](development.md) · [返回 README](../README.md)

## 环境与启动

需要 Windows 11 x64、Node.js 24、根目录 `package.json` 声明的 pnpm 版本、Rust stable/MSVC、Visual Studio C++ 构建工具、Python 3.12+、uv 和 Microsoft Edge WebView2。发行脚本会准备 Python 构建依赖并打包独立引擎，最终用户无需安装 Python。

```powershell
pnpm install --frozen-lockfile
uv sync --project engine --extra windows --extra ocr --extra dev --extra build
pnpm tauri dev
```

开发模式会启动真实桌面程序；开启任务会向游戏发送真实输入，没有默认 dry-run。若自动路径发现不适用，可分别用 `IDENTITYV_PYTHON`、`IDENTITYV_ENGINE_PATH` 指定 Python 可执行文件和引擎 `main.py`。

单独启动协议引擎：`uv run --project engine python engine/main.py`。标准输入与输出采用 [NDJSON 协议](ipc.md)，每行一个 JSON，例如：

```json
{"version":1,"type":"ping","timestamp":"2026-01-01T00:00:00Z","request_id":"manual"}
```

## 验证

```powershell
pnpm typecheck
pnpm build
node --test frontend/tests/logs.test.mjs
uv run --project engine pytest engine/tests
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
```

前端浏览器测试运行真实 React 界面，但模拟 Tauri IPC。使用 `frontend/tests/launcher.test.mjs` 时先在 1420 端口启动 Vite；若 `agent-browser` 不在 PATH 中，可通过 `AGENT_BROWSER_BIN` 指定。它们不能证明原生文件选择、安装、账号登录或实际对局成功。截图样本、模拟输入、离线测试和构建结果需要与真实运行验证分开报告。

## 分层与生命周期

- React 编辑配置、显示事件；游戏动作判断留在 Python。
- Rust 管理进程、配置、地图导入、更新校验和应用退出。
- Python 管理捕捉、识别、任务路由、计时和受前台检查约束的输入。

`start_task` / `start_tasks` 会自动启动尚未运行的引擎；F3 切换引擎，并与其他生命周期操作串行。显式启动自动化时应用游戏窗口布局，单次诊断不应用。暂停释放输入并冻结任务计时，恢复重新截图，停止清空队列。前台或捕捉丢失有有限恢复时间，详见 [功能逻辑](README_LOGIC.md) 与 [IPC](ipc.md)。

本地规格和问题放在 `.scratch/`，遵循 [问题工作流](agents/issue-tracker.md)。真正的架构决策记入 `docs/adr/`，共享术语记入 `GLOSSARY.md`，不要为局部问题扩展成全项目重写。

## 打包与发布

`pnpm tauri build` 生成签名安装包，但不会自动发布。签名密钥、干净源码校验、附件命名和发布步骤见 [更新与发布](updates.md)。
