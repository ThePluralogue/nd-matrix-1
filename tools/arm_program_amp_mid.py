#!/usr/bin/env python3
"""Amp LP24 / HP24 knob mapping through the booted firmware's own program recall (D234).

The Rust port (`fx/amp_eq.rs::process_fw_lp24`) turns the decoded file fields into the two ARM
indices it feeds the routine tables with: `kAmpMid` counts `-60..60` (from `eq_mid_db / 0.25`
through the 0..10 resonance knob) and the `kAmpMidFreq` index `0..127` (from the Hz of the table
step). This tool checks the file side against the firmware: it puts a factory `.ns4p` into the
Store record (`arm_program_load.build_image`), boots the full machine so the firmware recalls the
program itself, then reads the three recalled cells `kAmpTypeNew` (0x1A7), `kAmpMid` (0x1A0) and
`kAmpMidFreq` (0x1A2) of every FX unit and compares them with the decoder values of `ns4 inspect
--json` (every FX block with an enabled Amp of type 8 or 9).

    uv run --with unicorn python tools/arm_program_amp_mid.py <CODE> <SRAM> \
        --eeprom target/oled/eeprom.bin --ns4b "<backup>.ns4b" --ns4 target/release/ns4.exe \
        --program "Program/Bank F/Unclocked Pad.ns4p" --jsonl recall.jsonl
    python tools/arm_program_amp_mid.py --to-csv recall.jsonl docs/research/data/fx-lp24-recall.csv

Reads the local firmware only; the output is derived numbers (never commit the firmware).
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import re
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

#: VERIFIED (`docs/research/arm-edit-params.csv`): the three Amp edit ids.
EDIT_IDS = {"type": 0x1A7, "mid": 0x1A0, "freq": 0x1A2}
#: `kAmpTypeNew` values of LP24 / HP24 (`FxDecoded::amp_mode`, low nibble of the file field).
FILTER_TYPES = (8, 9)
#: Count step of the file's Mid field: `eq_mid_db = count * 0.25` (`EQ_GAIN_DB_PER_COUNT`).
DB_PER_COUNT = 0.25

Tuple3 = tuple[int, int, int]


def signed32(word: int) -> int:
    """Two's complement of a 32-bit cell word."""
    return word - (1 << 32) if word & 0x8000_0000 else word


def fx_blocks(node: object) -> list[dict]:
    """Every dict of a decoded program that is an FX block (has `amp_type_new`)."""
    found: list[dict] = []
    if isinstance(node, dict):
        if "amp_type_new" in node and "eq_mid_db" in node:
            found.append(node)
        for value in node.values():
            found += fx_blocks(value)
    elif isinstance(node, list):
        for value in node:
            found += fx_blocks(value)
    return found


def expected_tuples(decoded: dict) -> list[Tuple3]:
    """`(type, kAmpMid counts, kAmpMidFreq index)` of every enabled LP24 / HP24 Amp in the file."""
    out = []
    for block in fx_blocks(decoded):
        kind = int(block["amp_type_new"]) & 0x0F
        if block.get("amp_enabled") and kind in FILTER_TYPES:
            counts = round(float(block["eq_mid_db"]) / DB_PER_COUNT)
            index = round(float(block["eq_mid_freq_norm"]) * 127.0)
            out.append((kind, counts, index))
    return out


def recalled_tuples(cells: list[Tuple3]) -> list[Tuple3]:
    """Recalled `(type, mid, freq)` cells of the units whose Amp is an LP24 / HP24."""
    return [c for c in cells if c[0] in FILTER_TYPES]


def compare(expected: list[Tuple3], recalled: list[Tuple3]) -> list[str]:
    """Mismatch lines: a file tuple the firmware did not recall, or none recalled at all."""
    problems = [
        f"file {e} not among the recalled {sorted(set(recalled))}"
        for e in expected
        if e not in recalled
    ]
    if expected and not recalled:
        problems.append("no recalled LP24 / HP24 unit")
    return problems


def eq_table(source: Path) -> list[float]:
    """`EQ_FREQ_HZ` (128 display steps) parsed from `fx/tables.rs`."""
    text = source.read_text(encoding="utf-8")
    body = text.split("pub const EQ_FREQ_HZ: [f32; 128] = [", 1)[1].split("];", 1)[0]
    values = [float(v) for v in re.findall(r"\d+\.\d+", body)]
    if len(values) != 128:
        raise ValueError(f"EQ_FREQ_HZ has {len(values)} values")
    return values


