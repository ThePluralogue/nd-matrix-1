#!/usr/bin/env python3
"""Per Mid-position response of the Amp LP24 / HP24 routine on the oracle (D234).

`fx_lp24_capture.py golden` proves the Rust stage word for word on 6 scenarios. This tool sweeps
the knob: LP24 and HP24, 9 Mid positions (`kAmpMid` -60 .. 60, step 15) at 3 cutoffs
(`kAmpMidFreq` 60 / 90 / 110), each over 128 calls (512 stereo frames, seeded LCG noise, amplitude
0.4) of the real template with **settled** smoothers (the steady response, not the glide). For
each scenario it writes the oracle output power gain in 12 bands (bins of a 512-point DFT of the
left channel) and the worst word difference of the stateful model against the oracle. The Rust
test `tests_fw_lp24_sweep.rs` runs the same input through the port and compares the bands.

    python tools/fx_lp24_mid_sweep.py run --image C:/tmp/h/img_0_FX/core2_sram.bin
    python tools/fx_lp24_mid_sweep.py check   # CSV shape and the model against the stored bands

Output: `docs/research/data/fx-lp24-mid-sweep.csv`.
"""

from __future__ import annotations

import argparse
import csv
import math
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import fx_lp24_capture as cap  # noqa: E402
import fx_model_lp24 as lp  # noqa: E402

CSV = HERE.parent / "docs/research/data/fx-lp24-mid-sweep.csv"
MODES = (0, 1)
FREQS = (60, 90, 110)
MIDS = tuple(range(-60, 61, 15))
CALLS = 128
SEED = 7000
AMP = 0.4
#: Band edges in DFT bins of a `CALLS * 4` = 512 frame block (bin `k` is `k / 512` of the rate).
EDGES = (1, 2, 4, 6, 9, 14, 21, 32, 48, 72, 108, 160, 256)
#: Tolerance of the band comparison, dB (the port equals the oracle to ~1e-5 on words).
TOLERANCE_DB = 0.01


def band_db(out: list[float], inp: list[float]) -> list[float]:
    """Power gain `dB` of `out` over `inp` (left channel) in the 12 bands."""
    x = np.abs(np.fft.rfft(np.asarray(inp, dtype=np.float64))) ** 2
    y = np.abs(np.fft.rfft(np.asarray(out, dtype=np.float64))) ** 2
    res = []
    for lo, hi in zip(EDGES[:-1], EDGES[1:], strict=True):
        res.append(10.0 * math.log10(max(y[lo:hi].sum(), 1e-30) / x[lo:hi].sum()))
    return res


def left(words: list[float]) -> list[float]:
    """Left channel of `L R` interleaved output."""
    return words[0::2]


def oracle_words(image: str, binary: Path, scenario: tuple[int, int, int], frames) -> list[int]:
    """Oracle output words of a settled-start scenario (`CALLS * 8` words)."""
    import fx_biquad_check as fb
    import fx_oracle as fo

    mode, mf, mid = scenario
    t = cap.load_tables()
    mem0 = {lp.CTX + o: w for o, w in cap.template_words().items()}
    mem0[lp.CTX - 4] = mode
    mem0[lp.CTX - 32] = mem0[lp.CTX - 31] = t["mid_freq"][mf]
    res, pre = t["mid"][mid + 60]
    mem0[lp.CTX - 29] = mem0[lp.CTX - 27] = res
    mem0[lp.CTX - 28] = mem0[lp.CTX - 26] = pre
    cases = []
    for c in range(CALLS):
        mem = dict(mem0) if c == 0 else {}
        for i, (a, b) in enumerate(frames[c * 4 : (c + 1) * 4]):
            mem[lp.IN + 2 * i], mem[lp.IN + 2 * i + 1] = lp.bits(a), lp.bits(b)
        cases.append(mem)
    rt = fo.Routine("lp24", lp.PC, lp.END, lp.SETS, None, None)  # type: ignore[arg-type]
    text = fo.run_oracle(image, binary, rt, cases, set(range(lp.IN, lp.IN + 8)))
    res_d = fb.parse_dumps(text)
    return [res_d[c][lp.IN + k] for c in range(CALLS) for k in range(8)]


def model_words(scenario: tuple[int, int, int], frames) -> list[float]:
    """Stateful model output (settled start), `CALLS * 8` floats."""
    mode, mf, mid = scenario
    m = cap.Lp24(mode, mf, mid)
    t = cap.load_tables()
    m.c[-31] = lp.f32(t["mid_freq"][mf])
    res, pre = t["mid"][mid + 60]
    m.c[-27], m.c[-26] = lp.f32(res), lp.f32(pre)
    out: list[float] = []
    for c in range(CALLS):
        for a, b in m.call(frames[c * 4 : (c + 1) * 4]):
            out += [a, b]
    return out


def scenarios() -> list[tuple[int, int, int]]:
    """Every `(mode, kAmpMidFreq, kAmpMid)` of the sweep."""
    return [(m, f, d) for m in MODES for f in FREQS for d in MIDS]


def cmd_run(image: str, binary: Path) -> int:
    frames = cap.lcg_frames(SEED, CALLS * 4, AMP)
    inp = [a for a, _ in frames]
    rows = []
    for sc in scenarios():
        want = [lp.f32(w) for w in oracle_words(image, binary, sc, frames)]
        got = model_words(sc, frames)
        err = max(abs(a - b) for a, b in zip(want, got, strict=True))
        bands = band_db(left(want), inp)
        rows.append([*sc, f"{err:.3e}", *(f"{b:.4f}" for b in bands)])
        print(sc, f"word err {err:.2e}", f"peak band {max(bands):+.2f} dB")
    with CSV.open("w", encoding="utf-8", newline="") as fh:
        w = csv.writer(fh)
        w.writerow(["mode", "mid_freq", "mid", "word_err", *(f"b{i}" for i in range(12))])
        w.writerows(rows)
    print(f"wrote {CSV}")
    return 0


def load_rows(path: Path = CSV) -> list[dict[str, str]]:
    """The committed sweep rows."""
    with path.open(encoding="utf-8") as fh:
        return list(csv.DictReader(fh))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("cmd", choices=["run", "check"])
    ap.add_argument("--image")
    ap.add_argument("--binary", default="target/release/sharc-fx.exe")
    a = ap.parse_args()
    if a.cmd == "run":
        return cmd_run(a.image, Path(a.binary))
    rows = load_rows()
    print(f"{len(rows)} rows")
    return 0 if len(rows) == len(scenarios()) else 1


if __name__ == "__main__":
    raise SystemExit(main())
