# Slint WGPU Worker Bench

A behavioral harness for one question.
Does preparing the WGPU instance, adapter and device on a worker thread, before the Winit event loop starts and before any surface exists, behave like Slint's ordinary surface-first initialization?

Slint's `wgpu-worker-preparation` branch does this on macOS only, and only when the effective backend mask is exactly `METAL`.
This harness copies Slint's WGPU 30 settings into a standalone binary and runs the same scenarios under four strategies.
It then diffs the results against the ordinary path on any platform and driver.

This is a correctness harness.
It records phase timings for context, but its runs are short smoke runs and support no performance claims.

## Quick Start

Requirements: Rust 1.96 or newer, Python 3.12 or newer (standard library only), and Git.
Records were made with Rust 1.96.0 on macOS and 1.98.1 on Windows.

### macOS

```sh
python3 scripts/run-matrix.py --out runs/metal
python3 -m unittest discover -s tests -v
```

### Windows

Run from the logged-on user's desktop session:

```bat
py -3.12 scripts\run-matrix.py --out runs\win-default
py -3.12 scripts\run-matrix.py --out runs\win-dx12 --env WGPU_BACKEND=dx12
```

Over SSH or from a service, windows land in session 0.
DXGI rejects their swap chains there, so `Surface::configure` fails with `Invalid surface` under every strategy.
Run through a scheduled task in the interactive session instead:

```bat
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\windows\run-interactive.ps1 -Name win-dx12 -Extra "--env WGPU_BACKEND=dx12"
```

