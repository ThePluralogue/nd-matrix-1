#!/usr/bin/env python3
"""Diff of the ARM firmware's derived DSP events against the native engine, per program and layer.

Inputs (both: `{"programs": [{program, layers: {A|B|C: {voice_on_ms, voice_off_ms, gate_edges,
dsp_words?}}}]}` or a bare list of such program objects):

* ARM side: `docs/research/data/arm-dsp-events.json` (`tools/arm_dsp_event_sweep.py`).
* Native side: `cargo run --release -p nord-tools --bin native_event_trace -- <backup.ns4b>
  --out f.json`.

Mismatch classes (each counted once per program and layer, ranked by programs affected):

| class            | mismatch when                                                               |
|------------------|-----------------------------------------------------------------------------|
| voice_retrigger  | number of voice note-on bursts (note-ons within 5 ms) differs                |
| voice_count      | total number of voice note-ons differs                                       |
| step_period      | median rising-edge to rising-edge gate period differs by more than 5 %      |
| gate_duty        | median high fraction of a gate period differs by more than 0.10             |
| key_off          | first voice note-off differs by more than 25 ms, or off-burst count differs |
| dsp_word_missing | the ARM layer writes a DSP word the native side has no counterpart for      |

Writes the ranked CSV and, with `--md`, the research note. Derived numbers only.

    python tools/arm_native_event_diff.py --arm docs/research/data/arm-dsp-events.json \
        --native target/native-events.json --md docs/research/arm-native-event-diff.md
"""

from __future__ import annotations

import argparse
import csv
import json
import statistics
import sys
from dataclasses import dataclass
from pathlib import Path

BURST_MS = 5.0  # ASSUMED: note-ons of one chord land within one ARM tick (about 1 ms); 5 ms margin
PERIOD_TOL = 0.05  # ASSUMED: 5 % of a step; the ARM tick jitter is below 1 %
DUTY_TOL = 0.10  # ASSUMED: one eighth of a step is 0.125, so 0.10 flags a one-eighth difference
KEY_OFF_TOL_MS = 25.0  # ASSUMED: one ARM event poll interval is below this
CLASSES = (
    "voice_retrigger",
    "voice_count",
    "step_period",
    "gate_duty",
    "key_off",
    "dsp_word_missing",
)
CSV_COLUMNS = (
    "rank",
    "class",
    "class_programs",
    "program",
    "layer",
    "arm_value",
    "native_value",
    "detail",
)
TAGS = {
    "voice_retrigger": "VERIFIED (event counts)",
    "voice_count": "VERIFIED (event counts)",
    "step_period": "EMPIRICAL (5 % tolerance ASSUMED)",
    "gate_duty": "EMPIRICAL (0.10 tolerance ASSUMED)",
    "key_off": "EMPIRICAL (25 ms tolerance ASSUMED)",
    "dsp_word_missing": "VERIFIED (word list)",
}


@dataclass(frozen=True)
class Mismatch:
    cls: str
    program: str
    layer: str
    arm: str
    native: str
    detail: str
    magnitude: float


def norm_name(name: str) -> str:
    """Basename without `.ns4p`, lower case."""
    base = name.replace("\\", "/").rsplit("/", 1)[-1]
    return base.removesuffix(".ns4p").strip().lower()


def load_programs(path: Path) -> dict[str, dict]:
    """Programs of a JSON file keyed by normalised name; entries without layers are skipped."""
    data = json.loads(path.read_text(encoding="utf-8"))
    items = data["programs"] if isinstance(data, dict) else data
    return {norm_name(p["program"]): p for p in items if "layers" in p}


def bursts(times: list[float]) -> list[float]:
    """Start time of every group of events closer than `BURST_MS` to the previous one."""
    out: list[float] = []
    last = 0.0
    for t in sorted(times):
        if not out or t - last > BURST_MS:
            out.append(t)
        last = t
    return out


def gate_cycles(edges: list[dict], until: float | None) -> tuple[list[float], list[float]]:
    """(rise-to-rise periods, high fractions) of the gate edges before `until` ms."""
    ev = sorted((e["t_ms"], 1 if e["value"] >= 0.5 else 0) for e in edges)
    if until is not None:
        ev = [e for e in ev if e[0] < until]
    rises = [t for t, v in ev if v == 1]
    periods: list[float] = []
    duties: list[float] = []
    for a, b in zip(rises, rises[1:]):
        falls = [t for t, v in ev if v == 0 and a < t < b]
        periods.append(b - a)
        if falls:
            duties.append((falls[0] - a) / (b - a))
    return periods, duties


