"""Unit tests for scripts/compare-runs.py. Run: python3 -m unittest discover -s tests -v"""

import importlib.util
import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent.parent / "scripts"
sys.path.insert(0, str(SCRIPTS))
_spec = importlib.util.spec_from_file_location("compare_runs", SCRIPTS / "compare-runs.py")
compare_runs = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(compare_runs)


def case(adapter="lavapipe", order=("lavapipe", "SwiftShader"), limits="L", dx12_gate=False):
    return {
        "selected_adapter": {"name": adapter, "backend": "Vulkan", "vendor": 1, "device": 2,
                             "device_type": "Cpu", "driver": ""},
        "adapters_surface_free": [{"name": n, "backend": "Vulkan"} for n in order],
        "selected_device": {"features": "F", "limits": limits},
        "backends": {"dx12_gate_would_prepare": dx12_gate, "slint_would_prepare_on_worker": False},
    }


class CaseFacts(unittest.TestCase):
    def test_extracts_selection_and_gates(self):
        f = compare_runs.case_facts(case(dx12_gate=True))
        self.assertEqual(f["adapter"], "lavapipe (Vulkan)")
        self.assertEqual(f["adapter_order"], ["lavapipe (Vulkan)", "SwiftShader (Vulkan)"])
        self.assertTrue(f["dx12_gate"])

    def test_failed_case_has_no_adapter(self):
        self.assertIsNone(compare_runs.case_facts({"errors": []})["adapter"])


class SelfConsistency(unittest.TestCase):
    def test_agreeing_strategies_pass(self):
        facts = compare_runs.run_facts({("ordinary", "single"): case(), ("worker-device", "single"): case()})
        self.assertEqual(compare_runs.self_consistency(facts), [])

    def test_disagreeing_selection_is_reported(self):
        facts = compare_runs.run_facts({("ordinary", "single"): case(),
                                        ("worker-device", "single"): case(adapter="SwiftShader")})
        self.assertTrue(any("adapter" in p for p in compare_runs.self_consistency(facts)))

    def test_missing_adapter_is_ignored(self):
        facts = compare_runs.run_facts({("ordinary", "failure-limits"): {"errors": []},
                                        ("worker-device", "failure-limits"): case()})
        self.assertEqual(compare_runs.self_consistency(facts), [])


class CrossCheck(unittest.TestCase):
    def runs(self, **overrides):
        base = compare_runs.run_facts({("ordinary", "single"): case()})
        other = compare_runs.run_facts({("ordinary", "single"): case(**overrides)})
        return {"a": base, "b": other}

    def test_equal_runs_pass(self):
        self.assertEqual(compare_runs.cross_check(self.runs(), ["adapter", "device"]), [])

    def test_changed_limits_are_reported(self):
        problems = compare_runs.cross_check(self.runs(limits="L2"), ["device"])
        self.assertTrue(any("device differs" in p for p in problems))

    def test_unchecked_fact_may_differ(self):
        self.assertEqual(compare_runs.cross_check(self.runs(order=("SwiftShader", "lavapipe")), ["adapter"]), [])

    def test_only_shared_cases_are_compared(self):
        runs = {"a": compare_runs.run_facts({("ordinary", "single"): case()}),
                "b": compare_runs.run_facts({("ordinary", "multi-window"): case(adapter="x")})}
        self.assertEqual(compare_runs.cross_check(runs, ["adapter"]), [])


if __name__ == "__main__":
    unittest.main()