The script registers a one-shot task with `schtasks /Create /IT /RU %USERNAME%`, runs it, waits for it to finish, and deletes it.
The user must be logged on.
Output lands in `runs\<Name>\`, with the console log in `runs\<Name>.console.log`.

The DX12 hazard runs need the D3D12 debug layer for `--profile debug` and `WGPU_VALIDATION=1`.
Install it as an administrator with `DISM /Online /Add-Capability /CapabilityName:Tools.Graphics.DirectX~~~~0.0.1.0`.
Without it, validation runs still pass, but the records show `debug_device: false`.

```bat
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\windows\run-interactive.ps1 -Name dx12-hazards -Extra "--profile debug --suite base hazards --env WGPU_BACKEND=dx12"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\windows\run-interactive.ps1 -Name dx12-control -Extra "--suite control --env WGPU_BACKEND=dx12 --env WGPU_VALIDATION=0"
```

### Linux

With a running Docker daemon:

```sh
scripts/docker-linux.sh my-prefix                           # native architecture
scripts/docker-linux.sh my-prefix --platform linux/amd64    # x86_64 under emulation
```

The container uses Ubuntu 24.04's Mesa: lavapipe for Vulkan and llvmpipe for GL, under Xvfb.
On a Linux host with those packages, run `run-matrix.py` under `xvfb-run -a`, as `docker/run-in-container.sh` does.

`scripts/docker-two-icd.sh my-prefix` adds SwiftShader, built from source, so the Vulkan loader sees two ICDs.
It runs both ICD orders, `WGPU_POWER_PREF=low`, and `VK_LOADER_DISABLE_SELECT=1`, then compares selection across the orders.
Building SwiftShader's bundled LLVM needs the Docker VM's memory, so the build uses 4 jobs.
It hasn't completed yet (see [results/README.md](results/README.md)).

### CI

`.github/workflows/matrix.yml` runs the matrix on GitHub-hosted runners.
It covers Windows DX12 on WARP with the default mask and with `WGPU_BACKEND=dx12`, and Linux Vulkan on lavapipe and GL on llvmpipe under Xvfb.
An optional SwiftShader job runs on manual dispatch.
Driver setup follows [wgpu's CI at the pinned commit](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/.github/workflows/ci.yml): WARP 1.0.16.1 from NuGet, and Mesa 25.2.7 from `gfx-rs/ci-build` `build26`.
The workflow hasn't run yet.

## What It Does

The binary takes `--strategy` and `--scenario` and writes one JSON record per run.

Strategies:

- `ordinary`: create the instance, then the window surface, then select an adapter compatible with that surface, then the device.
  This is Slint's default path.
- `worker-device`: a `std::thread` started before `EventLoop::new` creates the instance, a surface-free adapter, and the device.
  The main thread creates the surface from that instance and checks `adapter.is_surface_supported` before configuring.
  If the check fails, it records the failure and falls back to surface-aware selection.
  This is the Slint branch's strategy, plus the check.
- `worker-instance`: the worker creates only the instance.
  The main thread does surface-aware selection.
- `worker-after-select`: the main thread does surface-aware selection, and a worker runs `request_device`.

Scenarios:

- `single`: one window.
- `multi-window`: N windows one after another, then N at once, all from one device.
- `abandon`: the worker blocks on a channel at `--abandon-at` (`before-adapter`, `before-device` or `after-device`).
  The main thread waits for it to arrive, releases it, and drops the handle.
  It then joins with a timeout and drops the result in an autorelease pool on Apple platforms.
  An ordinary initialization afterwards checks that the GPU is still usable.
- `failure`: forces `request_device` to fail with impossible limits (`--failure limits`), or uses an empty mask (`empty-mask`) or a mask with no compiled backend (`unsupported-mask`).
- `repeated`: creates and destroys a prepared device K times on one window.
- `join-block`: the worker blocks at `--abandon-at`, and the main thread waits in a plain join until a helper thread releases it after `--release-after-ms`.
  `--join-at before-event-loop` joins before `EventLoop::new`, and `after-window` joins with a shown window whose messages go unpumped.
- `early-exit`: the process calls `std::process::exit(7)` as soon as the worker enters instance, adapter or device creation (`--exit-at`), without joining it.

Options that combine with any scenario:

- `--probe P` runs a copy of Slint's `any_wgpu30_adapters_with_gpu` on the main thread.
  `concurrent` runs it right after the worker starts, as Slint's `new_suspended` does.
  `after-instance`, `after-adapter` and `after-device` wait for that worker stage first.
  `after-first-device` runs it once the first window's device exists, as a later window's instance would in ordinary Slint.
- `--on-probe-reject` decides what happens when the probe rejects WGPU because every adapter is a CPU adapter.
  `continue` renders anyway, as a GPU adapter passing the probe would.
  `keep` skips WGPU and keeps the prepared result until exit, as the Slint branch does.
  `drop` skips WGPU and releases the prepared result at once.
- `--probe-validation on` adds `VALIDATION` to the probe's instance only.
  With `WGPU_VALIDATION=0` it turns the D3D12 debug layer on after a device made without it exists, which is the positive control for the debug-layer hazard.

`run-matrix.py --suite` selects variant groups: `base` (the default), `probe`, `lifetime`, `control`, `hazards` (`probe` and `lifetime`), and `all`.
Several suites can be given at once.

Each record holds:

- platform, environment variables, and the requested, masked, and compiled backend masks
- the surface-free adapter enumeration in order, and per-adapter `is_surface_supported`
- the selected adapter, surface capabilities, and the configured format, present mode and alpha mode
- every error with its phase and a stable class, including `device-lost` from a lost-device callback
- WGPU log records: warnings and errors from any target, and debug records from `wgpu_hal` and `wgpu_core::instance`.
  The D3D12 debug layer's messages reach the log once a DX12 device exists under `VALIDATION`.
  `dx12_compiler` names the compiler `wgpu-hal` reported loading.
- the HAL counters of every new device right after `request_device`, such as `compute_pipelines`, which counts the indirect-validation pipelines
- on DX12, `GetDeviceRemovedReason`, whether the debug layer wraps the device, and its info queue, after the probe and after the first present
- on Windows, which of `dxcompiler.dll`, `d3dcompiler_47.dll`, `d3d12sdklayers.dll` and related DLLs are loaded, and where the search path finds them
- the Slint gates: `slint_would_prepare_on_worker` for the branch's `METAL` gate, and `dx12_gate_would_prepare` for the proposed `DX12` extension
- per-phase timings with their thread: instance, adapter, device, surface, configure, first pipeline, first frame submitted, present
- a rendered-output check: a fixed clear plus triangle in a 64×64 texture of the surface format, read back as an RGBA hash and four pixels
- one frame presented to the real surface, retried from `about_to_wait` until the window is visible

A watchdog thread exits with code 3 when a run exceeds `--timeout`, after writing the partial record.

After the deferred present, each kept window renders offscreen once more, and the record keeps that readback in `after_present`.

`scripts/run-matrix.py` runs strategies × scenario variants and writes `runs/<name>/<strategy>__<variant>.json`, a log per case, and `manifest.json` with the toolchain and environment.
`scripts/compare.py` then checks each worker strategy against `ordinary` for the same adapter, formats, present modes, pixel hashes, present results, and error class.
It also compares the selected device's features and limits, and the surface-free enumeration order.
It checks every record on its own: expected pixel colors, accepted surface configuration, gate points, abandon joins, the intended exit code within the watchdog, a device still usable after present, no removed device, and no logged or debug-layer errors.
A `join-block` join must last until the release, and an `early-exit` must happen inside the requested call.
The run passes, fails, or is `unavailable` when no adapter exists for the requested mask.

`scripts/compare-runs.py` compares runs made under different environments.
It lists each run's selection and gates, fails when strategies of one run disagree, and with `--same adapter device` fails when those facts differ between runs:

```sh
python3 scripts/compare-runs.py runs/win-dx12 runs/win-dx12-power-low --same adapter device
```

[docs/slint-correspondence.md](docs/slint-correspondence.md) maps every setting to the Slint function it copies.

## What It Establishes and What It Doesn't

It can establish, for a given driver and mask:

- whether surface-free and surface-aware selection pick the same adapter
- whether that adapter can present to the window, and with which format
- whether the rendered pixels and error classes match the ordinary path
- whether an abandoned or failing worker joins, releases its resources, and leaves the GPU usable
- whether Slint's main-thread probe, placed at each point of worker progress, removes or invalidates the worker's device
- whether a blocking join, or a process exit during a worker call, completes cleanly
- which DX12 compiler loads, and how many internal pipelines a new device builds

It can't establish:

- anything about performance, since runs are short and unrepeated
- physical-GPU behavior from software drivers: WARP, lavapipe, llvmpipe and SwiftShader expose one CPU adapter each
- hybrid-GPU routing, such as an integrated and a discrete GPU where the surface decides which can present
- Slint's own behavior end to end, since the harness copies Slint's settings rather than running Slint

## Results and Write-Ups

This repository preserves an experimental Slint change and its correctness evidence for future Slint and FemtoVG work.
The shutdown regressions described below were introduced by the local worker-preparation branches, not upstream Slint.
The public history starts from a sanitized snapshot; historical harness commit IDs in the records refer to the original local history.
`<slint-worktree>` and `<metal-evidence>` in the write-ups identify separate local checkouts, which are not included here.

[REPORT.md](REPORT.md) answers whether worker preparation can extend beyond Metal, starting with DX12.
[docs/PR.md](docs/PR.md) drafts the Metal-only PR and an optional DX12 follow-up.

[results/README.md](results/README.md) indexes the archived runs with their raw records.
That covers the harness matrix, the DX12 hazard runs, the Slint-integrated `slint-windows-*` runs, and `exit-stress`.
Archived records retain platform, driver and toolchain details needed to interpret the experiments, with personal paths redacted.
Raw logs, crash reports, local task wrappers, fresh runs, build outputs and assistant configuration are excluded from Git.
New result directories are ignored by default; review and sanitize any evidence before explicitly adding it with `git add -f`.

Research pages, each with an "Experiment Outcomes" section:

- [docs/research/dx12-worker-safety.md](docs/research/dx12-worker-safety.md): threading, lifetime and failure safety of a DX12 worker, and tests W1 to W10
- [docs/research/windows-mixed-default.md](docs/research/windows-mixed-default.md): whether any strategy selects exactly under Windows' default `VULKAN | DX12` mask
- [docs/research/pipeline-compilation-and-caching.md](docs/research/pipeline-compilation-and-caching.md): where shader and pipeline compilation happens on Metal, DX12 and Vulkan

[docs/slint-windows-integration.md](docs/slint-windows-integration.md) records Slint itself running with the DX12 gate, using the probe app in `slint-integration/`.

## Layout

```text
src/                Rust harness: Slint's settings and probe, the worker, rendering, diagnostics, and the JSON record
scripts/            run-matrix.py, compare.py, compare-runs.py, the Windows task wrapper, the Docker launchers
tests/              Unit tests for the comparison logic
docker/             Linux software-driver images, one with lavapipe and SwiftShader
.github/            CI matrix
docs/               Slint correspondence, research pages, the Windows integration write-up and PR drafts
slint-integration/  Slint probe app, trace patches and Windows scripts
results/            Archived runs with raw data
REPORT.md           Conclusions across all runs
```

`runs/` holds new output and is ignored by Git.
Harness code is MIT licensed.
