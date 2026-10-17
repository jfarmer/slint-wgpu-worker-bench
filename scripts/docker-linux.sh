#!/bin/sh
# Runs the Linux software-driver matrices locally in Docker. Needs a running Docker daemon.
# Usage: scripts/docker-linux.sh [run-name-prefix] [--platform linux/amd64]
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
prefix=${1:-docker-$(date +%Y%m%d-%H%M%S)}
shift $(( $# > 0 ? 1 : 0 ))
docker build "$@" -t slint-wgpu-worker-bench-linux "$root/docker"
docker run --rm "$@" -v "$root:/work" slint-wgpu-worker-bench-linux "$prefix"
