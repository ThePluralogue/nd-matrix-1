---
# ns4-vt4r
title: 'Move release tags and purge old PR refs after the history rewrite'
status: todo
type: task
priority: normal
created_at: 2026-11-23T16:00:00Z
updated_at: 2026-11-23T16:00:00Z
parent: ns4-9wq1
---

## Context
The 2026-11-23 history rewrite (docs/LEGAL.md section 8 question 5) rewrote `main` and every branch, but this session could not push tags. `v0.1.0`, `v0.1.1`, `v0.1.2` still point at old commits that contain the purged files, and about 620 read-only `refs/pull/*` keep old commits reachable on GitHub.

## Higher Goal
No purged vendor file is reachable from the repository on GitHub.

## Acceptance Criteria
- [ ] Owner force-pushes the three tags to the rewritten commits (`git push --force origin <new-sha>:refs/tags/<tag>`; SHAs v0.1.0 `e36542a0d3d9`, v0.1.1 `ee9b6a9d2ba1`, v0.1.2 `ebf0cd784754`)
- [ ] GitHub Releases for the three tags still show the right assets
- [ ] GitHub Support request filed to purge cached views and old pull request refs (sensitive data removal)
- [ ] Every local clone re-synced (`git fetch && git reset --hard origin/main`) or re-cloned
- [ ] Note the completion date in docs/LEGAL.md section 8 question 5

## Out of Scope
Forks and third-party clones (cannot be cleaned).
