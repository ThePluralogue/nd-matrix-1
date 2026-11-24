---
# ns4-s3pn
title: 'Remove in-GUI sponsorship button to keep strictly non-commercial posture'
status: todo
type: task
priority: high
created_at: 2026-11-23T21:25:00Z
updated_at: 2026-11-23T21:25:00Z
parent: ns4-9wq1
---

## Context
PR #635 added a GitHub Sponsors button inside the GUI, a README badge, and `.github/FUNDING.yml`. Accepting donations inside an emulator of a currently-sold commercial instrument weakens the non-commercial research/interoperability posture and introduces ambiguity under PolyForm Noncommercial 1.0.0 when combined with DCO third-party contributions.

## Higher Goal
The shipped application has zero monetization UI so the project remains unmistakably non-commercial under a $0 legal budget.

## Acceptance Criteria
- [ ] Remove the GitHub Sponsors button from the GUI (`crates/nord-gui`)
- [ ] Owner decides whether to also remove the README Sponsors badge and `.github/FUNDING.yml`
- [ ] All quality gates pass

## Out of Scope
Changing the project license.
