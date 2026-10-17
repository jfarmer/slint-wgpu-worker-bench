#!/bin/sh
# Second macOS pass: debug builds, where objc2 tracks autorelease pools in a thread-local,
# and a traced set whose worker delay puts the exit inside device preparation.
# Usage: run-macos-2.sh <build-dir>, as for run-macos-1.sh.
set -eu
B=$1
R=$(cd "$(dirname "$0")" && pwd)
run() { echo "## $1"; "$R/run-macos-set.sh" "$R/$@"; }
run macos-traced-exit-after-backend-delay-50-70 "$B/target-traced/release/slint-dx12-probe" exit-after-backend 200 SLINT_TRACE_WORKER_DELAY_MIN_MS=50 SLINT_TRACE_WORKER_DELAY_MAX_MS=70
run macos-debug-control-97066a9e8-exit-immediately "$B/target-control/debug/slint-dx12-probe" exit-immediately 200
run macos-debug-fixes-exit-immediately "$B/target-fixes/debug/slint-dx12-probe" exit-immediately 200
run macos-debug-control-97066a9e8-exit-after-backend "$B/target-control/debug/slint-dx12-probe" exit-after-backend 200
run macos-debug-fixes-exit-after-backend "$B/target-fixes/debug/slint-dx12-probe" exit-after-backend 200
