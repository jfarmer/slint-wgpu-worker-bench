# Linux software-driver environment for the matrix: Mesa lavapipe (Vulkan) and llvmpipe (GL)
# from Ubuntu 24.04, under Xvfb. Works on amd64 and arm64 hosts, unlike wgpu's x86_64-only
# gfx-rs/ci-build Mesa that the GitHub workflow uses.
FROM ubuntu:24.04

ARG RUST_TOOLCHAIN=1.96.0
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl build-essential pkg-config python3 \
        xvfb xauth libx11-xcb1 libxcursor1 libxrandr2 libxi6 libxkbcommon-x11-0 \
        mesa-vulkan-drivers libvulkan1 vulkan-tools \
        libgl1-mesa-dri libegl1 libegl-mesa0 libgl1 mesa-utils \
    && rm -rf /var/lib/apt/lists/*

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain "$RUST_TOOLCHAIN"
ENV PATH=/root/.cargo/bin:$PATH
# Keeps container builds apart from the host's target/ in the mounted checkout.
ENV CARGO_TARGET_DIR=/work/target/docker-linux

WORKDIR /work
ENTRYPOINT ["/work/docker/run-in-container.sh"]
