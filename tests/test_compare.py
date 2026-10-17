"""Unit tests for scripts/compare.py. Run: python3 -m unittest discover -s tests -v"""

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
import compare  # noqa: E402


def good_case(fmt="Bgra8Unorm", adapter="Test GPU", hash_="aaaa"):
    clear = compare.expected_rgba(compare.CLEAR_LINEAR, fmt)
    tri = compare.expected_rgba(compare.TRIANGLE_LINEAR, fmt)
    return {
        "status": "completed",
        "selected_adapter": {"name": adapter, "vendor": 1, "device": 2, "device_type": "IntegratedGpu",
                             "backend": "Metal", "driver": "", "driver_info": ""},
        "errors": [],
        "windows": [{
            "context": "w0",
            "is_surface_supported": True,
            "configured": {"format": fmt, "present_mode": "Fifo"},
            "offscreen": {"format": fmt, "rgba_hash": hash_, "pixels_rgba": {
                "corner_0_0": clear, "corner_63_63": clear, "top_32_4": clear, "center_32_32": tri}},
            "present_deferred": {"status": "success"},
        }],
    }


def failed_case(cls, phase="device"):
    return {"status": "completed", "errors": [{"class": cls, "phase": phase, "context": "w0", "message": "x"}],
            "windows": []}


class ExpectedPixels(unittest.TestCase):
    def test_unorm_values(self):
        self.assertEqual(compare.expected_rgba(compare.CLEAR_LINEAR, "Rgba8Unorm"), [26, 51, 76, 255])

    def test_srgb_encodes_color_but_not_alpha(self):
        self.assertEqual(compare.expected_rgba((0.5, 0.0, 1.0, 0.5), "Bgra8UnormSrgb"), [188, 0, 255, 128])

    def test_pixel_check_tolerates_rounding(self):
        case = good_case()
        px = case["windows"][0]["offscreen"]["pixels_rgba"]
        px["center_32_32"] = [p + 1 for p in px["center_32_32"][:3]] + [255]
        self.assertTrue(compare.pixel_check(case["windows"][0]["offscreen"])["ok"])

    def test_pixel_check_rejects_missing_triangle(self):
        case = good_case()
        off = case["windows"][0]["offscreen"]
        off["pixels_rgba"]["center_32_32"] = off["pixels_rgba"]["corner_0_0"]
        self.assertFalse(compare.pixel_check(off)["ok"])


class ComparePair(unittest.TestCase):
    def test_identical_cases_match(self):
        r = compare.compare_pair("single", "worker-device", good_case(), good_case())
        self.assertEqual(r["verdict"], "match")
        self.assertEqual(r["differences"], [])

    def test_different_adapter_differs(self):
        r = compare.compare_pair("single", "worker-device", good_case(), good_case(adapter="Other GPU"))
        self.assertEqual(r["verdict"], "differ")
        self.assertIn("adapter", r["differences"])

    def test_different_format_and_hash_differ(self):
        r = compare.compare_pair("single", "worker-device", good_case(), good_case(fmt="Rgba8Unorm", hash_="bbbb"))
        self.assertIn("formats", r["differences"])
        self.assertIn("pixel_hashes", r["differences"])

    def test_same_error_class_matches(self):
        r = compare.compare_pair("failure-limits", "worker-device",
                                 failed_case("device-request-failed"), failed_case("device-request-failed"))
        self.assertEqual(r["verdict"], "match")

    def test_different_error_class_differs(self):
        r = compare.compare_pair("failure-limits", "worker-instance",
                                 failed_case("device-request-failed"), failed_case("adapter-not-found"))
        self.assertEqual(r["differences"], ["error_classes"])

    def test_known_difference_is_labeled(self):
        r = compare.compare_pair("failure-unsupported-mask", "worker-device",
                                 failed_case("surface-creation-failed", "surface"),
                                 failed_case("adapter-not-found", "adapter"))
        self.assertEqual(r["verdict"], "known-difference")
        self.assertIn("known", r["checks"]["error_classes"])


class Unavailable(unittest.TestCase):
    def test_no_backend_run_is_unavailable_not_failed(self):
        cases = {("ordinary", "single"): failed_case("surface-creation-failed", "surface"),
                 ("worker-device", "single"): failed_case("adapter-not-found", "adapter")}
        s = compare.summarize(cases)
        self.assertEqual(s["result"], "unavailable")
        self.assertEqual(s["comparisons"][0]["verdict"], "known-difference")

    def test_rendering_baseline_is_available(self):
        cases = {("ordinary", "single"): good_case(), ("worker-device", "single"): good_case()}
        self.assertFalse(compare.environment_unavailable(cases))

    def test_other_errors_in_unavailable_run_fail(self):
        cases = {("ordinary", "single"): failed_case("surface-creation-failed", "surface"),
                 ("worker-device", "single"): failed_case("panic", "worker")}
        self.assertEqual(compare.summarize(cases)["result"], "fail")


