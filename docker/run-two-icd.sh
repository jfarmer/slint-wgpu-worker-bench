#!/bin/sh
# Runs the matrix with two Vulkan ICDs, lavapipe and SwiftShader, in both loader orders,
# with WGPU_POWER_PREF=low and with the loader's device sorting disabled.
# Then compares selection across the runs, and records lavapipe's device counters.
# Usage inside the two-ICD container: run-two-icd.sh <run-name-prefix>
set -eu
prefix=${1:-two-icd-$(date +%Y%m%d-%H%M%S)}
cargo build --release --locked
bin="$CARGO_TARGET_DIR/release/slint-wgpu-worker-bench"
lvp=$(ls /usr/share/vulkan/icd.d/lvp_icd*.json | head -1)
test -n "$lvp" || { echo "lavapipe ICD not found" >&2; exit 4; }
ss=/opt/swiftshader/vk_swiftshader_icd.json
scenarios="single multi-window failure-limits abandon-before-device probe-after-device"
status=0

matrix() {
    name=$1; shift
    xvfb-run -a -s "-screen 0 1280x1024x24" python3 scripts/run-matrix.py \
        --out "runs/$prefix-$name" --binary "$bin" --env WGPU_BACKEND=vulkan "$@" || status=$?
}

matrix lvp-ss --scenarios $scenarios --env "VK_DRIVER_FILES=$lvp:$ss"
matrix ss-lvp --scenarios $scenarios --env "VK_DRIVER_FILES=$ss:$lvp"
matrix lvp-ss-low --scenarios $scenarios --env "VK_DRIVER_FILES=$lvp:$ss" --env WGPU_POWER_PREF=low
matrix ss-lvp-low --scenarios $scenarios --env "VK_DRIVER_FILES=$ss:$lvp" --env WGPU_POWER_PREF=low
matrix lvp-ss-unsorted --scenarios $scenarios --env "VK_DRIVER_FILES=$lvp:$ss" --env VK_LOADER_DISABLE_SELECT=1
matrix ss-lvp-unsorted --scenarios $scenarios --env "VK_DRIVER_FILES=$ss:$lvp" --env VK_LOADER_DISABLE_SELECT=1
matrix lvp-counters --scenarios single --env "VK_DRIVER_FILES=$lvp"
matrix lvp-no-indirect-validation --scenarios single --env "VK_DRIVER_FILES=$lvp" --env WGPU_VALIDATION_INDIRECT_CALL=0

for group in "lvp-ss ss-lvp" "lvp-ss-low ss-lvp-low" "lvp-ss-unsorted ss-lvp-unsorted"; do
    dirs=""
    for n in $group; do dirs="$dirs runs/$prefix-$n"; done
    set -- $group
    python3 scripts/compare-runs.py $dirs --out "runs/$prefix-$1.cross-order.md" > /dev/null || status=$?
done
exit "$status"
