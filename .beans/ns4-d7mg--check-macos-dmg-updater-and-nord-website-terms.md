---
# ns4-d7mg
title: 'Check macOS .dmg updater and Nord website terms (free check)'
status: todo
type: task
priority: normal
created_at: 2026-11-23T21:25:00Z
updated_at: 2026-11-23T21:25:00Z
parent: ns4-9wq1
---

## Context
`docs/LEGAL.md` section 2 checked the Windows v1.66 `.exe` updater with `strings` and found no EULA or reverse-engineering restriction, but left the macOS `.dmg` updater and `nordkeyboards.com` download page / terms of use unchecked.

## Higher Goal
Close the remaining factual gaps in `docs/LEGAL.md` section 2 without paid counsel.

## Acceptance Criteria
- [ ] Run a text/resource inspection on a local, git-ignored `Update Nord Stage 4 OS v1.66.dmg` (DMG license resource + strings inside the `.app` bundle)
- [ ] Inspect `nordkeyboards.com` Downloads page and site footer/terms for any license or reverse-engineering terms
- [ ] Replace the remaining direct `.zip` deep-link constants in `crates/nord-core/src/firmware_setting.rs` and `crates/nord-gui/src/debug_dialog.rs` with the Downloads page URL (matching `docs/guides/nord-os-update.md`)
- [ ] Record factual findings and date in `docs/LEGAL.md` section 2 and section 8 question 1

## Out of Scope
Legal conclusions or paid counsel review (`ns4-ns2j`).
