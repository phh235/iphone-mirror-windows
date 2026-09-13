"""Offline analysis only: native QPC observations + PID-filtered PresentMon v1 CSV.
Never uses render-attempt counts as unique presented/displayed FPS.
"""
import argparse
import bisect
import csv
import json
import math
from pathlib import Path
from statistics import fmean
import unittest
import tempfile


def distribution(values, frame_intervals=False):
    values = sorted(values)
    if not values:
        return None
    percentile = lambda p: values[max(0, math.ceil(len(values) * p) - 1)]
    worst = values[-max(1, math.ceil(len(values) * 0.01)):]
    result = {"samples": len(values), "average_ms": fmean(values),
              "p50_ms": percentile(.5), "p95_ms": percentile(.95),
              "p99_ms": percentile(.99), "max_ms": values[-1]}
    if frame_intervals:
        result["one_percent_low_fps"] = 1000 / fmean(worst) if fmean(worst) > 0 else None
    return result


def intervals(times, frequency):
    times = sorted(times)
    return distribution([(b-a)*1000/frequency for a, b in zip(times, times[1:])], frame_intervals=True)


def first_by_pts(events):
    result = {}
    for event in sorted(events, key=lambda e: e["qpc"]):
        result.setdefault(event["pts"], event["qpc"])
    return result


def number(row, name):
    text = row.get(name, "")
    try:
        value = float(text)
        return value if math.isfinite(value) else None
    except (ValueError, TypeError):
        return None


def displayed(csv_path, events, begin, end, frequency, pid, include_frames=False):
    if not csv_path.exists():
        return {"available": False, "reason": "PresentMon CSV absent"}
    with csv_path.open(encoding="utf-8-sig", newline="") as stream:
        reader = csv.DictReader(stream)
        fields = reader.fieldnames or []
        rows = list(reader)
    qpc_field = next((k for k in fields if k.lower() in ("qpctime", "qpc", "presentstartqpc")), None)
    if qpc_field is None:
        return {"available": False, "reason": "No v1 Present-start QPC column", "columns": fields}
    calls = sorted((e for e in events if e["kind"] == 4), key=lambda e: e["a"])
    starts = [e["a"] for e in calls]
    used = set()
    matches = []
    matched_success_pts = set()
    dropped_success_pts = set()
    relevant = 0
    unmatched = 0
    for row in rows:
        if str(row.get("ProcessID")) != str(pid):
            continue
        try:
            at = int(row[qpc_field])
            chain = int(row["SwapChainAddress"], 0)
        except (ValueError, KeyError):
            continue
        if at < events[0]["qpc"] or at > events[-1]["qpc"]:
            continue
        relevant += 1
        where = bisect.bisect_left(starts, at)
        candidates = [i for i in range(max(0, where-3), min(len(calls), where+3))
                      if i not in used and calls[i]["c"] == chain
                      and calls[i]["a"]-frequency*.0001 <= at <= calls[i]["qpc"]+frequency*.0001]
        if not candidates:
            unmatched += 1
            continue
        index = min(candidates, key=lambda i: abs(calls[i]["a"]-at))
        used.add(index)
        event = calls[index]
        until = number(row, "msUntilDisplayed")
        dropped = str(row.get("Dropped", "")).lower() in ("1", "true")
        if event["b"] == 0:
            matched_success_pts.add(event["pts"])
            if dropped:
                dropped_success_pts.add(event["pts"])
        if event["b"] == 0 and not dropped and until is not None and until >= 0:
            matches.append((event["pts"], at+until*frequency/1000))
    first = {}
    for pts, at in sorted(matches, key=lambda pair: pair[1]):
        first.setdefault(pts, at)
    visible = [at for at in first.values() if begin <= at < end]
    accepted_all = first_by_pts([e for e in calls if e["b"] == 0])
    accepted = {pts for pts, at in accepted_all.items() if begin <= at < end}
    not_displayed = accepted - first.keys()
    valid = relevant > 0 and unmatched/relevant <= .01
    result = {"available": valid, "reason": None if valid else "Missing or insufficiently matched ETW evidence",
            "etw_rows_in_trace": relevant, "matched_rows": relevant-unmatched,
            "unmatched_rows": unmatched, "unique_displayed_frames": len(visible),
            "unique_displayed_fps": len(visible)*frequency/(end-begin) if valid else None,
            "display_interval": intervals(visible, frequency) if valid else None,
            "accepted_frames_not_observed_displayed_by_tail": len(not_displayed) if valid else None,
            "not_displayed_with_etw_dropped_evidence": len(not_displayed & dropped_success_pts) if valid else None,
            "accepted_frames_without_matched_etw_row": len(accepted - matched_success_pts),
            "accepted_frames_first_displayed_after_window": sum(first.get(pts, -1) >= end for pts in accepted),
            "definition": "First ETW display of each source PTS; repeated presentation of that PTS is excluded"}
    if include_frames:
        result["frame_displays"] = first
    return result


