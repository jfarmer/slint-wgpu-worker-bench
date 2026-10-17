#!/usr/bin/env python3
"""Run every strategy against every scenario and compare each worker strategy with ordinary.

Examples:
  python3 scripts/run-matrix.py --out runs/metal
  python3 scripts/run-matrix.py --out runs/dx12 --env WGPU_BACKEND=dx12
  python3 scripts/run-matrix.py --out runs/quick --scenarios single failure-limits
  python3 scripts/run-matrix.py --out runs/dx12-hazards --suite base hazards --env WGPU_BACKEND=dx12

Writes runs/<name>/<strategy>__<variant>.json, one log per case, manifest.json with the
environment and exit codes, and summary.json / summary.md from compare.py.
Exits 1 when a case fails its sanity checks or differs from ordinary unexpectedly,
and 4 when no adapter exists for the backend mask (see compare.py).
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import compare  # noqa: E402

EXE = "slint-wgpu-worker-bench" + (".exe" if os.name == "nt" else "")

# Variant name -> harness arguments.
BASE = {
    "single": ["--scenario", "single"],
    "multi-window": ["--scenario", "multi-window"],
    "repeated": ["--scenario", "repeated"],
    "abandon-before-adapter": ["--scenario", "abandon", "--abandon-at", "before-adapter"],
    "abandon-before-device": ["--scenario", "abandon", "--abandon-at", "before-device"],
    "abandon-after-device": ["--scenario", "abandon", "--abandon-at", "after-device"],
    "failure-limits": ["--scenario", "failure", "--failure", "limits"],
    "failure-empty-mask": ["--scenario", "failure", "--failure", "empty-mask"],
    "failure-unsupported-mask": ["--scenario", "failure", "--failure", "unsupported-mask"],
}

GATES = ["before-adapter", "before-device", "after-device"]

# Slint's main-thread GPU probe at each point of worker progress (dx12-worker-safety.md W2, W7).
# `continue` renders after a rejection, as a GPU adapter passing the probe would.
PROBE = {
    **{f"probe-{p}": ["--scenario", "single", "--probe", p, "--on-probe-reject", "continue"]
       for p in ["concurrent", "after-instance", "after-adapter", "after-device", "after-first-device"]},
    "probe-reject-keep": ["--scenario", "single", "--probe", "concurrent", "--on-probe-reject", "keep"],
    "probe-reject-drop": ["--scenario", "single", "--probe", "concurrent", "--on-probe-reject", "drop"],
}

# Positive control for W2: the probe alone enables the D3D12 debug layer after the device exists.
# Run it with WGPU_VALIDATION=0 so the device is created without the layer.
CONTROL = {
    "control-late-debug-layer": ["--scenario", "single", "--probe", "after-device", "--on-probe-reject", "continue",
                                 "--probe-validation", "on"],
    "control-late-debug-layer-first-device": ["--scenario", "single", "--probe", "after-first-device",
                                              "--probe-validation", "on"],
}

# The main thread blocked in a plain join while the worker is gated (W4), and process exit
# while the worker is inside a call (W6).
LIFETIME = {
    **{f"join-block-{g}-{j}": ["--scenario", "join-block", "--abandon-at", g, "--join-at", at]
       for g in GATES for j, at in [("before-loop", "before-event-loop"), ("after-window", "after-window")]},
    **{f"early-exit-{e}": ["--scenario", "early-exit", "--exit-at", e]
       for e in ["in-instance", "in-adapter", "in-device"]},
}

VARIANTS = {**BASE, **PROBE, **LIFETIME, **CONTROL}
SUITES = {"base": list(BASE), "probe": list(PROBE), "lifetime": list(LIFETIME), "control": list(CONTROL),
          "hazards": list(PROBE) + list(LIFETIME), "all": list(VARIANTS)}

EXIT_MEANINGS = {0: "completed", 2: "usage", 3: "watchdog", 5: "panic", 7: "early-exit", 101: "rust-panic"}


def capture(cmd: list[str]) -> str | None:
    try:
        return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=60).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return None


def build(profile: str) -> Path:
    cmd = ["cargo", "build", "--locked"] + (["--release"] if profile == "release" else [])
    subprocess.run(cmd, cwd=ROOT, check=True)
    return ROOT / "target" / profile / EXE


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", type=Path, required=True, help="new run directory, such as runs/metal")
    ap.add_argument("--strategies", nargs="+", default=compare.STRATEGIES, choices=compare.STRATEGIES)
    ap.add_argument("--suite", nargs="+", choices=list(SUITES), default=["base"],
                    help="variant groups to run when --scenarios is not given (default base)")
    ap.add_argument("--scenarios", nargs="+", choices=list(VARIANTS), help="explicit variants")
    ap.add_argument("--env", action="append", default=[], metavar="KEY=VALUE",
                    help="environment for every case; KEY= unsets KEY")
    ap.add_argument("--windows", type=int, default=2)
    ap.add_argument("--repeat", type=int, default=3)
    ap.add_argument("--timeout", type=int, default=60, help="per-case watchdog seconds")
    ap.add_argument("--profile", choices=["debug", "release"], default="release")
    ap.add_argument("--binary", type=Path, help="use this harness binary instead of building")
    args = ap.parse_args()
    scenarios = args.scenarios or list(dict.fromkeys(v for suite in args.suite for v in SUITES[suite]))

    out = args.out if args.out.is_absolute() else ROOT / args.out
    if out.exists() and any(out.iterdir()):
        sys.exit(f"{out} is not empty; use a new run directory")
    out.mkdir(parents=True, exist_ok=True)

    env = dict(os.environ)
    overrides = {}
    for item in args.env:
        key, _, value = item.partition("=")
        overrides[key] = value
        if value:
            env[key] = value
        else:
            env.pop(key, None)

    binary = args.binary or build(args.profile)
    manifest = {
        "started": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "argv": sys.argv,
        "env_overrides": overrides,
        "platform": {"system": platform.system(), "release": platform.release(), "version": platform.version(),
                     "machine": platform.machine(), "python": platform.python_version()},
        "harness_commit": capture(["git", "rev-parse", "HEAD"]),
        "harness_dirty": bool(capture(["git", "status", "--porcelain", "--", "src", "Cargo.toml", "Cargo.lock"])),
        "rustc": capture(["rustc", "-V"]),
        "cargo": capture(["cargo", "-V"]),
        "binary": str(binary),
        "profile": None if args.binary else args.profile,
        "timeout_s": args.timeout,
        "cases": [],
    }
    for strategy in args.strategies:
        for variant in scenarios:
            name = f"{strategy}__{variant}"
            cmd = [str(binary), "--strategy", strategy, *VARIANTS[variant], "--out", str(out / f"{name}.json"),
                   "--windows", str(args.windows), "--repeat", str(args.repeat), "--timeout", str(args.timeout)]
            start = time.monotonic()
            try:
                proc = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=args.timeout + 30)
                code, stdout, stderr = proc.returncode, proc.stdout, proc.stderr
            except subprocess.TimeoutExpired as e:
                code, stdout, stderr = None, e.stdout or "", (e.stderr or "") + "\nrunner timeout\n"
            (out / f"{name}.log").write_text(f"$ {' '.join(cmd)}\n--- stdout\n{stdout}\n--- stderr\n{stderr}")
            meaning = EXIT_MEANINGS.get(code, "runner-timeout" if code is None else "other")
            manifest["cases"].append({"case": name, "exit_code": code, "exit": meaning,
                                      "wall_s": round(time.monotonic() - start, 3)})
            print(f"{name}: {meaning} ({code})", flush=True)
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return compare.main([str(out)])


if __name__ == "__main__":
    sys.exit(main())
