#!/usr/bin/env python3
"""Compare worker strategies against `ordinary` for one matrix run directory.

Usage: python3 scripts/compare.py runs/<name> [--out runs/<name>/summary.json]

Each case file is `<strategy>__<variant>.json`, as written by run-matrix.py.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

BASELINE = "ordinary"
STRATEGIES = ["ordinary", "worker-device", "worker-instance", "worker-after-select"]
# Strategies whose worker starts before the event loop, so it can be gated or exited under.
EARLY_WORKERS = {"worker-device", "worker-instance"}

# The harness's intended exit code per variant prefix, and the status its record ends with.
EARLY_EXIT_PREFIX = "early-exit-"
EXIT_EARLY = 7
FATAL_CLASSES = ("panic", "timeout", "wgpu-validation", "device-lost")
# `D3D12_MESSAGE_SEVERITY`: 0 corruption, 1 error, 2 warning, 3 info, 4 message.
D3D12_SEVERITY_WARNING = 2
# A join that returned sooner than this before the release didn't block on the gate.
JOIN_SLACK_MS = 5


def expected_exit(variant: str) -> int:
    return EXIT_EARLY if variant.startswith(EARLY_EXIT_PREFIX) else 0


def expected_status(variant: str) -> str:
    return "early-exit" if variant.startswith(EARLY_EXIT_PREFIX) else "completed"

# Error classes that mean no HAL backend loaded for the requested mask.
NO_BACKEND = {"surface-creation-failed", "adapter-not-found", "no-backends"}
NO_BACKEND_NOTE = (
    "no backend loaded for this mask: surface-first paths fail at surface creation, "
    "surface-free selection fails at adapter selection"
)


def known_difference(check: str, base: dict, cand: dict) -> str | None:
    """Explains a difference that follows from the strategy's design, not from a defect."""
    if check != "error_classes":
        return None
    b, c = set(base["error_classes"]), set(cand["error_classes"])
    if b and c and b <= NO_BACKEND and c <= NO_BACKEND:
        return NO_BACKEND_NOTE
    return None


def environment_unavailable(cases: dict) -> bool:
    """True when ordinary never got a device outside the failure variants."""
    ordinary = [c for (s, v), c in cases.items()
                if s == BASELINE and not v.startswith(("failure-", EARLY_EXIT_PREFIX))]
    if not ordinary or any(c is None for c in ordinary):
        return False
    return all(
        not c.get("windows") and c.get("errors") and {e.get("class") for e in c["errors"]} <= NO_BACKEND
        for c in ordinary
    )

CLEAR_LINEAR = (0.1, 0.2, 0.3, 1.0)
TRIANGLE_LINEAR = (1.0, 0.5, 0.0, 1.0)
PIXEL_TOLERANCE = 2


def srgb_encode(c: float) -> float:
    return 12.92 * c if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055


def expected_rgba(linear: tuple, fmt: str) -> list[int]:
    srgb = fmt.endswith("Srgb")
    out = []
    for i, c in enumerate(linear):
        v = srgb_encode(c) if srgb and i < 3 else c
        out.append(round(v * 255))
    return out


def close(a: list[int], b: list[int], tol: int = PIXEL_TOLERANCE) -> bool:
    return len(a) == len(b) and all(abs(x - y) <= tol for x, y in zip(a, b))


def pixel_check(offscreen: dict) -> dict:
    """Checks the readback against the scene's known colors for its format."""
    fmt = offscreen.get("format", "")
    px = offscreen.get("pixels_rgba", {})
    clear = expected_rgba(CLEAR_LINEAR, fmt)
    tri = expected_rgba(TRIANGLE_LINEAR, fmt)
    checks = {
        "corner_is_clear": close(px.get("corner_0_0", []), clear),
        "far_corner_is_clear": close(px.get("corner_63_63", []), clear),
        "center_is_triangle": close(px.get("center_32_32", []), tri),
        "top_is_clear": close(px.get("top_32_4", []), clear),
    }
    return {"ok": all(checks.values()), "checks": checks, "expected_clear": clear, "expected_triangle": tri}


