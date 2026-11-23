---
# ns4-lp7q
title: 'Plugin: sound library setting and library_program parameter (headless program loading)'
status: completed
type: task
created_at: 2026-11-23T15:30:00Z
updated_at: 2026-11-23T15:30:00Z
---

## Context
A DAW without plugin windows (SoundCraft on Windows: no VST3 editor hosting) never reaches the editor's OPEN .NS4B, so the CLAP/VST3 plugin only played its built-in engine, and a reopened project lost its program (TD-21 c). Bounces in such a host build fresh instances and render at once.

## Higher Goal
The plugin plays the programs of the user's own `.ns4b` backup, with their piano and sample banks, in any host, and a project sounds the same when reopened.

## Acceptance Criteria
- [x] `ns4 library set|status|clear` remembers a `.ns4b` path next to the OS update setting (`NS4_LIBRARY` overrides, `none` disables)
- [x] Host parameter `library_program` (Off, A:11..H:88) selects the program; it is saved with the project
- [x] A worker thread decodes the program and its referenced banks and queues them through the D218 load queue; `initialize` waits for it so a bounce contains the program
- [x] Decoded banks are shared between instances of one process
- [x] WAVE stage switched on once the OS update tables are installed
- [x] Unit tests for every new module; plugin tests ignore the user's saved library
- [x] Verified in a DAW: SoundCraft 0.20.1 bounces of A:11 and A:14 differ from the built-in engine; A:14 restored from a saved project

## Out of Scope
- Persisting a program opened by hand in the editor (TD-21 c for the editor path)
- Mirroring program values into host parameters (TD-21 a, b)
- A program browser in the editor for the library parameter

## Summary of Changes
`nord-core`: `library_setting.rs`, `user_config::library_path`, `synth::wave_tables_installed`, `ns4 library`. `nord-plugin`: `library_select.rs`, `auto_library.rs`, `shared_sounds.rs`, `LoadJob::WaveBank`, `initialize` waits for the program, `plugin.log`. Docs: `docs/specs/daw-plugin.md`, `docs/guides/plugin-sound-library.md`, README, TECH_DEBT.
