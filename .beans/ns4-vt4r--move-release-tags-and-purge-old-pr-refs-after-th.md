---
# ns4-vt4r
title: 'Move release tags and purge old PR refs after the history rewrite'
status: in-progress
type: task
priority: high
created_at: 2026-11-23T16:00:00Z
updated_at: 2026-11-23T21:25:00Z
parent: ns4-9wq1
---

## Context
The 2026-11-23 history rewrite (docs/LEGAL.md section 8 question 5) rewrote `main` and every branch. `v0.1.0`, `v0.1.1`, and `v0.1.2` have been moved on `origin` to the rewritten commits (`e36542a0d3d9`, `ee9b6a9d2ba1`, `ebf0cd784754`). About 620 read-only `refs/pull/*` still keep old commits reachable on GitHub until GitHub Support purges them.

## Higher Goal
No purged vendor file is reachable from the repository on GitHub.

## Acceptance Criteria
- [x] Owner force-pushes the three tags to the rewritten commits (`v0.1.0` `e36542a0d3d9`, `v0.1.1` `ee9b6a9d2ba1`, `v0.1.2` `ebf0cd784754` verified via `git ls-remote --tags origin`)
- [ ] GitHub Releases for the three tags still show the right assets
- [ ] GitHub Support request filed to purge cached views and old pull request refs (sensitive data removal)
- [ ] Every local clone re-synced (`git fetch && git reset --hard origin/main`) or re-cloned
- [ ] Note the completion date in docs/LEGAL.md section 8 question 5

## Out of Scope
Forks and third-party clones (cannot be cleaned).