def lossy_indices(table: list[float], indices: list[int]) -> list[int]:
    """Indices that the Hz-only path (nearest display step in log Hz, first on a tie) gets wrong."""

    def nearest(i: int) -> int:
        target = math.log(table[i])
        return min(range(128), key=lambda j: (abs(math.log(table[j]) - target), j))

    return [i for i in indices if nearest(i) != i]


def fmt(tuples: list[Tuple3]) -> str:
    """`type/mid/freq;...` of the distinct tuples."""
    return ";".join("%d/%d/%d" % t for t in sorted(set(tuples)))


def jsonl_to_csv(jsonl: Path, out: Path, table: list[float]) -> int:
    """Rewrite the per program JSON lines as the committed CSV; returns the number of failures."""
    rows = sorted(
        (json.loads(line) for line in jsonl.read_text(encoding="utf-8").splitlines() if line),
        key=lambda r: r["program"],
    )
    with out.open("w", encoding="utf-8", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(
            ["program", "file_type_mid_freq", "recalled_type_mid_freq", "ok", "lossy_hz_indices"]
        )
        for r in rows:
            recalled = [tuple(t) for t in r["recalled"]]
            lossy = lossy_indices(table, sorted({t[2] for t in recalled}))
            w.writerow(
                [
                    r["program"],
                    fmt([tuple(t) for t in r["file"]]),
                    fmt(recalled),
                    int(r["ok"]),
                    ";".join(map(str, lossy)),
                ]
            )
    return sum(1 for r in rows if not r["ok"])


def read_member(ns4b: Path, member: str) -> bytes:
    """Bytes of an archive member."""
    with zipfile.ZipFile(ns4b) as archive, archive.open(member) as handle:
        return handle.read()


def decode(ns4: Path, program: bytes) -> dict:
    """`ns4 inspect --json` of a program blob."""
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "p.ns4p"
        path.write_bytes(program)
        run = subprocess.run(
            [str(ns4), "inspect", str(path), "--json"], capture_output=True, text=True, check=True
        )
    return json.loads(run.stdout)


def main(argv: list[str] | None = None) -> int:  # pragma: no cover - needs the local firmware
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    ap.add_argument("image", type=Path, nargs="?")
    ap.add_argument("sram", type=Path, nargs="?")
    ap.add_argument("--eeprom", type=Path)
    ap.add_argument("--ns4b", type=Path)
    ap.add_argument("--ns4", type=Path)
    ap.add_argument("--program", action="append", help="archive member of a .ns4p")
    ap.add_argument(
        "--to-csv", nargs=2, type=Path, metavar=("JSONL", "CSV"), help="convert results"
    )
    ap.add_argument("--ticks", type=int, default=300)
    ap.add_argument("--jsonl", type=Path, help="append one result line per program")
    ns = ap.parse_args(argv)
    here = Path(__file__).resolve().parent
    if ns.to_csv:
        table = eq_table(here.parent / "crates/nord-core/src/fx/tables.rs")
        return int(jsonl_to_csv(ns.to_csv[0], ns.to_csv[1], table) > 0)
    sys.path.insert(0, str(here))
    import arm_edit_capture as aec
    import arm_program_load as pg
    import arm_screens as sc
    import fx_clients as fc

    params = aec.load_params(Path("docs/research/arm-edit-params.csv"), ".")
    ids = {p.name: p.edit_id for p in params}
    markers = {n: ids[n] for n in fc.KIND_MARKERS if n in ids}
    bad = 0
    for member in ns.program:
        program = read_member(ns.ns4b, member)
        want = expected_tuples(decode(ns.ns4, program))
        session = sc.Session(ns.image, ns.sram)
        session.load_eeproms(
            pg.build_image(ns.eeprom.read_bytes(), program, Path(member).stem[:23])
        )
        session.boot()
        session.run(ns.ticks)
        cells = []
        for unit in fc.find_units(session.emu, markers):
            words = []
            for key in ("type", "mid", "freq"):
                cell = aec.value_cell(session.emu, unit.address, EDIT_IDS[key])
                words.append(signed32(session.emu.read32(cell)) if cell else -1)
            cells.append((words[0], words[1], words[2]))
        got = recalled_tuples(cells)
        problems = compare(want, got)
        verdict = problems or "OK"
        print(f"{member}: file {want} recalled {sorted(set(got))} {verdict}")
        if ns.jsonl:
            with ns.jsonl.open("a", encoding="utf-8") as fh:
                fh.write(
                    json.dumps(
                        {
                            "program": member,
                            "file": want,
                            "recalled": sorted(set(got)),
                            "ok": not problems,
                        }
                    )
                    + "\n"
                )
        bad += bool(problems)
    return int(bad > 0)


if __name__ == "__main__":
    sys.exit(main())
