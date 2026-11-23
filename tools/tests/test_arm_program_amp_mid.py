"""Unit tests for tools/arm_program_amp_mid.py and tools/fx_lp24_mid_sweep.py (no firmware)."""

from __future__ import annotations

import io
import json
import math
import sys
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import arm_program_amp_mid as am  # noqa: E402
import fx_lp24_mid_sweep as sweep  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
TABLES = ROOT / "crates/nord-core/src/fx/tables.rs"


def block(kind: int, enabled: bool, db: float, norm: float) -> dict:
    return {
        "amp_type_new": kind,
        "amp_enabled": enabled,
        "eq_mid_db": db,
        "eq_mid_freq_norm": norm,
    }


def test_edit_ids_are_the_registry_ids() -> None:
    assert am.EDIT_IDS == {"type": 0x1A7, "mid": 0x1A0, "freq": 0x1A2}


def test_signed32_reads_negative_cells() -> None:
    assert am.signed32(0xFFFFFFC4) == -60
    assert am.signed32(0x27) == 39


def test_expected_tuples_take_enabled_filter_amps_from_nested_blocks() -> None:
    decoded = {
        "synth": {"fx_a": block(201, True, -15.0, 39 / 127), "fx_b": block(2, True, 0.0, 0.5)},
        "piano": {"fx_a": block(8, False, 3.0, 0.2)},
        "list": [block(8, True, 4.5, 80 / 127)],
    }
    assert sorted(am.expected_tuples(decoded)) == [(8, 18, 80), (9, -60, 39)]


def test_compare_reports_missing_and_empty() -> None:
    assert am.compare([(8, 0, 10)], [(8, 0, 10), (9, 1, 1)]) == []
    assert am.compare([(8, 0, 10)], [(8, 0, 11)])[0].startswith("file (8, 0, 10)")
    assert "no recalled" in am.compare([(8, 0, 10)], [])[-1]
    assert am.compare([], []) == []


def test_recalled_tuples_keep_only_lp24_hp24() -> None:
    assert am.recalled_tuples([(0, 0, 64), (8, 5, 1), (9, 0, 2)]) == [(8, 5, 1), (9, 0, 2)]


def test_eq_table_reads_the_engine_table_and_flags_repeated_steps() -> None:
    table = am.eq_table(TABLES)
    assert len(table) == 128 and table[0] == 200.0 and table[127] == 8000.0
    lossy = am.lossy_indices(table, list(range(128)))
    assert 66 in lossy and 0 not in lossy and len(lossy) >= 10


def test_jsonl_to_csv_writes_one_row_per_program(tmp_path: Path) -> None:
    table = am.eq_table(TABLES)
    rows = [
        {"program": "B", "file": [[8, 0, 80]], "recalled": [[8, 0, 80]], "ok": True},
        {"program": "A", "file": [[9, 1, 2]], "recalled": [], "ok": False},
    ]
    src, out = tmp_path / "r.jsonl", tmp_path / "r.csv"
    src.write_text("\n".join(json.dumps(r) for r in rows), encoding="utf-8")
    assert am.jsonl_to_csv(src, out, table) == 1
    lines = out.read_text(encoding="utf-8").splitlines()
    assert lines[1].startswith("A,9/1/2,,0") and lines[2] == "B,8/0/80,8/0/80,1,80"


def test_committed_recall_csv_matches_every_factory_program() -> None:
    path = ROOT / "docs/research/data/fx-lp24-recall.csv"
    lines = path.read_text(encoding="utf-8").splitlines()[1:]
    assert len(lines) == 26
    assert all(line.split(",")[3] == "1" for line in lines)


def test_read_member_returns_the_archive_bytes(tmp_path: Path) -> None:
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("Program/Bank A/X.ns4p", b"abc")
    path = tmp_path / "x.ns4b"
    path.write_bytes(buf.getvalue())
    assert am.read_member(path, "Program/Bank A/X.ns4p") == b"abc"


def test_sweep_csv_has_every_scenario_and_equal_model_words() -> None:
    rows = sweep.load_rows()
    assert len(rows) == len(sweep.scenarios()) == 54
    assert max(float(r["word_err"]) for r in rows) < 3e-5
    first = rows[0]
    assert (int(first["mode"]), int(first["mid_freq"]), int(first["mid"])) == (0, 60, -60)


def test_band_db_of_a_scaled_copy_is_the_scale() -> None:
    x = [((i * 37) % 17 - 8) / 8.0 for i in range(512)]
    got = sweep.band_db([0.5 * v for v in x], x)
    assert all(abs(g - 20 * math.log10(0.5)) < 1e-9 for g in got)
