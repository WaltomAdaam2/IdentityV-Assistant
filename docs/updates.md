# 更新与发布

## 用户更新

从 [GitHub Releases](https://github.com/WaltomAdaam2/IdentityV-Assistant/releases) 下载 Windows x64 安装包。v0.1.0 没有内置更新，需要手动安装 v0.1.1 或更新版本一次。

此后在「设置 → 软件更新」选择 GitHub 直连或国内代理（第三方 GH-Proxy）。启动时默认只检查更新，不自动下载或安装；可关闭自动检查。代理的速度与可用性取决于第三方服务，失败时可以切换线路重试。

下载必须同时通过发布签名、SHA-256 和文件大小校验。未通过时禁止安装。下载可取消，重试从头开始。点击安装会停止自动化服务、释放输入并退出助手，随后启动安装器。配置、日志和导入地图仍保存在 AppData。

## 维护者打包

正常开发使用 `pnpm tauri dev`。发行构建使用仓库根目录的 `打包.bat`，或：

```powershell
pnpm install --frozen-lockfile
pnpm tauri build
```

签名私钥默认位于 `%LOCALAPPDATA%\IdentityV-Assistant\release-keys\updater.key`。也可通过环境变量 `TAURI_SIGNING_PRIVATE_KEY` 指定安全存储中的原始私钥文件；加密私钥另设 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。脚本只临时加载路径，不打印私钥、不生成替代密钥。**备份私钥并限制访问，不上传到仓库或 Release；丢失或更换私钥会使旧客户端无法验证后续更新。** 仓库只保存对应公钥。

其他贡献者没有发行私钥时可以运行开发构建和测试；独立分叉项目应建立自己的签名密钥及公钥配置，不能冒用本项目更新链。

成功构建会在 `src-tauri/target/release/bundle/nsis/` 生成以下发布文件：

- `IdentityV-Assistant_VERSION_x64-setup.exe`：完整安装包。
- 同名 `.exe.sig`：更新签名。
- `latest.json`：版本说明、平台 URL、签名、SHA-256、文件大小及构建提交。
- `SHA256SUMS.txt`：手动校验清单。

这些文件来自同一次构建，不能混用不同版本。签名必须包含与清单一致的版本号（Tauri CLI 2.12 或更新版本生成），客户端要求验证签名内版本，防止代理篡改版本字段。打包不会自动发布。

## 发布新版

1. 更新各处版本号，完成检查，提交并推送 `main`。
2. 从干净的 `main` 运行 `打包.bat`。未提交改动下构建的产物可本地测试，但发布脚本会拒绝。
3. 将面向用户的版本说明写入一个 UTF-8 文件，例如 `%TEMP%\release-notes.md`。
4. 登录 GitHub CLI 后显式发布：

```powershell
gh auth login
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/publish_release.ps1 -NotesFile "$env:TEMP\release-notes.md"
```

发布脚本确认工作区干净、分支为 `main`、远端与构建提交一致、版本与校验信息一致。它先创建草稿并上传全部附件，再发布为最新版本，避免客户端读到上传一半的更新。

如果同名标签或 Release 已存在（包括上次中断留下的草稿），脚本会停止，不覆盖、不删除。网络结果不明确时，先检查 GitHub 上的草稿和附件，人工确认完整后再发布；不要盲目重复创建或替换已有版本。

发布签名用于内置更新来源校验，与 Windows Authenticode 代码签名不同；Windows 仍可能显示未知发布者提示。
