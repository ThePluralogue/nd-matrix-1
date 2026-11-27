#!/usr/bin/env python3
"""Build the corresponding-source asset for the AppImage runtime (LGPL-2.1, D242).

The `.AppImage` starts with the type2-runtime, which links libfuse (LGPL-2.1) statically.
LGPL-2.1 section 6 requires the complete source of the library (with our patches) and the means
to relink, delivered with the binary. This tool writes
`nd-matrix-1-<label>-appimage-runtime-sources.tar.gz` (+ `.sha256`) next to the release archives:

    SOURCES.txt                       what is inside, pins, relink instructions
    fuse-3.15.0.tar.xz                libfuse source, sha256 pinned
    squashfuse-0.5.2.tar.gz           squashfuse source, sha256 pinned
    type2-runtime-20251108.tar.gz     runtime source, `git archive` of the pinned commit
    patches/libfuse/mount.c.diff      the libfuse patch, extracted for convenience

Downloads are checked against the sha256 pins below and the runtime clone against its full commit
id, so a changed upstream fails the release instead of shipping other source.

Usage: python tools/release_runtime_sources.py --out dist [--tag vX.Y.Z]
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
from collections.abc import Callable
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

RUNTIME_TAG = "20251108"
RUNTIME_REPO = "https://github.com/AppImage/type2-runtime"
# VERIFIED: `git ls-remote --tags` and `git rev-parse HEAD` of a clone at tag 20251108.
RUNTIME_COMMIT = "dd6cebedcbddde9c82f89b011e8e1d40b6e43868"
RUNTIME_ARCHIVE = f"type2-runtime-{RUNTIME_TAG}.tar.gz"
PATCH_PATH = "patches/libfuse/mount.c.diff"
# (file name, url, sha256). The libfuse hash was computed here on the downloaded file
# (4589124 bytes); the squashfuse hash is the one `scripts/common/install-dependencies.sh` of the
# runtime at the pinned commit checks before building (not downloadable from the build host that
# wrote this, so VERIFIED only by upstream's own build script).
TARBALLS = (
    (
        "fuse-3.15.0.tar.xz",
        "https://github.com/libfuse/libfuse/releases/download/fuse-3.15.0/fuse-3.15.0.tar.xz",
        "70589cfd5e1cff7ccd6ac91c86c01be340b227285c5e200baa284e401eea2ca0",
    ),
    (
        "squashfuse-0.5.2.tar.gz",
        "https://github.com/vasi/squashfuse/archive/0.5.2.tar.gz",
        "db0238c5981dabbd80ee09ae15387f390091668ca060a7bc38047912491443d3",
    ),
)
SOURCES_FILES = (
    "SOURCES.txt",
    *(name for name, _, _ in TARBALLS),
    RUNTIME_ARCHIVE,
    PATCH_PATH,
)


def sources_asset_name(label: str) -> str:
    return f"nd-matrix-1-{label}-appimage-runtime-sources.tar.gz"


def sources_name_for_archive(archive_name: str) -> str | None:
    """Sources asset next to a `nd-matrix-1-<label>-linux-x86_64.tar.gz` archive, else None."""
    suffix = "-linux-x86_64.tar.gz"
    if not archive_name.endswith(suffix):
        return None
    return archive_name[: -len(suffix)] + "-appimage-runtime-sources.tar.gz"


def render_sources_txt() -> str:
    lines = [
        "Corresponding source of the AppImage runtime (LGPL-2.1 section 6)",
        "",
        "The .AppImage starts with the type2-runtime. It links libfuse 3.15.0 (LGPL-2.1) and",
        "squashfuse 0.5.2 statically. This archive holds the complete source used to build it:",
        "",
    ]
    for name, url, sha in TARBALLS:
        lines += [f"  {name}", f"    {url}", f"    sha256 {sha}"]
    lines += [
        f"  {RUNTIME_ARCHIVE}",
        f"    {RUNTIME_REPO}/tree/{RUNTIME_TAG} (commit {RUNTIME_COMMIT}),",
        "    git archive of that commit; it contains the build scripts and the libfuse patch.",
        f"  {PATCH_PATH}",
        "    the patch applied to libfuse, copied out of the runtime source.",
        "",
        "To replace the runtime with your own build (relinking with a modified libfuse):",
        "  1. Unpack the three tarballs. Edit libfuse in fuse-3.15.0/.",
        "  2. Rebuild the runtime as BUILD.md in the runtime source says (Docker build:",
        "     env ARCH=x86_64 scripts/build-with-docker.sh); its",
        "     scripts/common/install-dependencies.sh builds libfuse and squashfuse from the",
        "     tarballs, so point it at your modified libfuse.",
        "  3. Unpack the AppImage (./app.AppImage --appimage-extract) and repack it with",
        "     appimagetool --runtime-file <your runtime> squashfs-root <new>.AppImage",
        "",
        "Licenses: licenses/appimage-runtime/ in the release archive, or",
        "usr/share/doc/nd-matrix-1/appimage-runtime/ inside the AppImage.",
        "",
    ]
    return "\n".join(lines)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def download(url: str) -> bytes:
    with urllib.request.urlopen(url, timeout=300) as response:
        return response.read()


def git_archive(commit: str = RUNTIME_COMMIT) -> bytes:
    """`git archive` of the pinned runtime commit (a clone is checked against `commit`)."""
    with tempfile.TemporaryDirectory() as tmp:
        repo = Path(tmp) / "type2-runtime"
        subprocess.run(["git", "clone", "-q", RUNTIME_REPO, str(repo)], check=True)
        head = subprocess.run(
            ["git", "-C", str(repo), "rev-parse", f"{RUNTIME_TAG}^{{commit}}"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        if head != commit:
            raise ValueError(f"tag {RUNTIME_TAG} is {head}, expected the pinned {commit}")
        tar = subprocess.run(
            [
                "git",
                "-C",
                str(repo),
                "archive",
                "--format=tar",
                f"--prefix={RUNTIME_ARCHIVE[:-7]}/",
                commit,
            ],
            check=True,
            capture_output=True,
        ).stdout
    out = io.BytesIO()
    with gzip.GzipFile(fileobj=out, mode="wb", mtime=0) as gz:
        gz.write(tar)
    return out.getvalue()


def patch_from_runtime_archive(data: bytes) -> bytes:
    """Read the libfuse patch out of the runtime source archive."""
    prefix = RUNTIME_ARCHIVE[:-7]
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tf:
        handle = tf.extractfile(f"{prefix}/{PATCH_PATH}")
        if handle is None:
            raise ValueError(f"{PATCH_PATH} is not a file in {RUNTIME_ARCHIVE}")
        return handle.read()


def build(
    out_dir: Path,
    label: str,
    fetch: Callable[[str], bytes] = download,
    archive_runtime: Callable[[], bytes] = git_archive,
) -> Path:
    """Write the sources asset and its `.sha256`; fail on any hash mismatch."""
    members: dict[str, bytes] = {"SOURCES.txt": render_sources_txt().encode("utf-8")}
    for name, url, want in TARBALLS:
        data = fetch(url)
        got = sha256_bytes(data)
        if got != want:
            raise ValueError(f"{url} has sha256 {got}, expected the pinned {want}")
        members[name] = data
    runtime = archive_runtime()
    members[RUNTIME_ARCHIVE] = runtime
    members[PATCH_PATH] = patch_from_runtime_archive(runtime)

    out_dir.mkdir(parents=True, exist_ok=True)
    asset = out_dir / sources_asset_name(label)
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w", format=tarfile.PAX_FORMAT) as tf:
        for name in SOURCES_FILES:
            info = tarfile.TarInfo(f"{asset.name[:-7]}/{name}")
            info.size = len(members[name])
            info.mode = 0o644
            info.mtime = 0
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            tf.addfile(info, io.BytesIO(members[name]))
    with (
        asset.open("wb") as handle,
        gzip.GzipFile(filename="", fileobj=handle, mode="wb", mtime=0) as gz,
    ):
        gz.write(raw.getvalue())
    from release_package import write_sha256

    write_sha256(asset)
    return asset


def main(argv: list[str] | None = None) -> int:
    from release_package import version_from_tag, workspace_version

    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--tag", help="release tag vX.Y.Z; omit for v<cargo_version>-dev")
    parser.add_argument("--out", type=Path, default=Path("dist"))
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args(argv)
    cargo_version = workspace_version(args.root)
    label = f"v{version_from_tag(args.tag, cargo_version)}" if args.tag else f"v{cargo_version}-dev"
    print(build(args.out, label))
    return 0


if __name__ == "__main__":
    sys.exit(main())
