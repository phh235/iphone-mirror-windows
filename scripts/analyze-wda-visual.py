"""Offline WDA result-ROI/PresentMon correlation. Never equates HTTP with display."""
import argparse
import bisect
import csv
import json
import math
import unittest
from pathlib import Path
from statistics import fmean


def distribution(values):
    values = sorted(values)
    if not values:
        return None
    p = lambda q: values[max(0, math.ceil(len(values)*q)-1)]
    return {"samples": len(values), "average_ms": fmean(values), "p50_ms": p(.5),
            "p95_ms": p(.95), "p99_ms": p(.99), "max_ms": values[-1]}


def correlate(record, rows, starts, frequency):
    events = record.get("visual", {}).get("events", [])
    start = record.get("generated_input_qpc")
    if not start or not events:
        return {"available": False, "reason": "No generated-input timestamp or changed ROI frame"}
    if any("render_width" in e and (e["render_width"] < 32 or e["render_height"] < 32) for e in events):
        return {"available": False, "reason": "Changed ROI was not rendered to a usable viewport"}
    if any(e["present_start_qpc"] < start for e in events):
        return {"available": False, "reason": "ROI changed before the input; exclude false-positive candidate"}
    displays = []
    matched = 0
    for event in events:
        lo = bisect.bisect_left(starts, event["present_start_qpc"]-frequency*.0001)
        hi = bisect.bisect_right(starts, event["present_end_qpc"]+frequency*.0001)
        possible = [r for r in rows[lo:hi] if r["chain"] == event["swap_chain"]]
        if not possible:
            continue
        row = min(possible, key=lambda r: abs(r["qpc"]-event["present_start_qpc"]))
        matched += 1
        if row["display"] is not None and row["display"] >= start:
            displays.append(row["display"])
    return {"available": bool(displays), "roi_frames": len(events), "matched_frames": matched,
            "generated_input_to_result_display_ms": (min(displays)-start)*1000/frequency if displays else None,
            "reason": None if displays else "No ETW display of a changed ROI frame"}


def analyze(result_path, etw_path):
    report = json.loads(result_path.read_text(encoding="utf-8-sig"))
    pid = report.get("visual_config", {}).get("pid")
    records = report.get("results", [])
    frequency = next((r.get("visual_arm", {}).get("frequency") for r in records if r.get("visual_arm")), None)
    rows = []
    if etw_path and pid and frequency:
        with etw_path.open(encoding="utf-8-sig", newline="") as stream:
            for r in csv.DictReader(stream):
                if r.get("ProcessID") != str(pid):
                    continue
                try:
                    qpc = int(r["QPCTime"])
                    chain = int(r["SwapChainAddress"], 0)
                except (ValueError, KeyError):
                    continue
                try:
                    until = float(r["msUntilDisplayed"])
                except (ValueError,KeyError):
                    until = float("nan")
                display = qpc+until*frequency/1000 if r.get("Dropped") == "0" and math.isfinite(until) and until >= 0 else None
                rows.append({"qpc": qpc, "chain": chain, "display": display})
        rows.sort(key=lambda r: r["qpc"])
    starts = [r["qpc"] for r in rows]
    summaries = []
    details = []
    for r in records:
        visual = correlate(r, rows, starts, frequency) if rows else {"available": False, "reason": "ETW unavailable"}
        details.append({"round": r["round"], "contact_ms": r["contact_ms"], "outcome": r["outcome"], "visual": visual})
    for contact in report["contacts_ms"]:
        selected = [r for r in records if r["contact_ms"] == contact]
        corresponding = [d for d in details if d["contact_ms"] == contact]
        passed = sum(r["outcome"] == "PASS" for r in selected)
        summaries.append({"contact_ms": contact, "attempts": len(selected), "verified_pass": passed,
            "success_rate_percent": 100*passed/len(selected) if selected else None,
            "missed": sum(r["outcome"] == "MISSED" for r in selected),
            "wrong_or_unrecognized": sum(r["outcome"] == "WRONG_OR_UNRECOGNIZED" for r in selected),
            "http_or_verification_unknown": sum(r["outcome"] not in ("PASS", "MISSED", "WRONG_OR_UNRECOGNIZED") for r in selected),
            "dispatch_ms": distribution([r["dispatch_ms"] for r in selected]),
            "http_stages": {name: distribution([r["http"][name] for r in selected if (r.get("http") or {}).get(name) is not None])
                for name in ("request_preparation_ms", "send_to_headers_ms", "response_body_ms", "response_validation_ms", "total_ms")},
            "visual_coverage": sum(d["visual"]["available"] and d["outcome"] == "PASS" for d in corresponding),
            "generated_input_to_result_display_ms": distribution([d["visual"]["generated_input_to_result_display_ms"]
                for d in corresponding if d["visual"]["available"] and d["outcome"] == "PASS"])})
    return {"complete": report["complete"], "summary": summaries, "trials": details,
        "status_request_ms": distribution([r["total_ms"] for r in report.get("status_probes", [])]),
        "notes": ["Timing starts at generated WDA dispatch, not a physical Windows mouse event.",
            "ROI covers Calculator result, not the earlier key-highlight feedback; only real-state PASS samples enter visual percentiles.",
            "ETW gives Windows display timing, not phone-photon latency. Missing/invalid visual observations are reported separately.",
            "HTTP wait includes tunnel, WDA routing and XCTest; status time is not subtracted to invent a server breakdown."]}


