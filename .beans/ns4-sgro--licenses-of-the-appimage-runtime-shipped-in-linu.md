---
# ns4-sgro
title: 'Licenses of the AppImage runtime shipped in Linux releases'
status: in-progress
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
- [x] Identify the exact runtime version used by `tools/release_installers.py` and its license(s) from the upstream source
- [x] Ship its notice in the AppImage and the release archive, or switch to a runtime with clear terms
- [x] `tools/release_verify.py` checks the notice; tests added
- [x] LEGAL.md question 9 answered
- [ ] Owner decision on the LGPL-2.1 source step for the statically linked libfuse (LEGAL.md section 4, options A to D)

## Out of Scope
Other Linux packaging formats.

## Summary of Changes
- Finding: `appimagetool` 1.9.0 (pinned) downloads the moving `continuous` type2-runtime when `--runtime-file` is absent (`src/appimagetool_fetch_runtime.cpp`, tag `1.9.0`), so the runtime was unpinned. It is now pinned to release `20251108` (commit `dd6cebe`, sha256 `2fca8b44...ec260d`): `release.yml` downloads it, `tools/release_installers.py --appimage-runtime` verifies the hash and passes `--runtime-file` (D241).
- Runtime licenses from the upstream source and `strings` on the binary: type2-runtime MIT; statically linked musl (MIT), libfuse 3.15.0 (LGPL-2.1, patched), squashfuse 0.5.2 (BSD-2-Clause), zstd 1.5.6 (BSD-3-Clause), zlib 1.3.1 (zlib), mimalloc (MIT, version not verified). Hosts blocked: `api.github.com`, release pages on `github.com`, `gitlab.alpinelinux.org`, `git.musl-libc.org`.
- `licenses/appimage-runtime/` (README with versions, sources, sha256 sums, and the seven texts) ships in the AppImage (`usr/share/doc/nd-matrix-1/appimage-runtime/`) and in the Linux archive; `tools/release_verify.py` fails a Linux archive without it, with an empty file, or with a non-LGPL libfuse text. Tests in `tools/tests/test_release_{package,installers,verify}.py`.
- LEGAL.md section 4 (component table, LGPL analysis, options A to D) and section 8 question 9, RELEASING.md (runtime bump procedure), D241.
- Remaining: libfuse is LGPL-2.1 and statically linked; we do not host its source or make a written offer. The owner chooses among options A (mirror sources as release assets), B (dynamic runtime), C (other runtime) and D (drop AppImage). Not resolved here by design.
