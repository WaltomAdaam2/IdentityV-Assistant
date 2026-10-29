# Domain documentation

This application uses a single domain context across React, Rust, and Python.
Before exploring an area, read root `GLOSSARY.md` and the applicable `docs/adr/` entries when present.
Use established domain names in code, tests, issues, and explanations.
Create or update glossary terms only when actual concepts have been established.
Record durable, consequential architecture decisions in numbered ADRs with context, decision,
alternatives, and consequences. Link superseding decisions instead of silently rewriting history.
Surface conflicts with existing ADRs before changing the affected design.
Keep active implementation and verification progress in `.scratch/`; do not duplicate it in ADRs.
