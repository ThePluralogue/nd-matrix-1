---
# ns4-b9rn
title: 'Rename nord-plugin-standalone and remove vendor prefix from shipped binaries'
status: todo
type: task
priority: high
created_at: 2026-11-23T21:25:00Z
updated_at: 2026-11-23T21:25:00Z
parent: ns4-9wq1
---

## Context
PR #619 renamed the DAW plugin bundles to `nd-matrix-1.clap` and `nd-matrix-1.vst3`, but `tools/release_package.py` (`BINS = ("ns4", "nd-matrix-1", "nord-plugin-standalone")`) and `README.md` still ship a user-facing executable named `nord-plugin-standalone[.exe]`.

## Higher Goal
No shipped user-facing binary, installer entry, or archive carries the vendor's `nord-` trademark prefix.

## Acceptance Criteria
- [ ] Rename the standalone plugin binary to `nd-matrix-1-standalone` (or drop it from release archives if redundant with `nd-matrix-1`)
- [ ] Update `tools/release_package.py`, `tools/release_installers.py`, `tools/release_verify.py`, `README.md`, and tests
- [ ] Add a `release_verify.py` check that fails if any shipped binary or bundle starts with `nord-`

## Out of Scope
Internal Rust crate directory names (`crates/nord-*`, `publish = false`).
