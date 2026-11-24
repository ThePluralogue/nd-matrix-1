---
# ns4-lle6
title: 'Wire live MIDI, panel controls, .ns4b store and OLED into LLE backend'
status: todo
type: task
priority: normal
created_at: 2026-11-23T21:40:00Z
updated_at: 2026-11-23T21:40:00Z
parent: ns4-lle1
blocked_by:
    - ns4-lle5
---

## Context
To be playable as an instrument, the unified LLE engine (`ns4-lle5`) needs live user input (MIDI notes, CCs, sustain/control pedals, pitch stick, mod wheel, panel knob/button events), program switching from the user's `.ns4b` backup (`ns4-pyqg`), and live OLED/LED telemetry (`ns4-2gdc`).

## Higher Goal
A musician can play the LLE engine live from a MIDI keyboard and the ND Matrix 1 panel UI, switch programs from their `.ns4b` backup, and see the real firmware OLED screens and LEDs respond in real time.

## Acceptance Criteria
- [ ] Inject live MIDI (note on/off, velocity, aftertouch, CC, pitch bend, program change) into ARM0 input queues with lock-free ring buffers
- [ ] Map ND Matrix 1 panel knob/button/drawbar edits to ARM0 panel input events
- [ ] Populate the ARM0 program store from the user's `.ns4b` backup (`ns4-pyqg`)
- [ ] Stream real firmware OLED framebuffers (`ns4-2gdc`) and LED states back to the GUI via lock-free telemetry
- [ ] Unit tests for MIDI/panel event translation and telemetry snapshots

## Out of Scope
Default backend switch (`ns4-lle7`).
