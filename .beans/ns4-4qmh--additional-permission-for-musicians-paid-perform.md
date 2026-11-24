---
# ns4-4qmh
title: 'Additional permission for musicians (paid performances and recordings)'
status: in-progress
type: task
priority: normal
created_at: 2026-11-23T16:00:00Z
updated_at: 2026-11-23T16:00:00Z
parent: ns4-9wq1
---

## Context
PolyForm Noncommercial 1.0.0 counts personal use as permitted only "without any anticipated commercial application". A musician who plays the emulator on a paid gig or a sold recording is arguably outside the license. The licensor can grant extra permissions.

## Higher Goal
Musicians can use the instrument for their work while selling, sublicensing or bundling the software stays forbidden.

## Acceptance Criteria
- [x] Owner approves wording, for example: "Additional permission: using the software to make, perform or record music, including paid performances and commercial recordings, is a permitted purpose. Selling, sublicensing or bundling the software itself is not."
- [ ] Wording reviewed by counsel (see the counsel review bean)
- [x] Added next to the license (README License section and a `Required Notice:` or NOTICE file shipped in releases), with a decision in docs/DECISIONS.md
- [x] `tools/license_check.py` test pins the presence of the permission

## Out of Scope
Changing the base license.

## Summary of Changes
Drafted `ADDITIONAL-PERMISSIONS.md` (root, separate from the unmodified `LICENSE`), shipped in every release layout and enforced by `release_package.py`, `release_installers.py`, `release_verify.py`; `license_check.py` pins "permitted purpose" and "selling"; README, CONTRIBUTING, LEGAL.md s4 and RELEASING.md link it; D241. The README says the wording is pending counsel review. Owner approval: given in session ("do everything you recommend"). Still open: counsel review.
