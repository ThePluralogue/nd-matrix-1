---
# ns4-eh57
title: 'Second history rewrite after the firmware migrations'
status: todo
type: task
priority: normal
created_at: 2026-11-23T16:00:00Z
updated_at: 2026-11-23T16:00:00Z
parent: ns4-9wq1
blocked_by:
    - ns4-u8o2
---

## Context
The 2026-11-23 rewrite purged files already deleted from `main`. Class B and C firmware-derived files still tracked (38 paths in `tools/fw-derived-baseline.txt`) stay in history until they are migrated to runtime loading (bean ns4-u8o2, D128).

## Higher Goal
Once nothing firmware-derived is tracked, history holds none of it either.

## Acceptance Criteria
- [ ] ns4-u8o2 completed (`tools/fw-derived-baseline.txt` has no class B or C lines)
- [ ] Dry run of `git filter-repo --invert-paths` on a mirror; `main` tree unchanged; report to the owner
- [ ] Owner approval; branches force-pushed with lease; owner moves tags; GitHub Support purge requested
- [ ] docs/LEGAL.md updated

## Out of Scope
Forks and third-party clones.
