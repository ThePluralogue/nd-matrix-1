---
# ns4-9ngp
title: 'Decide DCO or CLA for contributions'
status: todo
type: task
priority: normal
created_at: 2026-11-23T16:00:00Z
updated_at: 2026-11-23T16:00:00Z
parent: ns4-9wq1
---

## Context
D232 requires DCO sign-off. DCO licenses contributions only under PolyForm Noncommercial, so the owner cannot later relicense or sell commercial licenses for other people's code without each contributor's consent. A CLA grants that right (and an explicit patent license) at the cost of a signature step.

## Higher Goal
The contribution terms match the owner's long-term plans for the project.

## Acceptance Criteria
- [ ] Owner decides: keep DCO, or switch to a CLA while outside contributors are few
- [ ] If CLA: CLA text reviewed by counsel, CLA check on PRs, CONTRIBUTING and D232 updated (superseded by a new decision)
- [ ] Decision recorded in docs/DECISIONS.md

## Out of Scope
Relicensing existing contributions.
