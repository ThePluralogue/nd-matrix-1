"""Tests of `smp_bus_rank`: a bus tap file to the `program_rank_smp` chain record (D233)."""

import json
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import program_rank_smp as ps  # noqa: E402
import smp_bus_rank as sb  # noqa: E402


def tone(frames: int, period: float, amp: float, start: int = 0) -> bytes:
    """`L R` f32 frames of a harmonic tone from frame `start` (silence before)."""
    out = bytearray()
    for i in range(frames):
        t = i - start
        v = 0.0
        if t >= 0:
            v = amp * sum(math.sin(2 * math.pi * h * t / period) / h for h in (1, 2, 3))
        out += struct.pack("<ff", v, v)
    return bytes(out)


def test_a_tone_ranks_as_audio_with_its_pitch_and_peak() -> None:
    rec = sb.bus_record("Program/Bank E/X.ns4p", tone(6000, 83.0, 0.02))
    smp = rec["census"]["smp"]
    assert smp["kind"] == "audio" and smp["frames"] == 6000
    assert abs(smp["fundamental_cycles"] - 1 / 83.0) < 0.0003
    assert -34 < smp["peak_dbfs"] < -26


def test_the_note_window_excludes_a_louder_release() -> None:
    held, release = tone(4000, 83.0, 0.02), tone(1000, 83.0, 0.1)
    rec = sb.bus_record("e", held + release, note_frames=4000)["census"]["smp"]
    assert rec["peak_dbfs"] < rec["full_peak_dbfs"] - 10, rec
    assert rec["frames"] == 4000


def test_silence_is_no_audio_and_the_record_merges_into_a_row() -> None:
    silent = sb.bus_record("a", bytes(8 * 5000))
    assert silent["census"]["smp"]["kind"] == "silent"
    assert silent["census"]["smp"]["fundamental_cycles"] is None
    audio = sb.bus_record("a", tone(6000, 83.0, 0.02))
    row = {
        "entry": "a",
        "engines": "piano+synth",
        "smp_peak_dbfs": "",
        "smp_cycles": "",
        "flags": "piano_chain_no_audio",
    }
    out = ps.merge([row], [audio])[0]
    assert out["flags"] == "" and out["smp_peak_dbfs"] and out["smp_cycles"]


def test_main_appends_one_json_line(tmp_path: Path) -> None:
    bus = tmp_path / "smp_bus.f32"
    bus.write_bytes(tone(5000, 83.0, 0.02))
    out = tmp_path / "chain.jsonl"
    assert sb.main(["--bus", str(bus), "--entry", "e", "--out", str(out)]) == 0
    assert sb.main(["--bus", str(bus), "--entry", "f", "--out", str(out)]) == 0
    lines = [json.loads(x) for x in out.read_text("utf-8").splitlines()]
    assert [r["entry"] for r in lines] == ["e", "f"]
