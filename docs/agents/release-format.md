# Release format

All agents and continued chats must read this file before preparing a release.
Create a **Draft Release only**. The user reviews and personally publishes it.

- Title: `Identity V Assistant v<version>`.
- Body: the template below, describing only real changes. Normally 3–15 bullets per change section; fewer are required when fewer factual changes exist. Never invent changes to fill a template.
- Notes are tracked at `docs/releases/<version>.md`. The publisher copies them into `latest.json` and the GitHub body.
- Append `## SHA-256` **after Full Changelog**, covering installer, signature, manifest and checksum file. The publishing script generates this section; do not include it in the input notes or manifest itself.
- The first version without an earlier tag links to actual source history. Otherwise use a version comparison. A draft's future tag comparison becomes available when the user publishes.

```markdown
# Identity V Assistant v<version>

<简洁概述。>

## 新增与优化

- <真实新增或优化。>

## 修复内容

- <真实修复。>

**Full Changelog**: https://github.com/WaltomAdaam2/IdentityV-Assistant/compare/v<previous>...v<version>

## SHA-256

<由发布脚本追加四个附件的 SHA-256 代码块。>
```

See [updates](../updates.md) and [AGENTS.md](../../AGENTS.md) for permanent SNAPSHOT branches, post-merge freeze, clean builds and manual publication.
