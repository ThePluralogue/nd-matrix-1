---
# ns4-6fbu
title: 'Repository settings: required checks and description'
status: todo
type: task
priority: normal
created_at: 2026-11-23T16:00:00Z
updated_at: 2026-11-23T16:00:00Z
parent: ns4-9wq1
---

## Context
DCO (D232), the firmware ratchet (D128), the copyleft gate and third-party notices (D233, D238) run in CI but are not required status checks, so a PR can merge while a job is missing or cancelled. The repository description still says "Open-source", which contradicts the PolyForm Noncommercial license (D231).

## Higher Goal
Branch protection enforces the legal gates, and public metadata matches the license.

## Acceptance Criteria
- [ ] Branch protection on `main` requires: Rust DSP & CBIN Core (ubuntu, windows, macos), Python Tools & Unit-Test Ratchet, DCO sign-off on PR commits, No copyleft crate in the shipped plugin tree, Third-party license notices are complete
- [ ] Repository description says "Source-available (non-commercial)" instead of "Open-source"
- [ ] Repository license field on GitHub shows the PolyForm license (or "Other")

## Out of Scope
CI cost changes for macOS runners.
