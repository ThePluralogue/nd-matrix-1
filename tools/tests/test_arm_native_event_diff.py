"""Unit tests for tools/arm_native_event_diff.py: mismatch classes, ranking, CSV and note."""

from __future__ import annotations

import csv
import json
from pathlib import Path

import arm_native_event_diff as d

CHORD_ON = [0.0, 0.0, 0.0]
CHORD_OFF = [700.0, 700.0, 700.0]


def edges(*pairs: tuple[float, float]) -> list[dict]:
    return [{"t_ms": t, "value": v} for t, v in pairs]


# Dual Gates at 120 BPM (ARM: gate floats 1.0 at 124 ms, 0.0 at 208 ms, 1.0 at 249 ms; chord key-on
# once at 0 ms, key-off once at 700 ms) against a native half-step gate (rise 136.75, fall 199.27).
ARM_DUAL = {
    "program": "Dual Gates",
    "layers": {
        "A": {
            "voice_on_ms": CHORD_ON,
            "voice_off_ms": CHORD_OFF,
            "gate_edges": edges((124, 1), (208, 0), (249, 1), (333, 0), (374, 1)),
        }
    },
}
NAT_DUAL = {
    "program": "Dual Gates.ns4p",
    "layers": {
        "A": {
            "voice_on_ms": CHORD_ON,
            "voice_off_ms": CHORD_OFF,
            "gate_edges": edges((136.75, 1), (199.27, 0), (261.77, 1), (324.27, 0), (386.77, 1)),
        }
    },
}


def layer(**kw: object) -> dict:
    base = {"voice_on_ms": CHORD_ON, "voice_off_ms": CHORD_OFF, "gate_edges": []}
    base.update(kw)
    return base


def classes(arm: dict, nat: dict, words: set[str] | None = None) -> set[str]:
    return {m.cls for m in d.compare_layer("p", "A", arm, nat, words or set())}


def test_names_normalise_to_basename_without_extension() -> None:
    assert d.norm_name("Bank A/Dual Gates.ns4p") == "dual gates"
    assert d.norm_name("C:\\x\\Dual Gates") == "dual gates"


def test_bursts_group_a_chord_into_one_start() -> None:
    assert d.bursts([0.0, 0.4, 1.0, 250.0, 250.2]) == [0.0, 250.0]
    assert d.bursts([]) == []


def test_gate_cycles_give_period_and_high_fraction() -> None:
    periods, duties = d.gate_cycles(ARM_DUAL["layers"]["A"]["gate_edges"], 700.0)
    assert periods == [125, 125]
    assert abs(duties[0] - 84 / 125) < 1e-9


def test_gate_cycles_stop_at_the_key_off() -> None:
    periods, _ = d.gate_cycles(edges((0, 1), (50, 0), (100, 1), (150, 0), (200, 1)), 150.0)
    assert periods == [100]


def test_dual_gates_differs_only_in_gate_duty() -> None:
    found = d.compare_layer(
        "Dual Gates", "A", ARM_DUAL["layers"]["A"], NAT_DUAL["layers"]["A"], set()
    )
    assert [m.cls for m in found] == ["gate_duty"]
    assert found[0].arm == "0.672" and found[0].native == "0.500"


def test_equal_layers_have_no_mismatch() -> None:
    assert classes(layer(), layer()) == set()


def test_retriggered_voices_hit_retrigger_and_count() -> None:
    arm = layer()
    nat = layer(voice_on_ms=[0.0] * 3 + [250.0] * 3)
    assert classes(arm, nat) == {"voice_retrigger", "voice_count"}


def test_fewer_voices_hit_count_only() -> None:
    assert classes(layer(), layer(voice_on_ms=[0.0, 0.0])) == {"voice_count"}


def test_step_period_needs_more_than_five_percent() -> None:
    a = layer(gate_edges=edges((0, 1), (60, 0), (125, 1)))
    ok = layer(gate_edges=edges((0, 1), (60, 0), (130, 1)))
    bad = layer(gate_edges=edges((0, 1), (60, 0), (150, 1)))
    assert "step_period" not in classes(a, ok)
    assert "step_period" in classes(a, bad)


def test_gate_cycles_on_one_side_only_count_as_step_period() -> None:
    a = layer(gate_edges=edges((0, 1), (60, 0), (125, 1)))
    assert classes(a, layer()) == {"step_period"}
    assert d.fmt(None) == "none" and d.fmt(125.02) == "125.0"


