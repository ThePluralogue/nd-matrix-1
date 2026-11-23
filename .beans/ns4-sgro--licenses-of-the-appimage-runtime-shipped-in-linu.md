---
# ns4-sgro
title: 'Licenses of the AppImage runtime shipped in Linux releases'
status: todo
type: task
priority: normal
created_at: 2026-11-23T16:00:00Z
updated_at: 2026-11-23T16:00:00Z
parent: ns4-9wq1
---

## Context
`appimagetool` embeds a runtime in every `.AppImage`. Its license and notices are not covered by `THIRD_PARTY_LICENSES.html` (D238; docs/LEGAL.md section 8 question 9).

## Higher Goal
Every shipped binary, including the AppImage runtime, carries its required notices.

## Acceptance Criteria
- [ ] Identify the exact runtime version used by `tools/release_installers.py` and its license(s) from the upstream source
- [ ] Ship its notice in the AppImage and the release archive, or switch to a runtime with clear terms
- [ ] `tools/release_verify.py` checks the notice; tests added
- [ ] LEGAL.md question 9 answered

## Out of Scope
Other Linux packaging formats.
