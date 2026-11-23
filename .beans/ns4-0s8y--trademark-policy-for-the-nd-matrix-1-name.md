---
# ns4-0s8y
title: 'Trademark policy for the ND Matrix 1 name'
status: in-progress
type: task
priority: normal
created_at: 2026-11-23T16:00:00Z
updated_at: 2026-11-23T16:00:00Z
parent: ns4-9wq1
---

## Context
The license covers code, not names. A fork could ship under "ND Matrix 1". Nord and Clavia names are used only descriptively (D16, docs/specs/visual-style-and-panel.md section 10).

## Higher Goal
Forks must rename; the project name stays tied to this project.

## Acceptance Criteria
- [x] `TRADEMARKS.md`: forks and modified builds must use a different name and icon; descriptive use ("based on ND Matrix 1") allowed
- [x] Linked from README, CONTRIBUTING and docs/LEGAL.md section 5
- [ ] Owner decides whether to register the name (jurisdiction, cost)

## Out of Scope
Registration itself.

## Summary of Changes
Drafted `TRADEMARKS.md` (forks rename, descriptive use fine, vendor names descriptive only), shipped in every release layout, pinned by `license_check.py` ("different name"), linked from README, CONTRIBUTING and LEGAL.md s5; D241. Still open: owner decision on registration.
