#!/bin/sh
# Classifies each run of a traced set by the worker's state when main released the claim.
# Usage: classify-trace.sh <set-dir>. Output: counts per class, written to <set-dir>/classification.txt.
awk '
/^=== run/ { prep = fin = delay = destroyed = 0 }
/slint-wgpu-trace: worker delay_ms=/ { delay = 1 }
/slint-wgpu-trace: worker preparing/ { prep = 1 }
/slint-wgpu-trace: worker finished/ { fin = 1 }
/slint-wgpu-trace: worker destroyed unclaimed result/ { destroyed = 1 }
/^exit_code=/ {
  if (fin) state = "finished-before-release"
  else if (prep) state = "exit-during-preparation"
  else if (delay) state = "exit-during-delay"
  else state = "no-worker-line"
  print state, "destroyed_line=" destroyed, $0
}' "$1/runs.log" | sort | uniq -c > "$1/classification.txt"
cat "$1/classification.txt"