def analyze_case(folder, case, frequency, pid):
    with (folder/case["trace"]).open(newline="", encoding="utf-8-sig") as stream:
        events = [{k: int(v) for k, v in row.items()} for row in csv.DictReader(stream)]
    events.sort(key=lambda e: e["qpc"])
    begin, end = case["qpc_start"], case["qpc_end"]
    seconds = (end-begin)/frequency
    inside = lambda t: begin <= t < end
    subset = lambda kind: [e for e in events if e["kind"] == kind]
    source, decoded, presents = subset(1), subset(3), subset(4)
    source_map, decoded_map = first_by_pts(source), first_by_pts(decoded)
    accepted_map = first_by_pts([e for e in presents if e["b"] == 0])
    selected = lambda m: {p: t for p, t in m.items() if inside(t)}
    src, dec, accepted = selected(source_map), selected(decoded_map), selected(accepted_map)
    epochs = set(e["a"] for e in source if inside(e["qpc"]))
    overflows = [e["b"] for e in subset(100)]
    unknown = len([e for e in subset(7) if inside(e["qpc"])])
    replacements = [e for e in decoded if inside(e["qpc"]) and e["a"] != 0 and e["a"] != e["pts"]]
    before_present = [e for e in replacements if e["a"] not in accepted_map or accepted_map[e["a"]] > e["qpc"]]
    never_presented = set(e["a"] for e in before_present if e["a"] not in accepted_map and e["a"] in dec)
    q = subset(6)
    prior_drop = max((e["b"] for e in q if e["qpc"] < begin), default=0)
    end_drop = max((e["b"] for e in q if e["qpc"] < end), default=prior_drop)
    counted_calls = [e for e in presents if inside(e["qpc"])]
    wsd = sum((e["b"] & 0xffffffff) == 0x887a000a for e in counted_calls)
    etw = displayed(folder/(case["case"]+".presentmon.csv"), events, begin, end, frequency, pid)
    return {"case": case["case"], "description": case["description"], "seconds": seconds,
            "valid_native": len(epochs) <= 1 and not any(overflows) and unknown == 0 and not case["format_changed"],
            "event_overflow": max(overflows, default=None), "unclocked_samples": unknown,
            "source_epochs": len(epochs), "format_changed": case["format_changed"],
            "unique_source_frames_received": len(src), "unique_source_fps_received": len(src)/seconds,
            "unique_decoded_frames": len(dec), "decoded_fps": len(dec)/seconds,
            "unique_accepted_present_frames": len(accepted), "unique_accepted_present_fps": len(accepted)/seconds,
            "source_arrival_interval": intervals(src.values(), frequency),
            "source_pts_interval": intervals(src.keys(), 10_000_000) if len(epochs) <= 1 else None,
            "decode_output_interval": intervals(dec.values(), frequency),
            "accepted_present_interval": intervals(accepted.values(), frequency),
            "actual_display": etw,
            "source_not_decoded_by_tail": len(set(src)-set(decoded_map)),
            "decoded_not_accepted_by_tail": len(set(dec)-set(accepted_map)),
            "encoded_samples_dropped": end_drop-prior_drop,
            "mailbox_replacements": len(replacements),
            "mailbox_overwritten_before_successful_present": len(before_present),
            "overwritten_but_presented_later": sum(e["a"] in accepted_map for e in before_present),
            "overwritten_never_presented_by_tail": len(never_presented),
            "present_calls": len(counted_calls), "was_still_drawing": wsd,
            "was_still_drawing_percent": 100*wsd/len(counted_calls) if counted_calls else None,
            "present_errors_other": sum(e["b"] not in (0, -2005270518) for e in counted_calls),
            "decode_processing": distribution([e["a"]/1e6 for e in subset(2) if inside(e["qpc"])]),
            "renderer_processing": distribution([e["a"]/1e6 for e in subset(5) if inside(e["qpc"])]),
            "cpu_average_percent": case["cpu_average_percent"],
            "cpu_peak_sample_percent": max((s["cpu_percent"] for s in case["cpu_samples"]), default=None),
            "maximum_encoded_queue_depth": max((e["a"] for e in q if inside(e["qpc"])), default=None),
            "notes": ["FPS uses the measured QPC window; counters are not pixel-content comparisons.",
                      "Mailbox replacement before Present can be benign when the renderer already holds the old frame.",
                      "Not decoded/presented by the 2s tail is an observed absence, not an unconditional transport-loss diagnosis.",
                      "Processing times are CPU/API wall time, not GPU timestamp duration."]}