class Sanity(unittest.TestCase):
    def test_good_case_is_clean(self):
        self.assertEqual(compare.case_sanity("single", good_case()), [])

    def test_watchdog_status_is_reported(self):
        case = good_case()
        case["status"] = "watchdog-timeout"
        self.assertTrue(any("watchdog" in p for p in compare.case_sanity("single", case)))

    def test_unsupported_surface_is_reported(self):
        case = good_case()
        case["windows"][0]["is_surface_supported"] = False
        self.assertTrue(compare.case_sanity("single", case))

    def test_rejected_configure_is_reported(self):
        case = good_case()
        case["windows"][0]["configured"]["accepted"] = False
        self.assertTrue(any("configure was rejected" in p for p in compare.case_sanity("single", case)))

    def test_failure_without_error_is_reported(self):
        self.assertIn("forced failure produced no error", compare.case_sanity("failure-limits", good_case()))

    def test_abandon_gate_mismatch_is_reported(self):
        case = good_case()
        case["abandon"] = {"reached": "before-adapter", "finished_before_timeout": True}
        problems = compare.case_sanity("abandon-after-device", case)
        self.assertTrue(any("gate reached" in p for p in problems))


class Summarize(unittest.TestCase):
    def write(self, d, name, case):
        (Path(d) / f"{name}.json").write_text(json.dumps(case))

    def test_missing_record_fails(self):
        with tempfile.TemporaryDirectory() as d:
            self.write(d, "ordinary__single", good_case())
            (Path(d) / "worker-device__single.json").write_text("{not json")
            s = compare.summarize(compare.load_cases(Path(d)))
            self.assertFalse(s["ok"])
            self.assertEqual(s["comparisons"][0]["verdict"], "missing")

    def test_matching_run_passes_and_renders(self):
        with tempfile.TemporaryDirectory() as d:
            for strategy in compare.STRATEGIES:
                self.write(d, f"{strategy}__single", copy.deepcopy(good_case()))
            s = compare.summarize(compare.load_cases(Path(d)))
            self.assertTrue(s["ok"])
            self.assertEqual(len(s["comparisons"]), 3)
            self.assertIn("Overall: PASS", compare.render_markdown(s, "t"))


