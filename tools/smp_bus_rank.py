#!/usr/bin/env python3
"""Rank a piano program from a bus tap file of `sharc-live` (D235).

The ring tap (`smp_tap.f32`) carries the layer A bus `0x9404E` only (D230) and its post stage gate
closes while a note is still held (D235), so a layer B program (Simple Tune, bus `0x9405A`) has no
ring tap. `tools/arm_dsp_live.py --bus-tap 0x9405A --bus-side --log DIR` writes the bus frames to
`DIR/smp_bus.f32` (f32 little endian `L R`, integer bus words x 2^-29). This tool reads that
file and writes one `chain.jsonl` record in the form `tools/program_rank_smp.py` merges into the
ranking CSV:

    python tools/smp_bus_rank.py --bus DIR/smp_bus.f32 --entry "Program/Bank E/Simple Tune.ns4p" \
        --note-frames 5500 --out chain.jsonl
    python tools/program_rank_smp.py --chain chain.jsonl --csv program-ranking.csv

`--note-frames N` keeps the first N frames (the held note, before the key-up voice: the ring rows
are the held note too, their gate closes before the key is released). The peak of the whole file,
release voice included, is reported as `full_peak_dbfs`. Local audio is never committed; only the
record (numbers) is.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
from pathlib import Path

from arm_smp_flash import tap_fundamental, tap_stats


def bus_record(entry: str, data: bytes, note_frames: int | None = None) -> dict:
    """The `chain.jsonl` record of a bus tap: `census.smp` of the held note, plus the full peak."""
    note = data if note_frames is None else data[: 8 * note_frames]
    stats = tap_stats(note)
    cycles = tap_fundamental(note) if stats["kind"] == "audio" else None
    n = len(data) // 4
    full = max(
        (abs(v) for v in struct.unpack(f"<{n}f", data[: 4 * n]) if math.isfinite(v)), default=0.0
    )
    smp = {
        "kind": stats["kind"],
        "peak_dbfs": stats["peak_dbfs"],
        "fundamental_cycles": cycles,
        "frames": len(note) // 8,
        "full_peak_dbfs": 20 * math.log10(full) if full > 0 else None,
    }
    return {"entry": entry, "census": {"smp": smp}}


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    ap.add_argument("--bus", type=Path, required=True, help="smp_bus.f32 of the run")
    ap.add_argument("--entry", required=True, help="ranking CSV `entry` of the program")
    ap.add_argument("--note-frames", type=int, help="frames of the held note (default: all)")
    ap.add_argument("--out", type=Path, help="append the record to this chain.jsonl")
    ns = ap.parse_args(argv)
    line = json.dumps(bus_record(ns.entry, ns.bus.read_bytes(), ns.note_frames))
    if ns.out:
        with ns.out.open("a", encoding="utf-8", newline="\n") as fh:
            fh.write(line + "\n")
    print(line)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
