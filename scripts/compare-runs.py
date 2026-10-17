#!/usr/bin/env python3
"""Compare selection facts across several matrix runs made under different environments.

Usage:
  python3 scripts/compare-runs.py runs/a runs/b [runs/c ...] [--same adapter device] [--out summary.md]

Each run directory is a run-matrix.py output. For every run, the table lists the adapter each
strategy selected, the surface-free enumeration order, and the Slint gates.
`--same` names facts that must be identical across all runs for each strategy and variant,
such as `adapter` and `device` for an environment variable that must not change selection.
Exits 1 when a run disagrees with itself across strategies, or when a `--same` fact differs.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import compare  # noqa: E402

CROSS_FACTS = ("adapter", "adapter_order", "device", "dx12_gate", "metal_gate", "dx12_compiler")


def case_facts(case: dict) -> dict:
    """The facts of one record that matter across environments."""
    adapter = compare.adapter_identity(case)
    backends = case.get("backends") or {}
    return {
        "adapter": f"{adapter['name']} ({adapter['backend']})" if adapter else None,
        "adapter_order": compare.adapter_order(case),
        "device": case.get("selected_device"),
        "dx12_gate": backends.get("dx12_gate_would_prepare"),
        "metal_gate": backends.get("slint_would_prepare_on_worker"),
        "dx12_compiler": case.get("dx12_compiler"),
    }


def run_facts(cases: dict) -> dict:
    """Maps (strategy, variant) to its facts, skipping unreadable records."""
    return {key: case_facts(case) for key, case in cases.items() if case is not None}


def self_consistency(facts: dict) -> list[str]:
    """Strategies of one run must agree on the adapter and the enumeration order per variant."""
    problems = []
    for variant in sorted({v for _, v in facts}):
        for name in ("adapter", "adapter_order"):
            seen = {json.dumps(f[name]) for (s, v), f in facts.items() if v == variant and f[name] is not None}
            if len(seen) > 1:
                problems.append(f"{variant}: strategies disagree on {name}: {sorted(seen)}")
    return problems


def cross_check(runs: dict, same: list[str]) -> list[str]:
    """Each `same` fact must be equal across runs for every (strategy, variant) they share."""
    problems = []
    keys = set.intersection(*(set(f) for f in runs.values())) if runs else set()
    for key in sorted(keys):
        for name in same:
            values = {run: json.dumps(f[key][name]) for run, f in runs.items()}
            if len(set(values.values())) > 1:
                detail = ", ".join(f"{run}: {v}" for run, v in values.items())
                problems.append(f"{key[0]}/{key[1]}: {name} differs across runs: {detail}")
    return problems


def render(runs: dict, envs: dict, problems: list[str]) -> str:
    lines = ["# Cross-Run Comparison", "", f"Overall: {'FAIL' if problems else 'PASS'}", "",
             "| Run | Environment | Variant | Strategy | Adapter | Order | DX12 gate | Compiler |",
             "| --- | --- | --- | --- | --- | --- | --- | --- |"]
    for run, facts in runs.items():
        env = " ".join(f"{k}={v}" for k, v in sorted(envs.get(run, {}).items())) or "-"
        for (strategy, variant), f in sorted(facts.items(), key=lambda kv: (kv[0][1], kv[0][0])):
            order = " > ".join(f["adapter_order"] or []) or "-"
            lines.append(f"| {run} | {env} | {variant} | {strategy} | {f['adapter'] or '-'} | {order} | "
                         f"{f['dx12_gate']} | {f['dx12_compiler'] or '-'} |")
    if problems:
        lines += ["", "## Problems", ""] + [f"- {p}" for p in problems]
    return "\n".join(lines) + "\n"


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("runs", nargs="+", type=Path)
    ap.add_argument("--same", nargs="*", default=[], choices=CROSS_FACTS)
    ap.add_argument("--out", type=Path, help="also write the Markdown table here")
    args = ap.parse_args(argv)
    runs, envs, problems = {}, {}, []
    for d in args.runs:
        facts = run_facts(compare.load_cases(d))
        runs[d.name] = facts
        envs[d.name] = compare.load_manifest(d).get("env_overrides", {})
        problems += [f"{d.name}: {p}" for p in self_consistency(facts)]
    problems += cross_check(runs, args.same)
    text = render(runs, envs, problems)
    if args.out:
        args.out.write_text(text)
    print(text)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
