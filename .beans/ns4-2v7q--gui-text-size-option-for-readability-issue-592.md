---
# ns4-2v7q
title: 'GUI text size option for readability (issue #592)'
status: todo
type: task
priority: high
created_at: 2026-11-24T02:05:15Z
updated_at: 2026-11-24T02:05:15Z
parent: ns4-xc5n
---

## Context
Issue ymatrix1/nd-matrix-1#592 ("Gui Text"): a user with poor eyesight cannot read the panel settings and asks for larger text. The owner replied they plan a config option for a double-stacked layout dividing the controls into two horizontal sections.

## Higher Goal
The panel is readable by users with low vision, in the standalone app and the DAW plugin editor.

## Acceptance Criteria
- [ ] Text scale option (for example 100 to 200 percent) in the standalone GUI and plugin editor, persisted in the app settings, applied to panel labels, OLED-adjacent text, menus and dialogs
- [ ] Layout stays usable at the largest scale (no clipped or overlapping labels); headless golden-image test at 100 and 150 percent
- [ ] Double-stacked layout option (controls in two horizontal sections) designed in docs/specs/visual-style-and-panel.md; implemented when the text scale lands cleanly, or split into its own bean
- [ ] Spec, README GUI section and docs/ROADMAP.md updated; issue #592 answered with the release that ships it

## Out of Scope
Changing the panel palette or baked art (ns4-80xe).