def fmt(x: float | None) -> str:
    return "none" if x is None else f"{x:.1f}"


def median(xs: list[float]) -> float | None:
    return statistics.median(xs) if xs else None


def active(layer: dict) -> bool:
    return bool(layer.get("voice_on_ms") or layer.get("gate_edges") or layer.get("dsp_words"))


def compare_layer(
    program: str, name: str, arm: dict, nat: dict, native_words: set[str]
) -> list[Mismatch]:
    """Mismatches of one layer."""
    out: list[Mismatch] = []

    def add(cls: str, a: object, n: object, detail: str, mag: float) -> None:
        out.append(Mismatch(cls, program, name, str(a), str(n), detail, mag))

    a_on, n_on = arm.get("voice_on_ms", []), nat.get("voice_on_ms", [])
    a_b, n_b = bursts(a_on), bursts(n_on)
    if len(a_b) != len(n_b):
        add("voice_retrigger", len(a_b), len(n_b), "note-on bursts", abs(len(a_b) - len(n_b)))
    if len(a_on) != len(n_on):
        add("voice_count", len(a_on), len(n_on), "note-ons", abs(len(a_on) - len(n_on)))
    a_off, n_off = arm.get("voice_off_ms", []), nat.get("voice_off_ms", [])
    a_first = min(a_off) if a_off else None
    n_first = min(n_off) if n_off else None
    if (a_first is None) != (n_first is None):
        add("key_off", a_first, n_first, "note-off on one side only", 1.0)
    elif a_first is not None and n_first is not None:
        if abs(a_first - n_first) > KEY_OFF_TOL_MS:
            add(
                "key_off",
                f"{a_first:.1f}",
                f"{n_first:.1f}",
                "first note-off ms",
                abs(a_first - n_first),
            )
        elif len(bursts(a_off)) != len(bursts(n_off)):
            add("key_off", len(bursts(a_off)), len(bursts(n_off)), "note-off bursts", 1.0)
    a_p, a_d = gate_cycles(arm.get("gate_edges", []), a_first)
    n_p, n_d = gate_cycles(nat.get("gate_edges", []), n_first)
    pa, pn = median(a_p), median(n_p)
    if pa is not None and pn is not None:
        if abs(pa - pn) > PERIOD_TOL * pa:
            add("step_period", f"{pa:.1f}", f"{pn:.1f}", "gate period ms", abs(pa - pn) / pa)
    elif pa is not None or pn is not None:
        add("step_period", fmt(pa), fmt(pn), "gate cycles on one side only", 1.0)
    da, dn = median(a_d), median(n_d)
    if da is not None and dn is not None and abs(da - dn) > DUTY_TOL:
        add("gate_duty", f"{da:.3f}", f"{dn:.3f}", "high fraction of a period", abs(da - dn))
    missing = sorted(set(arm.get("dsp_words", [])) - native_words - set(nat.get("dsp_words", [])))
    if missing:
        add(
            "dsp_word_missing",
            ";".join(missing),
            "",
            f"{len(missing)} word(s)",
            float(len(missing)),
        )
    return out


def diff(
    arm: dict[str, dict], native: dict[str, dict], native_words: set[str]
) -> tuple[list[Mismatch], dict]:
    """All mismatches plus coverage (`compared`, `arm_only`, `native_only`)."""
    found: list[Mismatch] = []
    compared = 0
    for key, a in sorted(arm.items()):
        n = native.get(key)
        if n is None:
            continue
        compared += 1
        for layer in ("A", "B", "C"):
            if layer not in a["layers"]:
                continue  # the ARM run did not measure this layer
            al, nl = a["layers"][layer], n["layers"].get(layer, {})
            if active(al) or active(nl):
                found += compare_layer(a["program"], layer, al, nl, native_words)
    cover = {
        "compared": compared,
        "arm_only": sorted(set(arm) - set(native)),
        "native_only": len(set(native) - set(arm)),
    }
    return found, cover


def rank(found: list[Mismatch]) -> list[tuple[str, int, int]]:
    """`(class, programs affected, layers affected)` sorted by programs, then layers."""
    rows = []
    for cls in CLASSES:
        hits = [m for m in found if m.cls == cls]
        rows.append((cls, len({m.program for m in hits}), len(hits)))
    return sorted(rows, key=lambda r: (-r[1], -r[2], CLASSES.index(r[0])))


