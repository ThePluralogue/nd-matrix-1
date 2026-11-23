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

**libfuse is licensed under LGPL-2.1 and is statically linked into the AppImage runtime.** The
complete corresponding source and the patch are provided as the release asset
`nd-matrix-1-<tag>-appimage-runtime-sources.tar.gz` (next to the `.AppImage` on the release page,
with a `.sha256`). It contains, with these pins:

| File | Origin | Pin |
|---|---|---|
| `fuse-3.15.0.tar.xz` | https://github.com/libfuse/libfuse/releases/download/fuse-3.15.0/fuse-3.15.0.tar.xz | SHA-256 `70589cfd5e1cff7ccd6ac91c86c01be340b227285c5e200baa284e401eea2ca0` |
| `squashfuse-0.5.2.tar.gz` | https://github.com/vasi/squashfuse/archive/0.5.2.tar.gz | SHA-256 `db0238c5981dabbd80ee09ae15387f390091668ca060a7bc38047912491443d3` |
| `type2-runtime-20251108.tar.gz` | `git archive` of https://github.com/AppImage/type2-runtime tag `20251108` (build scripts, `BUILD.md`, runtime source) | commit `dd6cebedcbddde9c82f89b011e8e1d40b6e43868` |
| `patches/libfuse/mount.c.diff` | the libfuse patch from the runtime source | same commit |

You may modify libfuse and replace the runtime in the AppImage (relinking):

1. Unpack the three tarballs and change libfuse in `fuse-3.15.0/`.
2. Rebuild the runtime from the type2-runtime source as its `BUILD.md` says (for example
   `env ARCH=x86_64 scripts/build-with-docker.sh`); `scripts/common/install-dependencies.sh` there
   builds libfuse and squashfuse from the tarballs, so point it at your modified libfuse.
3. Unpack the AppImage (`./app.AppImage --appimage-extract`) and repack it with
   `appimagetool --runtime-file <your runtime> squashfs-root <new>.AppImage`.

The runtime is a separate program: our application is data inside the SquashFS image and is not
linked with libfuse. `SOURCES.txt` in the asset repeats these steps.

## Provenance of these texts

Fetched over HTTPS from the upstream tags named above: `type2-runtime` tag `20251108` (`LICENSE`),
`libfuse` tag `fuse-3.15.0` (`LGPL2.txt`), `squashfuse` tag `0.5.2` (`LICENSE`), `zstd` tag
`v1.5.6` (`LICENSE`), `zlib` tag `v1.3.1` (header comment of `zlib.h`), `mimalloc` tag `v2.1.7`
(`LICENSE`), `musl` tag `v1.2.5` (`COPYRIGHT`, from a GitHub mirror because
`git.musl-libc.org` was unreachable). This directory is checked by `tools/release_verify.py`
(Linux archives) and copied into the AppImage under `usr/share/doc/nd-matrix-1/appimage-runtime/`.
