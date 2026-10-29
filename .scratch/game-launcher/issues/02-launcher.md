# Implement version 0.1.6 launcher

Status: claimed
Type: task

## Scope

Implement the full requirements in ../spec.md. Keep game/proxy state distinct from automation task state.

## Progress

- Test seams approved by user.
- Read upstream discovery, standalone parameters, process-proxy ordering, public URI interface, and launcher resource metadata.
- Need to settle runtime integration and validate announcement/background sources before implementation.
- Updated seven first-party manifest/lock versions to 0.1.6; sidebar, tooltip and About now read frontend package metadata. README version notices updated.
- Existing TypeScript compiler checked app and Vite configs successfully; parsed all seven version declarations and confirmed 0.1.6.
- pnpm wrapper attempted dependency reinstallation and stopped at a non-interactive purge prompt; used installed compiler directly without deleting/reinstalling dependencies.
- Project context milestone committed as 98d4696. Two ordinary pushes failed (GitHub connection timeout, then connection reset); push remains pending.
- Awaiting user choice on launcher architecture and independent upstream login component integration.

## Verification

Version metadata and TypeScript checks passed. Launcher implementation and its tests remain pending.
