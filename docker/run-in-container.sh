#!/bin/sh
# Builds the harness and runs the Vulkan (lavapipe) and GL (llvmpipe) matrices under Xvfb.
# Usage inside the container: run-in-container.sh <run-name-prefix>
set -eu
prefix=${1:-docker-$(date +%Y%m%d-%H%M%S)}
cargo build --release --locked
bin="$CARGO_TARGET_DIR/release/slint-wgpu-worker-bench"
lvp_icd=$(ls /usr/share/vulkan/icd.d/lvp_icd.*.json | head -1)
status=0

xvfb-run -a -s "-screen 0 1280x1024x24" python3 scripts/run-matrix.py \
    --out "runs/$prefix-vulkan-lavapipe" --binary "$bin" \
    --env WGPU_BACKEND=vulkan --env "VK_DRIVER_FILES=$lvp_icd" || status=$?

xvfb-run -a -s "-screen 0 1280x1024x24" python3 scripts/run-matrix.py \
    --out "runs/$prefix-gl-llvmpipe" --binary "$bin" \
    --env WGPU_BACKEND=gl --env LIBGL_ALWAYS_SOFTWARE=1 --env GALLIUM_DRIVER=llvmpipe || status=$?

exit "$status"
