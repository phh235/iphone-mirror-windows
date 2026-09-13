"""Offline per-PTS spike correlation. Never alters timestamps or source trace."""
import argparse
import bisect
from collections import Counter, defaultdict
import csv
import json
from pathlib import Path
import runpy

core = runpy.run_path(str(Path(__file__).with_name("analyze-usb-pacing.py")))
stats = core["distribution"]

SPAN_NAMES = {8: "dxgi_output_lookup", 9: "staging_creation", 10: "gpu_copy_submit",
              11: "readback_map", 12: "cpu_allocation", 13: "cpu_memcpy",
              14: "mft_process_output", 15: "renderer_upload", 18: "usb_read_wait",
              19: "packet_batch_parse", 20: "annex_b", 21: "mft_input_buffer",
              22: "mft_process_input", 23: "gpu_handoff_submit",
              25: "gpu_import", 26: "gpu_mutex_acquire", 27: "gpu_mutex_release_flush"}


def correlate(folder, case, frequency, pid):
    with (folder/case["trace"]).open(encoding="utf-8-sig", newline="") as stream:
        events = sorted(({k: int(v) for k, v in row.items()} for row in csv.DictReader(stream)),
                        key=lambda e: e["qpc"])
    begin, end = case["qpc_start"], case["qpc_end"]
    inside = lambda t: begin <= t < end
    subset = lambda kind: [e for e in events if e["kind"] == kind]
    first = core["first_by_pts"]
    source = first(subset(1))
    decoded = first(subset(3))
    accepted = first([e for e in subset(4) if e["b"] == 0])
    decode_start = first(subset(17))
    inferred_decode_start = not bool(decode_start)
    if inferred_decode_start:
        for e in subset(2):
            decode_start.setdefault(e["pts"], e["qpc"]-e["a"]*frequency/1e9)
    rendered = first(subset(16))
    etw = core["displayed"](folder/(case["case"]+".presentmon.csv"), events,
                            begin, end, frequency, pid, include_frames=True)
    if not etw["available"]:
        raise ValueError("Cannot attribute display spikes without matched ETW")
    display = etw.pop("frame_displays")
    ordered = sorted(display, key=display.get)
    source_pts = sorted(source)
    phases = [("source_pts", {p: p*frequency/10_000_000 for p in source}),
              ("usb_sample_observed", source), ("decode_input", decode_start),
              ("decoded_mailbox_publish", decoded), ("render_start", rendered),
              ("present_accepted", accepted), ("windows_display", display)]
    spikes = []
    for previous, current in zip(ordered, ordered[1:]):
        if not inside(display[current]):
            continue
        interval = (display[current]-display[previous])*1000/frequency
        if interval <= 25:
            continue
        values = {name: (mapping[current]-mapping[previous])*1000/frequency
                  for name, mapping in phases if previous in mapping and current in mapping}
        intervening = max(0, bisect.bisect_left(source_pts, current)-bisect.bisect_right(source_pts, previous))
        first_crossing = {}
        for threshold in (25, 40, 50):
            if interval <= threshold:
                continue
            first_crossing[str(threshold)] = ("intervening_source_frames_not_displayed" if intervening
                else next((name for name, value in values.items() if value > threshold), "unknown"))
        row = {"pts": current, "previous_displayed_pts": previous, "display_qpc": display[current],
               "display_interval_ms": interval, "intervals_ms": values,
               "intervening_source_frames": intervening, "first_crossing": first_crossing,
               "stage_expansion_ms": {b: values[b]-values[a] for a,b in zip(values,list(values)[1:])}}
        if current in source:
            row["receive_to_display_ms"] = (display[current]-source[current])*1000/frequency
        spikes.append(row)
    stage_time = {}
    for kind, name in SPAN_NAMES.items():
        records = [e for e in subset(kind) if inside(e["qpc"]) and e["a"] > 0]
        stage_time[name] = {"count": len(records),
                            "duration": stats([(e["qpc"]-e["a"])*1000/frequency for e in records])}
    retries = defaultdict(list)
    for e in subset(4):
        retries[e["pts"]].append(e)
    gaps, success_duplicates, retry_frames = [], 0, 0
    for records in retries.values():
        relevant = [e for e in records if inside(e["qpc"])]
        if not relevant:
            continue
        success_duplicates += max(0, sum(e["b"] == 0 for e in relevant)-1)
        retry_frames += int(any((e["b"] & 0xffffffff) == 0x887a000a for e in relevant))
        gaps.extend((b["a"]-a["qpc"])*1000/frequency for a,b in zip(records, records[1:])
                    if inside(b["qpc"]) and (a["b"] & 0xffffffff) == 0x887a000a)
    thresholds = {}
    for threshold in (25,40,50):
        selected = [s for s in spikes if s["display_interval_ms"] > threshold]
        thresholds[str(threshold)] = {"display_spikes":len(selected),
            "first_crossing":dict(Counter(s["first_crossing"][str(threshold)] for s in selected))}
    by_frame = []
    for pts, received in source.items():
        if not inside(received):
            continue
        row = {"pts": pts, "received_qpc": received}
        for name, mapping in phases[2:]:
            row[name+"_qpc"] = mapping.get(pts)
        by_frame.append(row)
    path = folder/(case["case"]+".spikes.json")
    path.write_text(json.dumps({"case":case["case"],"thresholds_ms":thresholds,
        "spikes":spikes,"frames":by_frame},indent=2),encoding="utf-8")
    return {"case":case["case"],"thresholds_ms":thresholds,"substage_timings_ms":stage_time,
        "gpu_handoff_published":sum(e["b"] == 1 for e in subset(23) if inside(e["qpc"])),
        "gpu_consumer_success":sum(e["a"] == 1 for e in subset(24) if inside(e["qpc"])),
        "gpu_consumer_fallback":sum(e["a"] == 0 for e in subset(24) if inside(e["qpc"])),
        "gpu_import_cache_hits":sum(e["b"] == 1 for e in subset(25) if inside(e["qpc"])),
        "receive_to_display_ms": stats([(display[p]-t)*1000/frequency for p,t in source.items()
                                        if inside(t) and p in display]),
        "retry_gap_ms":stats(gaps),"frames_with_present_retry":retry_frames,
        "successful_same_pts_representations":success_duplicates,
        "decode_start_inferred_from_existing_decode_duration":inferred_decode_start,
        "memory_samples":[s["memory"] for s in case["cpu_samples"] if "memory" in s],
        "detail_file":path.name}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("suite",type=Path)
    args=parser.parse_args()
    suite=json.loads(args.suite.read_text(encoding="utf-8-sig"))
    report={"base_commit":suite["base_commit"],"notes":[
        "First threshold crossing compares the same displayed-frame pair; it is descriptive, not exclusive causation.",
        "Skipped source frames are separated before attribution; intervals at different stages may contract or expand.",
        "Baseline decode input is inferred when kind 17 is absent. GPU-copy-submit timing is not GPU completion.",
        "No phone clock synchronization; receive-to-display excludes capture, encoder and USB time before receipt."],
        "cases":[correlate(args.suite.parent,c,suite["qpc_frequency"],suite["pid"]) for c in suite["cases"]]}
    output=args.suite.with_suffix(".spikes.json")
    output.write_text(json.dumps(report,indent=2),encoding="utf-8")
    print(json.dumps(report,indent=2))
