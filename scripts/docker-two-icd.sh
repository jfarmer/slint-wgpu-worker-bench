#!/bin/sh
# Runs docker/run-two-icd.sh: lavapipe and SwiftShader under one Vulkan loader.
# Needs a running Docker daemon. Building SwiftShader is slow the first time.
# Usage: scripts/docker-two-icd.sh [run-name-prefix]
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
prefix=${1:-two-icd-$(date +%Y%m%d-%H%M%S)}
docker build -t slint-wgpu-worker-bench-linux "$root/docker"
docker build -f "$root/docker/Dockerfile.two-icd" -t slint-wgpu-worker-bench-two-icd "$root/docker"
docker run --rm -v "$root:/work" --entrypoint /work/docker/run-two-icd.sh slint-wgpu-worker-bench-two-icd "$prefix"
