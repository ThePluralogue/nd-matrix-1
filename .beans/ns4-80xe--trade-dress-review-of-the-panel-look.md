---
# ns4-80xe
title: 'Proactive trade dress mitigation: neutral non-red default panel theme'
status: todo
type: task
priority: high
created_at: 2026-11-23T16:00:00Z
updated_at: 2026-11-23T21:25:00Z
parent: ns4-9wq1
---

## Context
All shipped art is original (D16; vendor photos removed and blocked by `tools/check_no_vendor_photos.py`; the Blender proof `assets/blender/build_proof.py` builds geometry from primitives with procedural materials). However, the panel still reproduces the hardware's signature red chassis colour and section layout closely (`crates/nord-gui/assets/baked/panel_plate_2x.png`). Trade dress (overall look and feel) is a separate risk from copyright and trademarks, and under a $0 legal budget the safest move is proactive visual differentiation rather than paying counsel to defend a red panel.

## Higher Goal
The default GUI look and all public screenshots have a distinct, non-red ND Matrix 1 visual identity that does not mimic the vendor's signature red hardware chassis.

## Acceptance Criteria
- [ ] Design and bake a neutral, non-red default chassis theme (e.g., dark charcoal/slate/navy) with clear ND Matrix 1 branding
- [ ] Owner request 2026-11-23: change colors, images and other visual elements (knob and button shapes, key colors, section accents, typography, plate texture) so the GUI looks clearly different from the original Nord panel, not only a recolor of the chassis
- [ ] Make the neutral theme the default in `nord-gui` and `nord-plugin`
- [ ] Refresh README and release screenshots (`docs/img/gui/`) to show the neutral default theme
- [ ] Decision recorded in `docs/DECISIONS.md` and `docs/specs/visual-style-and-panel.md`

## Out of Scope
Removing the panel GUI.