def write_csv(path: Path, found: list[Mismatch], ranking: list[tuple[str, int, int]]) -> None:
    """Mismatch rows ordered by class rank, then magnitude."""
    order = {c: i + 1 for i, (c, _, _) in enumerate(ranking)}
    progs = {c: p for c, p, _ in ranking}
    rows = sorted(found, key=lambda m: (order[m.cls], -m.magnitude, m.program, m.layer))
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(CSV_COLUMNS)
        for m in rows:
            w.writerow(
                [order[m.cls], m.cls, progs[m.cls], m.program, m.layer, m.arm, m.native, m.detail]
            )


def render_md(
    ranking: list[tuple[str, int, int]],
    cover: dict,
    found: list[Mismatch],
    checked: str,
    arm_src: str,
    native_src: str,
    note: str = "",
) -> str:
    """The research note (Checked, Sources, Question, findings, Implications, Follow-ups)."""
    lines = [
        "# ARM vs native DSP event diff",
        "",
        f"Checked: {checked}",
        "",
        "Sources:",
        f"- ARM events: `{arm_src}` (derived DSP events of the firmware).",
        f"- Native events: `{native_src}` (`native_event_trace` bin of `nord-tools`).",
        "- Diff: `tools/arm_native_event_diff.py`; ranked rows in "
        "`docs/research/data/arm-native-event-diff.csv`.",
        "",
        "Question: which factory programs make the firmware send different note / gate events than "
        "the native engine for the same keys (60, 64, 67, velocity 100, held 1.0 s, released at "
        "0.7 s, 120 BPM)?",
        "",
        f"Programs compared: {cover['compared']} "
        f"(ARM programs without a native run: {len(cover['arm_only'])}).",
    ]
    if note:
        lines += ["", note]
    lines += [
        "",
        "## Classes by programs affected",
        "",
        "| Rank | Class | Programs | Layers | Tag |",
        "|---|---|---|---|---|",
    ]
    for i, (cls, p, n) in enumerate(ranking, 1):
        lines.append(f"| {i} | `{cls}` | {p} | {n} | {TAGS[cls]} |")
    lines += [
        "",
        "## Largest mismatch per class",
        "",
        "| Class | Program | Layer | ARM | Native | Detail |",
        "|---|---|---|---|---|---|",
    ]
    for cls, p, _ in ranking:
        if p:
            m = max((x for x in found if x.cls == cls), key=lambda x: x.magnitude)
            lines.append(
                f"| `{cls}` | {m.program} | {m.layer} | {m.arm} | {m.native} | {m.detail} |"
            )
    lines += [
        "",
        "## Implications",
        "",
        "- Classes at the top affect the most programs; fix them first, one PR per class.",
        "- Tolerances are tooling choices (`ASSUMED`); counts are exact.",
        "- Rows come from derived events only, so a class can hide another cause (for example a "
        "`voice_count` hit from a key split).",
        "",
        "## Follow-ups",
        "",
        "- Re-run on the full ARM sweep (`docs/research/data/arm-dsp-events.json`) when it lands.",
        "- Add a native counterpart per `dsp_word_missing` word, or document why none is needed.",
        "",
    ]
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="Diff ARM DSP events against native events.")
    ap.add_argument("--arm", type=Path, default=Path("docs/research/data/arm-dsp-events.json"))
    ap.add_argument("--native", type=Path, required=True)
    ap.add_argument(
        "--csv", type=Path, default=Path("docs/research/data/arm-native-event-diff.csv")
    )
    ap.add_argument("--md", type=Path, default=None, help="write the research note here")
    ap.add_argument("--checked", default="", help="date line of the note")
    ap.add_argument("--note", default="", help="extra paragraph for the note (scope caveats)")
    ap.add_argument("--native-words", type=Path, default=None, help="one native DSP word per line")
    args = ap.parse_args(argv)
    for p in (args.arm, args.native):
        if not p.exists():
            print(f"error: {p} not found", file=sys.stderr)
            return 1
    words = (
        set(args.native_words.read_text(encoding="utf-8").split()) if args.native_words else set()
    )
    found, cover = diff(load_programs(args.arm), load_programs(args.native), words)
    ranking = rank(found)
    write_csv(args.csv, found, ranking)
    if args.md:
        args.md.write_text(
            render_md(
                ranking,
                cover,
                found,
                args.checked,
                args.arm.as_posix(),
                args.native.name,
                args.note,
            ),
            encoding="utf-8",
        )
    for cls, p, n in ranking:
        print(f"{cls:18} programs={p:4} layers={n:4}")
    print(f"compared {cover['compared']} programs")
    return 0


if __name__ == "__main__":
    sys.exit(main())