def adapter_identity(case: dict) -> dict | None:
    a = case.get("selected_adapter")
    if not a:
        return None
    return {k: a.get(k) for k in ("name", "vendor", "device", "device_type", "backend", "driver")}


def window_map(case: dict, key) -> dict:
    return {w.get("context"): key(w) for w in case.get("windows", [])}


def present_status(w: dict):
    deferred = w.get("present_deferred")
    if deferred:
        return deferred.get("status")
    return w.get("present_immediate")


def adapter_order(case: dict) -> list | None:
    """Names and backends of the surface-free enumeration, in order."""
    order = case.get("adapters_surface_free")
    if order is None:
        return None
    return [f"{a.get('name')} ({a.get('backend')})" for a in order]


def facts(case: dict) -> dict:
    """The comparable facts of one case record."""
    return {
        "status": case.get("status"),
        "adapter": adapter_identity(case),
        "device": case.get("selected_device"),
        "adapter_order": adapter_order(case),
        "formats": window_map(case, lambda w: (w.get("configured") or {}).get("format")),
        "present_modes": window_map(case, lambda w: (w.get("configured") or {}).get("present_mode")),
        "pixel_hashes": window_map(case, lambda w: (w.get("offscreen") or {}).get("rgba_hash")),
        "present": window_map(case, present_status),
        "error_classes": sorted({e.get("class") for e in case.get("errors", [])}),
    }


def case_sanity(variant: str, case: dict, unavailable: bool = False) -> list[str]:
    """Problems visible in one case without a baseline.
    With `unavailable`, a missing adapter is the expected outcome, not a problem."""
    problems = []
    if unavailable:
        if case.get("status") != expected_status(variant):
            problems.append(f"status is {case.get('status')!r}")
        classes = {e.get("class") for e in case.get("errors", [])}
        if not classes or not classes <= NO_BACKEND:
            problems.append(f"expected only no-backend errors, got {sorted(classes)}")
        return problems
    if case.get("status") != expected_status(variant):
        problems.append(f"status is {case.get('status')!r}")
    for e in case.get("errors", []):
        if e.get("class") in FATAL_CLASSES:
            problems.append(f"{e.get('class')} in {e.get('context')}/{e.get('phase')}: {e.get('message')}")
    for rec in case.get("log", []):
        if rec.get("level") == "ERROR":
            problems.append(f"logged error [{rec.get('target')}]: {rec.get('message')}")
    expect_render = not (variant.startswith("failure-") or variant.startswith(EARLY_EXIT_PREFIX)
                         or case.get("gpu_skipped"))
    windows = case.get("windows", [])
    if expect_render and not windows:
        problems.append("no window was rendered")
    for w in windows:
        off = w.get("offscreen")
        if expect_render and not off:
            problems.append(f"{w.get('context')}: no offscreen readback")
        elif off and not pixel_check(off)["ok"]:
            problems.append(f"{w.get('context')}: pixels differ from the expected scene")
        if (w.get("configured") or {}).get("accepted") is False:
            problems.append(f"{w.get('context')}: surface configure was rejected, nothing was presented")
        if w.get("is_surface_supported") is False:
            problems.append(f"{w.get('context')}: selected adapter does not support this surface")
        problems += after_present_problems(w)
    if variant.startswith("failure-") and not case.get("errors"):
        problems.append("forced failure produced no error")
    ab = case.get("abandon")
    if variant.startswith("abandon-") and ab is not None:
        requested = variant.removeprefix("abandon-")
        if ab.get("reached") != requested:
            problems.append(f"gate reached {ab.get('reached')!r}, expected {requested!r}")
        if not ab.get("finished_before_timeout"):
            problems.append("abandoned worker did not finish before the timeout")
    problems += join_block_problems(variant, case)
    problems += early_exit_problems(variant, case)
    if variant.startswith("probe-") and "probe" not in case:
        problems.append("probe did not run")
    removed = (case.get("dx12_after_probe") or {}).get("removed_reason")
    if removed not in (None, "S_OK"):
        problems.append(f"device removed after the probe: {removed}")
    return problems


