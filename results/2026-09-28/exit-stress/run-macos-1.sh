#!/bin/sh
# Runs every macOS exit-stress set, 200 runs each, one after another.
# Usage: run-macos-1.sh <build-dir>, where <build-dir> holds target-fixes, target-control and
# target-traced from the builds described in README.md.
set -eu
B=$1
R=$(cd "$(dirname "$0")" && pwd)
F=$B/target-fixes/release/slint-dx12-probe
C=$B/target-control/release/slint-dx12-probe
T=$B/target-traced/release/slint-dx12-probe
run() { echo "## $1"; "$R/run-macos-set.sh" "$R/$@"; }
run macos-fixes-exit-immediately "$F" exit-immediately 200
run macos-control-97066a9e8-exit-immediately "$C" exit-immediately 200
run macos-fixes-exit-after-backend "$F" exit-after-backend 200
run macos-control-97066a9e8-exit-after-backend "$C" exit-after-backend 200
run macos-fixes-process-exit-after-backend "$F" process-exit-after-backend 200
run macos-fixes-exit-after-prepared "$F" exit-after-prepared 200
run macos-traced-exit-immediately "$T" exit-immediately 200
run macos-traced-exit-after-backend-delay "$T" exit-after-backend 200 SLINT_TRACE_WORKER_DELAY_MAX_MS=80
