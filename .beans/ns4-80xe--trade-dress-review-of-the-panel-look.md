---
# ns4-80xe
title: 'Trade dress review of the panel look'
status: todo
type: task
priority: normal
created_at: 2026-11-23T16:00:00Z
updated_at: 2026-11-23T16:00:00Z
parent: ns4-9wq1
---

## Context
All shipped art is original (D16; vendor photos removed and blocked by `tools/check_no_vendor_photos.py`; the Blender proof `assets/blender/build_proof.py` builds geometry from primitives with procedural materials). The panel still reproduces the hardware's red chassis colour and section layout closely (`crates/nord-gui/assets/baked/panel_plate_2x.png`). Trade dress (overall look and feel) is a separate question from copyright and trademarks.

## Higher Goal
The GUI's resemblance to the hardware is a conscious, reviewed choice with a documented risk level.

## Acceptance Criteria
- [ ] Counsel opinion on trade dress risk of a close visual match (colour, layout, proportions) for an interoperability emulator
- [ ] Owner decides: keep, add distinguishing elements (own colour theme option, wordmark, layout cues), or make a neutral theme the default
- [ ] Decision recorded in docs/DECISIONS.md and docs/specs/visual-style-and-panel.md

## Out of Scope
Removing the panel GUI.