def test_key_off_time_tolerance_and_missing_off() -> None:
    assert classes(layer(), layer(voice_off_ms=[710.0] * 3)) == set()
    assert classes(layer(), layer(voice_off_ms=[800.0] * 3)) == {"key_off"}
    assert classes(layer(), layer(voice_off_ms=[])) == {"key_off"}
    assert classes(layer(), layer(voice_off_ms=[700.0, 700.0, 900.0])) == {"key_off"}


def test_dsp_words_without_native_counterpart_are_reported() -> None:
    arm = layer(dsp_words=["gate", "lfo"])
    assert classes(arm, layer()) == {"dsp_word_missing"}
    assert classes(arm, layer(dsp_words=["gate", "lfo"])) == set()
    assert classes(arm, layer(), {"gate", "lfo"}) == set()


def test_inactive_layers_are_skipped() -> None:
    arm = {"program": "x", "layers": {"A": layer(), "B": {}}}
    nat = {"program": "x", "layers": {"A": layer(), "B": {}}}
    found, cover = d.diff({"x": arm}, {"x": nat}, set())
    assert found == [] and cover["compared"] == 1


def test_layers_the_arm_run_did_not_measure_are_skipped() -> None:
    nat = {"program": "x", "layers": {"A": layer(), "B": layer()}}
    arm = {"program": "x", "layers": {"A": layer()}}
    assert d.diff({"x": arm}, {"x": nat}, set())[0] == []


def test_programs_without_a_native_run_are_listed() -> None:
    _, cover = d.diff({"x": ARM_DUAL, "y": ARM_DUAL}, {"x": NAT_DUAL}, set())
    assert cover["arm_only"] == ["y"] and cover["compared"] == 1


def make_mismatch(cls: str, prog: str, mag: float = 1.0) -> d.Mismatch:
    return d.Mismatch(cls, prog, "A", "1", "2", "detail", mag)


def test_ranking_orders_classes_by_programs_affected() -> None:
    found = [
        make_mismatch("gate_duty", "p1"),
        make_mismatch("gate_duty", "p2"),
        make_mismatch("key_off", "p1"),
        make_mismatch("key_off", "p1"),
        make_mismatch("voice_count", "p3"),
    ]
    ranking = d.rank(found)
    assert [r[0] for r in ranking[:3]] == ["gate_duty", "key_off", "voice_count"]
    assert ranking[0][1:] == (2, 2) and ranking[1][1:] == (1, 2)


def test_csv_is_sorted_by_class_rank_then_magnitude(tmp_path: Path) -> None:
    found = [
        make_mismatch("key_off", "p1", 5.0),
        make_mismatch("gate_duty", "p1", 0.2),
        make_mismatch("gate_duty", "p2", 0.4),
    ]
    out = tmp_path / "d.csv"
    d.write_csv(out, found, d.rank(found))
    rows = list(csv.DictReader(out.open(encoding="utf-8")))
    assert [r["class"] for r in rows] == ["gate_duty", "gate_duty", "key_off"]
    assert [r["program"] for r in rows] == ["p2", "p1", "p1"]
    assert rows[0]["rank"] == "1" and rows[0]["class_programs"] == "2"
    assert tuple(rows[0]) == d.CSV_COLUMNS


def test_note_has_the_required_sections_and_sorted_table() -> None:
    found = [make_mismatch("gate_duty", "p1"), make_mismatch("gate_duty", "p2")]
    md = d.render_md(
        d.rank(found), {"compared": 2, "arm_only": []}, found, "2026-11-23", "a.json", "n.json"
    )
    for head in (
        "Checked: 2026-11-23",
        "Sources:",
        "Question:",
        "## Implications",
        "## Follow-ups",
    ):
        assert head in md
    assert md.index("`gate_duty`") < md.index("`voice_count`")
    assert "VERIFIED" in md and "ASSUMED" in md


def test_main_writes_csv_and_note_and_rejects_missing_files(tmp_path: Path) -> None:
    arm = tmp_path / "arm.json"
    nat = tmp_path / "nat.json"
    arm.write_text(json.dumps({"programs": [ARM_DUAL]}), encoding="utf-8")
    nat.write_text(json.dumps({"programs": [NAT_DUAL]}), encoding="utf-8")
    csv_path, md_path = tmp_path / "o.csv", tmp_path / "o.md"
    argv = ["--arm", str(arm), "--native", str(nat), "--csv", str(csv_path), "--md", str(md_path)]
    assert d.main(argv) == 0
    assert "gate_duty" in csv_path.read_text(encoding="utf-8")
    assert md_path.read_text(encoding="utf-8").startswith("# ARM vs native")
    assert d.main(["--arm", str(tmp_path / "none.json"), "--native", str(nat)]) == 1
