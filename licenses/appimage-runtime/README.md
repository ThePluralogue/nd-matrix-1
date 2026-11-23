# AppImage runtime notices

Every `nd-matrix-1-<tag>-linux-x86_64.AppImage` starts with a small executable, the AppImage
runtime, followed by a SquashFS image of our application. `appimagetool` writes the runtime in;
it is not our code. This directory holds the license texts that the runtime requires. Our own
license is `LICENSE`; the Rust crates in our binaries are listed in `THIRD_PARTY_LICENSES.html`.

## Which runtime

| Item | Value |
|---|---|
| Runtime | AppImage type2-runtime, release `20251108` (commit `dd6cebedcbddde9c82f89b011e8e1d40b6e43868`), file `runtime-x86_64` |
| Runtime SHA-256 | `2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d` (944632 bytes) |
| Download | https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64 |
| Source | https://github.com/AppImage/type2-runtime/tree/20251108 |
| Written by | `appimagetool` 1.9.0 (https://github.com/AppImage/appimagetool/tree/1.9.0), passed with `--runtime-file` |

## What is linked into the runtime

The runtime is one static-PIE ELF built on Alpine 3.21 (`scripts/docker/Dockerfile` in the source
tree). `src/runtime/Makefile` links `-lsquashfuse -lsquashfuse_ll -lzstd -lz -lfuse3 -lmimalloc`
against musl. The strings of the shipped file show `squashfuse 0.5.2`, libfuse `3.15.0`, zstd
`1.5.6` and zlib `1.3.1`.

| Component | Version | License | Text here |
|---|---|---|---|
| AppImage runtime (`runtime.c`) | 20251108 | MIT, copyright 2004-23 probonopd | `type2-runtime-LICENSE.txt` |
| musl libc | Alpine 3.21 package (1.2.5 text) | MIT, plus permissive terms for a few files | `musl-LICENSE.txt` |
| libfuse (patched) | 3.15.0 | **LGPL-2.1-only**, copyright Miklos Szeredi and contributors | `libfuse-LGPL-2.1.txt` |
| squashfuse | 0.5.2 | BSD-2-Clause, copyright 2012 Dave Vasilevsky and Phillip Lougher | `squashfuse-LICENSE.txt` |
| zstd | 1.5.6 | BSD-3-Clause (we use it under this option of its BSD or GPL-2.0 dual license), copyright Meta Platforms, Inc. and affiliates | `zstd-LICENSE.txt` |
| zlib | 1.3.1 | zlib license, copyright 1995-2024 Jean-loup Gailly and Mark Adler | `zlib-LICENSE.txt` |
| mimalloc | Alpine 3.21 package (version not verified) | MIT, copyright 2018-2021 Microsoft Corporation, Daan Leijen | `mimalloc-LICENSE.txt` |

The component list comes from the upstream `LICENSE` of the runtime (which names musl, libfuse,
squashfuse, libzstd and zlib), `src/runtime/Makefile` (which adds mimalloc) and `strings` on the
shipped binary (versions). The musl and mimalloc versions are those of the Alpine 3.21 packages
(not checked: `gitlab.alpinelinux.org` was unreachable from the build environment); the license
text of musl is from tag `v1.2.5`, that of mimalloc from tag `v2.1.7`. Both are MIT in every
release.

## libfuse (LGPL-2.1): your rights

libfuse is statically linked into the runtime. Under LGPL-2.1 section 6 you may modify libfuse
and relink the runtime. The complete corresponding source is public:

- runtime source, build scripts and the libfuse patch (`patches/libfuse/mount.c.diff`):
  https://github.com/AppImage/type2-runtime/tree/20251108
- libfuse 3.15.0 source (SHA-256 `70589cfd5e1cff7ccd6ac91c86c01be340b227285c5e200baa284e401eea2ca0`):
  https://github.com/libfuse/libfuse/releases/download/fuse-3.15.0/fuse-3.15.0.tar.xz
- squashfuse 0.5.2 source (SHA-256 `db0238c5981dabbd80ee09ae15387f390091668ca060a7bc38047912491443d3`):
  https://github.com/vasi/squashfuse/archive/0.5.2.tar.gz

`BUILD.md` in the runtime source explains how to rebuild it. To use a rebuilt runtime, run
`appimagetool --runtime-file <your runtime> <AppDir> <output>` on the unpacked AppImage
(`./app.AppImage --appimage-extract`). The runtime is a separate program: our application is data
inside the SquashFS image and is not linked with libfuse.

## Provenance of these texts

Fetched over HTTPS from the upstream tags named above: `type2-runtime` tag `20251108` (`LICENSE`),
`libfuse` tag `fuse-3.15.0` (`LGPL2.txt`), `squashfuse` tag `0.5.2` (`LICENSE`), `zstd` tag
`v1.5.6` (`LICENSE`), `zlib` tag `v1.3.1` (header comment of `zlib.h`), `mimalloc` tag `v2.1.7`
(`LICENSE`), `musl` tag `v1.2.5` (`COPYRIGHT`, from a GitHub mirror because
`git.musl-libc.org` was unreachable). This directory is checked by `tools/release_verify.py`
(Linux archives) and copied into the AppImage under `usr/share/doc/nd-matrix-1/appimage-runtime/`.
