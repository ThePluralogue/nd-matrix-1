#!/usr/bin/env python3
"""Envelope pass period of the ORG core 2 chains, fitted on all 128 knobs (D237).

A pass is the control step of the amp / filter / osc ADR chains (one record step, D210 / D212). Its
length in DSP frames `P` follows from two firmware tables that need no oracle run:

* the displayed attack time (`ENV_ATTACK_MS_TABLE`, firmware strings at `0x53abe4`) is the linear
  ramp of `1 / b` passes, `b` = settled attack coefficient of the index `knob - 64`;
* the displayed decay / release time (`ENV_DECAY_MS_TABLE`, `0x53ade4`) is the time to fall 40 dB,
  `ln(100) / (1 - a)` passes, `a` = settled decay coefficient.

`P = display_s * F / passes` with `F = 44,444,000 / 1024` Hz (D211). With the settled tables
(`synth-env-rates-settled.csv`) the fit is 2.000 frames at every knob inside the display rounding;
with the stale 3 pass sweep it holds only on the first index of each plateau (D212).

    uv run python tools/synth_pass_period.py fit
    uv run python tools/synth_pass_period.py check
"""

from __future__ import annotations

import argparse
import csv
import math
import re
import statistics
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PRESETS = ROOT / "crates/nord-core/src/cbin/presets.rs"
SETTLED = ROOT / "docs/research/data/synth-env-rates-settled.csv"
STALE = ROOT / "docs/research/data/synth-env-rate-table.csv"

# VERIFIED (D211): DSP frame rate, five independent readouts.
FRAME_RATE_HZ = 44_444_000 / 1024
# VERIFIED (this tool): frames per envelope pass.
PASS_FRAMES = 2.0
# Display strings carry 2 to 3 significant digits (0.5 ms steps near 0.5 ms), so a single knob is
# only known to about 8 %; the median over 128 knobs is the period measurement.
KNOB_TOLERANCE = 0.08
MEDIAN_TOLERANCE = 0.005


def display_table(name: str, text: str | None = None) -> list[float]:
    """The 128 display times (ms) of the firmware table `name` in `presets.rs`."""
    src = text if text is not None else PRESETS.read_text(encoding="utf-8")
    m = re.search(r"pub const " + name + r": \[f32; 128\] = \[(.*?)\];", src, re.S)
    if m is None:
        raise ValueError(f"table {name} not found")
    vals = [float(x) for x in re.findall(r"\d+\.?\d*", m.group(1))]
    if len(vals) != 128:
        raise ValueError(f"{name}: {len(vals)} values")
    return vals


def read_rows(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8") as f:
        return list(csv.DictReader(f))


def passes_attack(b: float) -> float:
    """Passes of the linear attack ramp to full scale."""
    return 1.0 / b


def passes_decay(a: float) -> float:
    """Passes for a one pole decay `y <- a y` to fall 40 dB."""
    return math.log(100.0) / (1.0 - a)


def frames_per_pass(display_ms: float, passes: float) -> float:
    """`P` that makes `passes` passes last the displayed time."""
    return display_ms * 1e-3 * FRAME_RATE_HZ / passes


def fit(attack_coeffs: list[float], decay_coeffs: list[float]) -> dict[str, list[float]]:
    """Per knob frames per pass for attack and decay (knob `k` is index `k - 64`)."""
    att = display_table("ENV_ATTACK_MS_TABLE")
    dec = display_table("ENV_DECAY_MS_TABLE")
    return {
        "attack": [frames_per_pass(att[k], passes_attack(attack_coeffs[k])) for k in range(128)],
        "decay": [frames_per_pass(dec[k], passes_decay(decay_coeffs[k])) for k in range(128)],
    }


def coeffs(path: Path, attack_key: str, decay_key: str) -> tuple[list[float], list[float]]:
    rows = read_rows(path)[:128]  # indexes -64..63 are knobs 0..127
    return [float(r[attack_key]) for r in rows], [float(r[decay_key]) for r in rows]


def summary(p: dict[str, list[float]]) -> dict[str, tuple[float, float, float]]:
    return {k: (statistics.median(v), min(v), max(v)) for k, v in p.items()}


def run_fit() -> dict[str, dict[str, tuple[float, float, float]]]:
    settled = fit(*coeffs(SETTLED, "attack_coeff", "decay_coeff"))
    stale = fit(*coeffs(STALE, "attack_coeff", "decay_release_coeff"))
    return {"settled": summary(settled), "stale": summary(stale)}


def check(s: dict[str, dict[str, tuple[float, float, float]]]) -> list[str]:
    """Failures of the settled fit against `PASS_FRAMES` (empty when it holds)."""
    bad = []
    for kind, (med, lo, hi) in s["settled"].items():
        if abs(med / PASS_FRAMES - 1) > MEDIAN_TOLERANCE:
            bad.append(f"{kind} median {med:.4f}")
        if max(abs(lo / PASS_FRAMES - 1), abs(hi / PASS_FRAMES - 1)) > KNOB_TOLERANCE:
            bad.append(f"{kind} range {lo:.3f}..{hi:.3f}")
    return bad


TABLE_FIELDS = (
    "index",
    "knob",
    "attack_display_ms",
    "decay_display_ms",
    "attack_frames_settled",
    "attack_frames_legacy",
    "decay_frames_settled",
    "decay_frames_legacy",
    "attack_legacy_over_settled",
    "decay_rate_legacy_over_settled",
)


def per_index_csv() -> str:
    """Per knob (index -64..63): display times, the implied pass length for both tables, and the
    legacy / settled ratios of the attack coefficient and of the decay rate `1 - a`."""
    sa, sd = coeffs(SETTLED, "attack_coeff", "decay_coeff")
    la, ld = coeffs(STALE, "attack_coeff", "decay_release_coeff")
    s, o = fit(sa, sd), fit(la, ld)
    att, dec = display_table("ENV_ATTACK_MS_TABLE"), display_table("ENV_DECAY_MS_TABLE")
    lines = [",".join(TABLE_FIELDS)]
    for k in range(128):
        vals = (
            k - 64,
            k,
            att[k],
            dec[k],
            round(s["attack"][k], 4),
            round(o["attack"][k], 4),
            round(s["decay"][k], 4),
            round(o["decay"][k], 4),
            round(la[k] / sa[k], 4),
            round((1 - ld[k]) / (1 - sd[k]), 4),
        )
        lines.append(",".join(str(v) for v in vals))
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("cmd", choices=("fit", "check", "table"))
    ap.add_argument("--out", help="table: write the per index CSV here (default: stdout)")
    args = ap.parse_args(argv)
    if args.cmd == "table":
        text = per_index_csv()
        if args.out:
            Path(args.out).write_text(text, encoding="utf-8", newline="")
        else:
            sys.stdout.write(text)
        return 0
    s = run_fit()
    for tables, rows in s.items():
        for kind, (med, lo, hi) in rows.items():
            print(
                f"{tables:8s} {kind:7s} frames/pass: median {med:.4f}  min {lo:.3f}  max {hi:.3f}"
            )
    if args.cmd == "check":
        bad = check(s)
        if bad:
            print("FAIL: " + "; ".join(bad), file=sys.stderr)
            return 1
        print(f"OK: settled tables fit {PASS_FRAMES} frames per pass on all 128 knobs")
    return 0


if __name__ == "__main__":
    sys.exit(main())
