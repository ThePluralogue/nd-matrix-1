"""Regression tests for the project license (D231)."""

from pathlib import Path

import license_check as lc


def _write_tree(root: Path, license_id: str, publish: str, license_text: str | None) -> None:
    (root / "crates" / "a").mkdir(parents=True)
    (root / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["crates/a"]\n[workspace.package]\n'
        f'license = "{license_id}"\n{publish}\n'
    )
    (root / "crates" / "a" / "Cargo.toml").write_text(
        '[package]\nname = "a"\nlicense.workspace = true\npublish.workspace = true\n'
    )
    (root / "pyproject.toml").write_text(f'[project]\nlicense = "{license_id}"\n')
    if license_text is not None:
        (root / "LICENSE").write_text(license_text)


def test_root_license_file_is_polyform_noncommercial():
    assert lc.license_file_ok()
    text = (lc.ROOT / "LICENSE").read_text(encoding="utf-8")
    assert text.startswith("# PolyForm Noncommercial License 1.0.0")
    assert "Required Notice:" in text


def test_every_crate_uses_polyform_and_is_unpublishable():
    settings = lc.crate_settings()
    assert settings, "no crates found"
    for name, (license_id, publish) in settings.items():
        assert license_id == "PolyForm-Noncommercial-1.0.0", name
        assert publish is False, name


def test_repository_has_no_problems():
    assert lc.problems() == []


def test_detects_permissive_license_missing_file_and_publish(tmp_path):
    _write_tree(tmp_path, "MIT OR Apache-2.0", "", None)
    found = lc.problems(tmp_path)
    assert any("LICENSE missing" in p for p in found)
    assert any("a: license is" in p for p in found)
    assert any("a: publish is" in p for p in found)
    assert any("pyproject.toml" in p for p in found)


def test_clean_tree_has_no_problems(tmp_path):
    _write_tree(
        tmp_path, lc.SPDX_ID, "publish = false", lc.LICENSE_HEADER + "\n\nRequired Notice: x\n"
    )
    _write_legal(tmp_path)
    assert lc.problems(tmp_path) == []


def _write_legal(root: Path) -> None:
    (root / "ADDITIONAL-PERMISSIONS.md").write_text("a permitted purpose; no selling")
    (root / "TRADEMARKS.md").write_text("use a different name")


def test_repository_legal_files_carry_key_phrases():
    assert lc.legal_file_problems() == []
    perms = (lc.ROOT / "ADDITIONAL-PERMISSIONS.md").read_text(encoding="utf-8")
    assert "paid performances and commercial recordings" in perms
    assert "does not allow selling" in perms


def test_legal_files_missing_or_stripped_are_reported(tmp_path):
    assert lc.legal_file_problems(tmp_path) == [
        "ADDITIONAL-PERMISSIONS.md missing",
        "TRADEMARKS.md missing",
    ]
    (tmp_path / "ADDITIONAL-PERMISSIONS.md").write_text("nothing")
    (tmp_path / "TRADEMARKS.md").write_text("nothing")
    found = lc.legal_file_problems(tmp_path)
    assert "ADDITIONAL-PERMISSIONS.md lacks 'permitted purpose'" in found
    assert "ADDITIONAL-PERMISSIONS.md lacks 'selling'" in found
    assert "TRADEMARKS.md lacks 'different name'" in found


def test_release_readme_carries_noncommercial_notice():
    import release_installers as ri

    readme = (lc.ROOT / "docs" / "RELEASE_README.txt").read_text(encoding="utf-8")
    assert ri.LICENSE_NOTICE in readme
    assert "commercial use is not permitted" in ri.LICENSE_NOTICE


def test_macos_license_fallback_is_the_notice(tmp_path):
    import release_installers as ri

    stage, res = tmp_path / "stage", tmp_path / "res"
    stage.mkdir()
    res.mkdir()
    ri.write_macos_license(stage, res)
    assert (res / "LICENSE").read_text(encoding="utf-8").strip() == ri.LICENSE_NOTICE
    (stage / "LICENSE").write_text("full text", encoding="utf-8")
    ri.write_macos_license(stage, res)
    assert (res / "LICENSE").read_text(encoding="utf-8") == "full text"
