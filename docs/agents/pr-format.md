# PR 格式记忆

以后所有相关项目的 PR 标题和正文默认使用以下格式。所有聊天和 agent 在编写或更新 PR 时，应先读取并遵循本文件。

## PR 标题

格式：`<type>: <English one-line summary>`

- 标题只用英文。
- 标题不加 scope。
- 常用 type：`feat`、`fix`、`chore`、`docs`、`refactor`、`test`、`build`、`ci`、`perf`。

示例：

- `feat: add backup retention operations`
- `fix: fix missed container scan matches`
- `chore: bump version to v1.7.0`

## PR 正文

英文在前，中文在后，中英分段，不交叉。

- `Updates & Fixes` / `更新与修复`：每段 4 至 14 行。
- `Scope & Impact` / `影响范围`：每段 2 至 7 行。
- `Verification` / `验证`：一般固定 3 行；特殊情况例如存在未覆盖验证风险时最多 5 行。
- 验证段前两行固定保留命令/验证方式与 CI、构建、测试、运行验证或 JAR 校验证据，不随意增减行数。

```md
## Updates & Fixes

- <Explain what this PR adds, improves, or fixes. Keep it specific and do not claim unverified behavior.>
- <Second change.>
- <Third change.>
- <Fourth change.>

## Scope & Impact

- <Affected modules, commands, config, data, compatible versions, or user-visible behavior.>
- <If there is no destructive database, schema, config, or compatibility change, state “none”.>

## Verification

- Ran `<command or verification method>`, result: <passed/failed/not run reason>.
- <CI, build, unit test, integration test, runtime check, JAR checksum, or other evidence.>
- <Remaining verification risk; if none, write “no known remaining risk”.>

---

## 更新与修复

- <说明本 PR 新增、优化或修复了什么，保持具体，不写未验证内容。>
- <第二项变更。>
- <第三项变更。>
- <第四项变更。>

## 影响范围

- <涉及的模块、命令、配置、数据、兼容版本或用户可见行为。>
- <如无数据库、schema、配置或兼容性破坏性变化，明确写“无”。>

## 验证

- 已运行 `<命令或验证方式>`，结果：<通过/失败/未运行原因>。
- <CI、构建、单元测试、集成测试、运行验证、JAR 校验等证据。>
- <仍未覆盖的验证风险；没有则写“无已知剩余风险”。>
```