class HazardSanity(unittest.TestCase):
    def test_removed_device_after_present_is_reported(self):
        case = good_case()
        off = case["windows"][0]["offscreen"]
        case["windows"][0]["after_present"] = {"offscreen": off, "dx12": {"removed_reason": "0x887a0005 removed"}}
        self.assertTrue(any("device removed" in p for p in compare.case_sanity("probe-after-device", case | {
            "probe": {}})))

    def test_clean_after_present_passes(self):
        case = good_case()
        off = case["windows"][0]["offscreen"]
        case["windows"][0]["after_present"] = {"offscreen": off, "dx12": {"removed_reason": "S_OK"}}
        self.assertEqual(compare.case_sanity("single", case), [])

    def test_debug_layer_error_is_reported_and_warning_is_not(self):
        case = good_case()
        off = case["windows"][0]["offscreen"]
        case["windows"][0]["after_present"] = {"offscreen": off, "dx12": {"removed_reason": "S_OK", "info_queue": [
            {"severity": 2, "id": 820, "description": "clear value"},
            {"severity": 1, "id": 7, "description": "bad"}]}}
        problems = compare.case_sanity("single", case)
        self.assertEqual(len(problems), 1)
        self.assertIn("#7", problems[0])

    def test_missing_readback_after_present_is_reported(self):
        case = good_case()
        case["windows"][0]["after_present"] = {"offscreen": None, "offscreen_error": "lost"}
        self.assertTrue(any("no readback after present" in p for p in compare.case_sanity("single", case)))

    def test_logged_error_is_reported(self):
        case = good_case()
        case["log"] = [{"level": "ERROR", "target": "wgpu_hal::auxil::dxgi::exception", "message": "x"}]
        self.assertTrue(any("logged error" in p for p in compare.case_sanity("single", case)))

    def test_device_lost_is_fatal(self):
        case = good_case()
        case["errors"] = [{"class": "device-lost", "phase": "device_lost", "context": "w0", "message": "m"}]
        self.assertTrue(compare.case_sanity("single", case))

    def test_device_removed_by_probe_is_reported(self):
        case = good_case() | {"probe": {}, "dx12_after_probe": {"removed_reason": "0x887a0005 removed"}}
        self.assertTrue(any("after the probe" in p for p in compare.case_sanity("probe-after-device", case)))

    def test_probe_variant_without_probe_is_reported(self):
        self.assertIn("probe did not run", compare.case_sanity("probe-concurrent", good_case()))

    def test_skipped_gpu_needs_no_window(self):
        case = {"status": "completed", "errors": [], "windows": [], "gpu_skipped": True, "probe": {}}
        self.assertEqual(compare.case_sanity("probe-reject-keep", case), [])

    def test_join_that_did_not_block_is_reported(self):
        case = good_case()
        case["join_block"] = {"requested": "before-device", "reached": "before-device", "release_after_ms": 300,
                              "join_ms": 12.0, "worker_result_ok": True}
        problems = compare.case_sanity("join-block-before-device-before-loop", case)
        self.assertTrue(any("before the gate was released" in p for p in problems))

    def test_blocking_join_passes(self):
        case = good_case()
        case["join_block"] = {"requested": "after-device", "reached": "after-device", "release_after_ms": 300,
                              "join_ms": 301.5, "worker_result_ok": True}
        self.assertEqual(compare.case_sanity("join-block-after-device-after-window", case), [])

    def early_exit_case(self, stage="in-device", has_worker=True):
        return {"status": "early-exit", "errors": [], "windows": [],
                "early_exit": {"requested": "in-device", "has_worker": has_worker, "stage_at_decision": stage}}

    def test_early_exit_in_requested_call_passes(self):
        self.assertEqual(compare.case_sanity("early-exit-in-device", self.early_exit_case()), [])

    def test_early_exit_at_wrong_stage_is_reported(self):
        problems = compare.case_sanity("early-exit-in-device", self.early_exit_case("finished"))
        self.assertTrue(any("exited at worker stage" in p for p in problems))

    def test_instance_worker_finishes_before_device(self):
        case = self.early_exit_case("finished") | {"strategy": "worker-instance"}
        self.assertEqual(compare.case_sanity("early-exit-in-device", case), [])

    def test_early_exit_without_worker_passes(self):
        self.assertEqual(compare.case_sanity("early-exit-in-device", self.early_exit_case("idle", False)), [])


class ExitCodes(unittest.TestCase):
    def test_expected_exit_per_variant(self):
        self.assertEqual(compare.expected_exit("early-exit-in-adapter"), compare.EXIT_EARLY)
        self.assertEqual(compare.expected_exit("single"), 0)

    def test_wrong_exit_code_is_reported(self):
        problems = compare.exit_problems("early-exit-in-device", {"exit_code": 0, "exit": "completed"}, 60)
        self.assertTrue(any("expected 7" in p for p in problems))

    def test_slow_case_is_reported(self):
        problems = compare.exit_problems("single", {"exit_code": 0, "wall_s": 75.0}, 60)
        self.assertTrue(any("watchdog" in p for p in problems))

    def test_summary_uses_manifest(self):
        cases = {("ordinary", "single"): good_case(), ("worker-device", "single"): good_case()}
        manifest = {"timeout_s": 60, "cases": [{"case": "worker-device__single", "exit_code": 3,
                                                "exit": "watchdog", "wall_s": 61}]}
        self.assertFalse(compare.summarize(cases, manifest)["ok"])


class NewFacts(unittest.TestCase):
    def test_adapter_order_difference_is_reported(self):
        a, b = good_case(), good_case()
        a["adapters_surface_free"] = [{"name": "lvp", "backend": "Vulkan"}, {"name": "ss", "backend": "Vulkan"}]
        b["adapters_surface_free"] = list(reversed(a["adapters_surface_free"]))
        self.assertIn("adapter_order", compare.compare_pair("single", "worker-device", a, b)["differences"])

    def test_device_limits_difference_is_reported(self):
        a, b = good_case(), good_case()
        a["selected_device"] = {"features": "F", "limits": "L1"}
        b["selected_device"] = {"features": "F", "limits": "L2"}
        self.assertIn("device", compare.compare_pair("single", "worker-device", a, b)["differences"])

    def test_early_exit_run_is_not_unavailable(self):
        cases = {("ordinary", "early-exit-in-device"): {"status": "early-exit", "errors": [], "windows": []},
                 ("ordinary", "single"): good_case()}
        self.assertFalse(compare.environment_unavailable(cases))


if __name__ == "__main__":
    unittest.main()