class Tests(unittest.TestCase):
    def test_percentiles_and_one_percent_low(self):
        result = distribution([10.0]*99+[100.0], frame_intervals=True)
        self.assertEqual(result["average_ms"], 10.9)
        self.assertEqual(result["p99_ms"], 10.0)
        self.assertEqual(result["one_percent_low_fps"], 10.0)

    def test_processing_time_does_not_claim_fps(self):
        self.assertNotIn("one_percent_low_fps", distribution([1.0, 2.0]))

    def test_repeated_pts_never_increase_unique_count(self):
        self.assertEqual(first_by_pts([{"pts": 10, "qpc": 2}, {"pts": 10, "qpc": 1},
                                      {"pts": 11, "qpc": 3}]), {10: 1, 11: 3})

    def test_interval_units_and_empty(self):
        self.assertIsNone(intervals([100], 1000))
        self.assertEqual(intervals([100, 120, 140], 1000)["p95_ms"], 20)

    def test_display_correlation_excludes_repeated_pts_and_dropped_rows(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/"etw.csv"
            path.write_text("ProcessID,SwapChainAddress,QPCTime,Dropped,msUntilDisplayed\n"
                            "123,0x1234,10020,0,5\n123,0x1234,10050,0,5\n"
                            "123,0x1234,10060,0,5\n123,0x1234,10080,1,NA\n",encoding="utf-8")
            events=[{"qpc":10000,"kind":0}]
            for at,pts in [(10020,1),(10050,1),(10060,2),(10080,3)]:
                events.append({"qpc":at+1,"kind":4,"pts":pts,"a":at,"b":0,"c":0x1234})
            events.append({"qpc":10100,"kind":100})
            result=displayed(path,events,10000,10100,1000,123)
            self.assertTrue(result["available"])
            self.assertEqual(result["unique_displayed_frames"],2)
            self.assertEqual(result["display_interval"]["average_ms"],40)
            self.assertEqual(result["accepted_frames_not_observed_displayed_by_tail"],1)
            self.assertEqual(result["not_displayed_with_etw_dropped_evidence"],1)
            self.assertEqual(result["accepted_frames_without_matched_etw_row"],0)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("suite", nargs="?", type=Path)
    parser.add_argument("--test", action="store_true")
    args = parser.parse_args()
    if args.test:
        unittest.main(argv=[__file__])
    elif args.suite:
        suite = json.loads(args.suite.read_text(encoding="utf-8-sig"))
        report = {"source": str(args.suite), "base_commit": suite["base_commit"], "view": suite["view"],
                  "vsync": suite["vsync"], "source_dimensions": suite["source"],
                  "hook_ns": suite["recorder_average_hook_ns"],
                  "one_percent_low_definition": "1000 divided by mean of slowest ceil(1%) unique frame intervals",
                  "cases": [analyze_case(args.suite.parent, case, suite["qpc_frequency"], suite["pid"]) for case in suite["cases"]]}
        path = args.suite.with_suffix(".analysis.json")
        path.write_text(json.dumps(report, indent=2, ensure_ascii=False, allow_nan=False), encoding="utf-8")
        print(json.dumps(report, indent=2, ensure_ascii=False, allow_nan=False))
    else:
        parser.error("Provide suite.json or --test")
