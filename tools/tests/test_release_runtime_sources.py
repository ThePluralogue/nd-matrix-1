import gzip
import hashlib
import io
import tarfile

import pytest
import release_package as rp
import release_runtime_sources as rrs
from test_release_package import LIN, make_tree


def _runtime_archive(patch=b"--- patch ---\n"):
    prefix = rrs.RUNTIME_ARCHIVE[:-7]
    raw = io.BytesIO()
    with tarfile.open(fileobj=raw, mode="w") as tf:
        info = tarfile.TarInfo(f"{prefix}/{rrs.PATCH_PATH}")
        info.size = len(patch)
        tf.addfile(info, io.BytesIO(patch))
    return gzip.compress(raw.getvalue())


def _fetcher(monkeypatch, bad=None):
    data = {url: f"payload {name}".encode() for name, url, _ in rrs.TARBALLS}
    pins = tuple(
        (name, url, hashlib.sha256(data[url]).hexdigest()) for name, url, _ in rrs.TARBALLS
    )
    monkeypatch.setattr(rrs, "TARBALLS", pins)
    if bad:
        data[bad] = b"tampered"
    return data.__getitem__


def test_build_writes_complete_deterministic_asset(tmp_path, monkeypatch):
    fetch = _fetcher(monkeypatch)
    asset = rrs.build(tmp_path / "a", "v1.2.3", fetch, _runtime_archive)
    assert asset.name == "nd-matrix-1-v1.2.3-appimage-runtime-sources.tar.gz"
    with tarfile.open(asset, "r:gz") as tf:
        names = {m.name.split("/", 1)[1] for m in tf.getmembers()}
        top = {m.name.split("/", 1)[0] for m in tf.getmembers()}
        patch = tf.extractfile(f"{asset.name[:-7]}/{rrs.PATCH_PATH}").read()
    assert names == {n for n in rrs.SOURCES_FILES}
    assert top == {asset.name[:-7]}
    assert patch == b"--- patch ---\n"
    sidecar = asset.with_name(asset.name + ".sha256").read_text()
    assert sidecar == f"{rp.sha256_hex(asset)}  {asset.name}\n"
    again = rrs.build(tmp_path / "b", "v1.2.3", fetch, _runtime_archive)
    assert rp.sha256_hex(again) == rp.sha256_hex(asset)


def test_build_rejects_a_tarball_with_the_wrong_sha256(tmp_path, monkeypatch):
    fetch = _fetcher(monkeypatch)
    bad_url = rrs.TARBALLS[0][1]
    fetch = _fetcher(monkeypatch, bad=bad_url)
    with pytest.raises(ValueError, match="expected the pinned"):
        rrs.build(tmp_path, "v1.2.3", fetch, _runtime_archive)


def test_build_fails_when_runtime_source_lacks_the_patch(tmp_path, monkeypatch):
    fetch = _fetcher(monkeypatch)
    with pytest.raises(KeyError):
        rrs.build(tmp_path, "v1.2.3", fetch, lambda: gzip.compress(b"\0" * 1024))
    with pytest.raises((tarfile.TarError, KeyError, OSError)):
        rrs.patch_from_runtime_archive(b"not a tar")


def test_sources_txt_names_every_pin_and_the_relink_steps():
    text = rrs.render_sources_txt()
    for _, url, sha in rrs.TARBALLS:
        assert url in text and sha in text
    assert rrs.RUNTIME_COMMIT in text and "--runtime-file" in text
    assert chr(0x2014) not in text


def test_pins_match_the_committed_notice_readme():
    readme = (rp.ROOT / rp.APPIMAGE_RUNTIME_DIR / "README.md").read_text(encoding="utf-8")
    assert rrs.RUNTIME_COMMIT in readme
    for _, _, sha in rrs.TARBALLS:
        assert sha in readme
    assert "appimage-runtime-sources.tar.gz" in readme


def test_release_workflow_builds_the_sources_and_skips_it_in_the_verify_loop():
    workflow = (rp.ROOT / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")
    assert "tools/release_runtime_sources.py" in workflow
    assert "*-appimage-runtime-sources.tar.gz) continue" in workflow
    assert workflow.index("release_runtime_sources.py") < workflow.index("release_verify.py")


def test_main_builds_with_dev_label(tmp_path, monkeypatch, capsys):
    root = make_tree(tmp_path, LIN)
    seen = {}

    def fake_build(out_dir, label):
        seen["label"] = label
        return out_dir / "x"

    monkeypatch.setattr(rrs, "build", fake_build)
    assert rrs.main(["--root", str(root), "--out", str(tmp_path / "dist")]) == 0
    assert seen["label"] == "v1.2.3-dev"
    assert "x" in capsys.readouterr().out