def after_present_problems(w: dict) -> list[str]:
    """The device must still render and must not be removed once the first frame was presented."""
    after = w.get("after_present")
    if after is None:
        return []
    ctx = w.get("context")
    problems = []
    off = after.get("offscreen")
    if not off:
        problems.append(f"{ctx}: no readback after present: {after.get('offscreen_error')}")
    elif not pixel_check(off)["ok"]:
        problems.append(f"{ctx}: pixels after present differ from the expected scene")
    dx12 = after.get("dx12") or {}
    reason = dx12.get("removed_reason")
    if reason not in (None, "S_OK"):
        problems.append(f"{ctx}: device removed after present: {reason}")
    for m in dx12.get("info_queue") or []:
        if m.get("severity", D3D12_SEVERITY_WARNING) < D3D12_SEVERITY_WARNING:
            problems.append(f"{ctx}: D3D12 debug layer error #{m.get('id')}: {m.get('description')}")
    return problems


def join_block_problems(variant: str, case: dict) -> list[str]:
    """`join-block` must have blocked on the gate until the release, then finished normally."""
    jb = case.get("join_block")
    if not variant.startswith("join-block-") or jb is None:
        return []
    problems = []
    if jb.get("reached") != jb.get("requested"):
        problems.append(f"gate reached {jb.get('reached')!r}, expected {jb.get('requested')!r}")
    if jb.get("join_ms", 0) < jb.get("release_after_ms", 0) - JOIN_SLACK_MS:
        problems.append(f"join returned after {jb.get('join_ms')} ms, before the gate was released")
    if jb.get("worker_result_ok") is not True:
        problems.append("joined worker has no usable result")
    return problems


def early_exit_problems(variant: str, case: dict) -> list[str]:
    """`early-exit` must have exited while the worker was in the requested call."""
    ee = case.get("early_exit")
    if not variant.startswith(EARLY_EXIT_PREFIX):
        return []
    if ee is None:
        return ["early exit was not recorded"]
    if case.get("strategy") == "worker-instance" and ee.get("requested") != "in-instance":
        # That worker creates only the instance, so it finishes without entering the call.
        expected = "finished"
    else:
        expected = ee.get("requested")
    if ee.get("has_worker") and ee.get("stage_at_decision") != expected:
        return [f"exited at worker stage {ee.get('stage_at_decision')!r}, expected {expected!r}"]
    return []


def exit_problems(variant: str, entry: dict | None, timeout_s: float | None) -> list[str]:
    """Checks one manifest entry: the intended exit code, within the watchdog timeout."""
    if entry is None:
        return []
    problems = []
    want = expected_exit(variant)
    if entry.get("exit_code") != want:
        problems.append(f"exit code {entry.get('exit_code')!r} ({entry.get('exit')}), expected {want}")
    if timeout_s is not None and entry.get("wall_s", 0) > timeout_s:
        problems.append(f"took {entry.get('wall_s')} s, longer than the {timeout_s} s watchdog")
    return problems


def compare_pair(variant: str, strategy: str, base: dict, cand: dict) -> dict:
    fb, fc = facts(base), facts(cand)
    checks = {}
    for name in ("status", "adapter", "device", "adapter_order", "formats", "present_modes", "pixel_hashes",
                 "present", "error_classes"):
        equal = fb[name] == fc[name]
        entry = {"equal": equal, BASELINE: fb[name], strategy: fc[name]}
        known = known_difference(name, fb, fc)
        if not equal and known:
            entry["known"] = known
        checks[name] = entry
    unexpected = [n for n, c in checks.items() if not c["equal"] and "known" not in c]
    if unexpected:
        verdict = "differ"
    elif any(not c["equal"] for c in checks.values()):
        verdict = "known-difference"
    else:
        verdict = "match"
    return {"variant": variant, "strategy": strategy, "verdict": verdict, "differences": unexpected, "checks": checks}


