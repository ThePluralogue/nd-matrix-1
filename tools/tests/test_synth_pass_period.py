"""Tests for `tools/synth_pass_period.py` (D237)."""

import math

import synth_pass_period as P


def test_display_tables_have_128_firmware_steps() -> None:
    att = P.display_table("ENV_ATTACK_MS_TABLE")
    dec = P.display_table("ENV_DECAY_MS_TABLE")
    assert (att[0], att[127]) == (0.5, 45000.0)
    assert (dec[0], dec[127]) == (3.0, 45000.0)
    with __import__("pytest").raises(ValueError):
        P.display_table("NO_SUCH_TABLE")


def test_frames_per_pass_inverts_the_pass_time() -> None:
    # 10 passes of 2 frames last 20 frames.
    ms = 20 / P.FRAME_RATE_HZ * 1e3
    assert math.isclose(P.frames_per_pass(ms, 10.0), 2.0)
    assert math.isclose(P.passes_attack(0.1), 10.0)
    assert math.isclose(P.passes_decay(0.99), math.log(100) / 0.01)


def test_settled_tables_give_two_frames_per_pass_on_every_knob() -> None:
    s = P.run_fit()
    assert P.check(s) == []
    for med, lo, hi in s["settled"].values():
        assert abs(med - 2.0) < 0.01
        assert 1.85 < lo and hi < 2.15


def test_stale_sweep_does_not_fit_a_single_period() -> None:
    s = P.run_fit()
    for med, _lo, hi in s["stale"].values():
        assert med > 2.2 and hi > 3.5  # plateau interiors read 2.5 to 5 frames
    assert P.check({"settled": s["stale"]})  # the check rejects it


def test_main_check_succeeds() -> None:
    assert P.main(["check"]) == 0


def test_per_index_table_has_one_row_per_knob_and_ratios_at_least_one() -> None:
    rows = P.per_index_csv().strip().splitlines()
    assert rows[0].split(",") == list(P.TABLE_FIELDS)
    assert len(rows) == 129
    first = rows[1].split(",")
    assert first[:2] == ["-64", "0"]
    # The stale sweep is the settled value at plateau anchors and never slower than it.
    assert float(first[8]) == 1.0
    ratios = [float(r.split(",")[8]) for r in rows[1:]]
    assert min(ratios) >= 0.999 and max(ratios) > 2.0
    dec = [float(r.split(",")[9]) for r in rows[1:]]
    assert min(dec) >= 0.999 and 1.9 < max(dec) < 2.1


def test_table_command_writes_the_csv(tmp_path) -> None:
    out = tmp_path / "t.csv"
    assert P.main(["table", "--out", str(out)]) == 0
    assert out.read_text(encoding="utf-8") == P.per_index_csv()