class Tests(unittest.TestCase):
    def test_earliest_actual_display_and_dropped_present(self):
        record={"generated_input_qpc":90,"visual":{"events":[
            {"present_start_qpc":100,"present_end_qpc":105,"swap_chain":1},
            {"present_start_qpc":110,"present_end_qpc":115,"swap_chain":1}]}}
        result=correlate(record,[{"qpc":101,"chain":1,"display":None},{"qpc":111,"chain":1,"display":120}],
                         [101,111],1000)
        self.assertEqual(result["generated_input_to_result_display_ms"],30)
        self.assertEqual(result["matched_frames"],2)

    def test_change_before_input_is_not_counted(self):
        result=correlate({"generated_input_qpc":90,"visual":{"events":[
            {"present_start_qpc":80,"present_end_qpc":85,"swap_chain":1}]}},[],[],1000)
        self.assertFalse(result["available"])

    def test_no_display_is_not_http_success(self):
        result=correlate({"generated_input_qpc":90,"visual":{"events":[
            {"present_start_qpc":100,"present_end_qpc":105,"swap_chain":1}]}},
            [{"qpc":101,"chain":1,"display":None}],[101],1000)
        self.assertFalse(result["available"])

    def test_one_pixel_render_is_not_visual_acceptance(self):
        result=correlate({"generated_input_qpc":90,"visual":{"events":[
            {"present_start_qpc":100,"present_end_qpc":105,"swap_chain":1,
             "render_width":1,"render_height":1}]}},
            [{"qpc":101,"chain":1,"display":110}],[101],1000)
        self.assertFalse(result["available"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("results", nargs="?", type=Path)
    parser.add_argument("--etw", type=Path)
    parser.add_argument("--test", action="store_true")
    parser.add_argument("--invalid-reason", help="Preserve statistics but explicitly invalidate acceptance (e.g. standby)")
    args = parser.parse_args()
    if args.test:
        unittest.main(argv=[__file__])
        raise SystemExit()
    if not args.results:
        parser.error("Provide results.json or --test")
    output = analyze(args.results, args.etw)
    output["measurement_condition_error"] = args.invalid_reason
    output["valid_for_tap_reliability"] = not args.invalid_reason and output["complete"] and all(
        s["attempts"] >= 100 and s["verified_pass"] == s["attempts"]
        for s in output["summary"])
    output["valid_for_visual_comparison"] = output["valid_for_tap_reliability"] and all(
        s["visual_coverage"] == s["attempts"] for s in output["summary"])
    # The original aggregate gate still requires the complete visual measurement.
    output["valid_for_acceptance"] = output["valid_for_visual_comparison"]
    path = args.results.with_suffix(".analysis.json")
    path.write_text(json.dumps(output, indent=2, allow_nan=False), encoding="utf-8")
    print(json.dumps(output["summary"], indent=2, allow_nan=False))