def load_cases(run_dir: Path) -> dict:
    """Maps (strategy, variant) to the parsed case record, or None when it is missing or unreadable."""
    cases = {}
    for path in sorted(run_dir.glob("*__*.json")):
        strategy, variant = path.stem.split("__", 1)
        try:
            cases[(strategy, variant)] = json.loads(path.read_text())
        except (OSError, json.JSONDecodeError):
            cases[(strategy, variant)] = None
    return cases


def load_manifest(run_dir: Path) -> dict:
    try:
        return json.loads((run_dir / "manifest.json").read_text())
    except (OSError, json.JSONDecodeError):
        return {}


def summarize(cases: dict, manifest: dict | None = None) -> dict:
    variants = sorted({v for _, v in cases})
    unavailable = environment_unavailable(cases)
    manifest = manifest or {}
    exits = {c["case"]: c for c in manifest.get("cases", [])}
    timeout_s = manifest.get("timeout_s")
    sanity = []
    for (strategy, variant), case in sorted(cases.items()):
        problems = ["record missing or unreadable"] if case is None else case_sanity(variant, case, unavailable)
        problems += exit_problems(variant, exits.get(f"{strategy}__{variant}"), timeout_s)
        sanity.append({"strategy": strategy, "variant": variant, "ok": not problems, "problems": problems})
    comparisons = []
    for variant in variants:
        # Abandon has no ordinary worker; its ordinary case only checks post-abandon rendering.
        base = cases.get((BASELINE, variant))
        for strategy in STRATEGIES[1:]:
            if (strategy, variant) not in cases:
                continue
            cand = cases[(strategy, variant)]
            if base is None or cand is None:
                comparisons.append({"variant": variant, "strategy": strategy, "verdict": "missing",
                                    "differences": [], "checks": {}})
                continue
            comparisons.append(compare_pair(variant, strategy, base, cand))
    ok = all(s["ok"] for s in sanity) and all(c["verdict"] in ("match", "known-difference") for c in comparisons)
    result = ("unavailable" if unavailable else "pass") if ok else "fail"
    return {"ok": ok, "result": result, "sanity": sanity, "comparisons": comparisons}


def render_markdown(summary: dict, title: str) -> str:
    overall = summary.get("result", "pass" if summary["ok"] else "fail").upper()
    lines = [f"# {title}", "", f"Overall: {overall}", ""]
    if overall == "UNAVAILABLE":
        lines += ["No adapter exists for this backend mask, so nothing was rendered.",
                  "Every strategy failed with a no-backend error, which is all this run establishes.", ""]
    lines += [
             "## Comparisons Against Ordinary", "",
             "| Variant | Strategy | Verdict | Differences |", "| --- | --- | --- | --- |"]
    for c in summary["comparisons"]:
        diffs = ", ".join(c["differences"]) or "-"
        known = [n for n, v in c["checks"].items() if "known" in v]
        if known:
            diffs += " (known: " + ", ".join(known) + ")"
        lines.append(f"| {c['variant']} | {c['strategy']} | {c['verdict']} | {diffs} |")
    lines += ["", "## Per-Case Sanity", "", "| Strategy | Variant | OK | Problems |", "| --- | --- | --- | --- |"]
    for s in summary["sanity"]:
        lines.append(f"| {s['strategy']} | {s['variant']} | {'yes' if s['ok'] else 'no'} | "
                     f"{'; '.join(s['problems']) or '-'} |")
    return "\n".join(lines) + "\n"


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("run_dir", type=Path)
    ap.add_argument("--out", type=Path, help="summary JSON path (default: <run_dir>/summary.json)")
    args = ap.parse_args(argv)
    summary = summarize(load_cases(args.run_dir), load_manifest(args.run_dir))
    out = args.out or args.run_dir / "summary.json"
    out.write_text(json.dumps(summary, indent=2) + "\n")
    out.with_suffix(".md").write_text(render_markdown(summary, f"Summary: {args.run_dir.name}"))
    print(render_markdown(summary, f"Summary: {args.run_dir.name}"))
    return {"pass": 0, "unavailable": 4}.get(summary["result"], 1)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
