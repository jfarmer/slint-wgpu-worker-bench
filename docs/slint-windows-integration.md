# Slint on Windows with DX12 Device Preparation

This page records a Slint-integrated run of the DX12 extension of worker device preparation.
It runs Slint itself, with its adapter probe, renderer fallback chain and `resume`, rather than the harness's copy of Slint's settings.

Run date: 2026-09-28.
No timing claims are made.
The probe app prints phase times for orientation only, from single runs on WARP in a VM.

Every claim carries one label:

- **Source-established**: read from pinned source, with a file reference.
- **Documented**: stated in vendor or Rust documentation, with a link.
- **Experiment**: observed in a run below, with its log directory.
- **Hypothesis**: not yet checked.

## Revisions

Slint worktree `<slint-worktree>`, branch `wgpu-worker-dx12`, which starts at `97066a9e8`:

| Commit | Subject |
| --- | --- |
| `4b7fea77e` | winit: Release a prepared WGPU device when the adapter probe fails |
| `adde532c9` | winit: Prepare the default DX12 device during backend initialization |
| `61f2ce99f` | winit: Don't join the WGPU device worker from thread-local destructors |

The three builds compared on Windows:

| Variant | Slint tree | Worker on Windows |
| --- | --- | --- |
| `baseline` | `97066a9e8`, the macOS-only gate | never |
| `candidate` | `adde532c9` plus `slint-integration/trace.patch` | effective mask `DX12`, no validation flags |
| `fixed` | `61f2ce99f` plus `slint-integration/trace-fixed.patch` | same gate as `candidate` |

The trace patches add `eprintln!` lines prefixed `slint-wgpu-trace:` at the gate, at worker completion, and in `take`, `discard` and `Drop`.
They were applied to the VM copy only and aren't committed.
They prove which path ran.

## Setup

- Windows 11 ARM64 VM, Rust 1.98.1, target `aarch64-pc-windows-msvc`.
  The only adapter is WARP, "Microsoft Basic Render Driver", Dx12, `DeviceType::Cpu`.
  `C:\Windows\System32\d3d12SDKLayers.dll` is present, so the D3D12 debug layer was available.
- The Slint branch went to `<slint-worktree>` as a git bundle.
  The baseline is a worktree of it at `<slint-baseline>`.
- The probe app is `slint-integration/app`.
  It depends on Slint with `std`, `compat-1-18`, `backend-winit`, `renderer-femtovg-wgpu`, `renderer-software` and `unstable-wgpu-30`.
  Winit tries `femtovg-wgpu` first and falls back to `software` (`internal/backends/winit/lib.rs`, `try_create_window_with_fallback_renderer`).
  It builds in `release` with `panic = "abort"`, so `InstanceFlags::from_build_config()` is empty apart from `VALIDATION_INDIRECT_CALL`.
- For each window the app records the renderer, `Device::adapter_info()`, an FNV-1a hash of the device's limits and features, and an FNV-1a hash of `Window::take_snapshot`.
- Every case ran three times in the interactive session through a one-shot `schtasks /IT` task, since session 0 can't configure DXGI swap chains.

Reproduction commands, run from this repository's root over SSH on the VM.
Replace `C:\src\slint` with the checkout containing the experimental revisions above:

```bat
powershell -NoProfile -ExecutionPolicy Bypass -File .\slint-integration\windows\setup.ps1 -SlintPath C:\src\slint
powershell -NoProfile -ExecutionPolicy Bypass -File .\slint-integration\windows\run-interactive.ps1 -Name run1 -Variants "baseline candidate"
powershell -NoProfile -ExecutionPolicy Bypass -File .\slint-integration\windows\setup.ps1 -SlintPath C:\src\slint -Variants fixed
powershell -NoProfile -ExecutionPolicy Bypass -File .\slint-integration\windows\run-interactive.ps1 -Name run2 -Variants fixed
```

The scripts are in `slint-integration/windows/`.
`cases.cmd` lists the cases and `run-case.cmd` sets the environment and appends the exit code.

## Cases

| Case | Scenario | Environment |
| --- | --- | --- |
| a | one window | `WGPU_BACKEND=dx12 SLINT_WGPU_CPU=1` |
| b | one window | `SLINT_WGPU_CPU=1`, default mask |
| c | one window | `WGPU_BACKEND=dx12`, no `SLINT_WGPU_CPU` |
| d | one window | `WGPU_BACKEND=dx12 SLINT_WGPU_CPU=1 WGPU_VALIDATION=1` |
| e | a second window after the first renders | `WGPU_BACKEND=dx12 SLINT_WGPU_CPU=1` |
| f | `show`, then `hide`, then quit the event loop | `WGPU_BACKEND=dx12 SLINT_WGPU_CPU=1` |
| g | create a component and return from `main` without an event loop | `WGPU_BACKEND=dx12 SLINT_WGPU_CPU=1` |

Case g was added to cover quitting while preparation may still run.
In case f the window still resumes, so it consumes the prepared device and doesn't exercise that path.

## Results

Raw logs: `results/2026-09-28/slint-windows-run1/` for `baseline` and `candidate`, and `results/2026-09-28/slint-windows-run2/` for `fixed`.
Each log holds the app's `event=` lines, the trace lines, and `exit_code=`.

Every run of a case gave the same result in all three repetitions, except where noted.

| Case | Renderer and adapter, all variants | Snapshot FNV-1a, all variants | `candidate` trace | `fixed` trace | Exit code baseline / candidate / fixed |
| --- | --- | --- | --- | --- | --- |
| a | femtovg-wgpu, WARP, Dx12, Cpu | `320bcf3566a2544d` | spawned, consumed, finished on `slint-wgpu-device` | same | 0 / 0 / 0 |
| b | femtovg-wgpu, WARP, Dx12, Cpu | `320bcf3566a2544d` | declined, `VULKAN \| DX12` | same | 0 / 0 / 0 |
| c | software fallback | `dfe0d2812ba262f3` | spawned, discarded | spawned, discarded, worker destroyed unclaimed result | 0 / 0 / 0 |
| d | femtovg-wgpu, WARP, Dx12, Cpu | `320bcf3566a2544d` | declined, flags `VALIDATION \| VALIDATION_INDIRECT_CALL` | same | 0 / 0 / 0 |
| e | femtovg-wgpu, WARP, both windows | `320bcf3566a2544d`, `79bac0ad5465ee0d` | spawned, consumed once | same | 0 / 0 / 0 |
| f | no frame rendered | none | spawned, consumed | same | 0 / 0 / 0 |
| g | no window | none | spawned, discarded, then a panic | spawned, released on drop in 2 of 3 runs | 0 / **-1073740791** / 0 |

The limits and features hash was `51340666fc77a6cd` for every WGPU window in every variant.

### Findings

**Experiment.** In case a the candidate prepared the device on the worker thread `slint-wgpu-device`, and the first window consumed it.
The trace prints `worker spawned backends=Backends(DX12)`, `consumed`, and `worker finished thread=Some("slint-wgpu-device") ok=true adapter=Some("Microsoft Basic Render Driver")`.
The window reports the same adapter, limits, features and snapshot as the baseline's ordinary path.

**Experiment.** The gate declines as designed.
With the default mask the effective mask is `VULKAN | DX12` (case b).
With `WGPU_VALIDATION=1` the flags contain `VALIDATION` (case d).
Both then take the ordinary path and match the baseline.

**Experiment.** When the probe rejects WARP (case c), the candidate discards the prepared device before falling back.
Both variants fall back to the software renderer and produce the same snapshot.
The `fixed` variant shows the worker destroying the unclaimed result on its own thread.

**Experiment.** A second window takes the ordinary path, since the prepared device is consumed once (case e).
Both windows match the baseline.

**Experiment.** At `adde532c9`, returning from `main` with a component but no shown window aborts with `0xC0000409` (case g).
The panic is `threads should not terminate unexpectedly`, raised from `std::thread` `lifecycle.rs` during thread-local destruction.
The baseline exits with 0 because it never prepares a device on Windows.

**Documented.** Rust runs the main thread's thread-local destructors at process exit on Windows, holding the loader lock, and other threads may already be forcibly terminated ([`LocalKey` platform notes](https://doc.rust-lang.org/std/thread/struct.LocalKey.html#platform-specific-behavior)).
**Source-established.** `JoinHandle::join` panics with that message when the joined thread still holds its result packet, which happens when it was terminated (Rust 1.98 `library/std/src/thread/lifecycle.rs`, `JoinInner::join`).
Slint's global context is a thread-local, so `PendingWgpuDevice::drop` runs there and joined the terminated worker.

**Experiment.** The same scenario aborts on macOS at `97066a9e8`, so the macOS branch already had this regression.
Upstream `73e341928` exits with 0.
`97066a9e8` and `adde532c9` exit with 134 (SIGABRT) after two panics: objc2's `autoreleasepool` accesses a destroyed thread-local, and the Metal queue drop then opens another pool.
Logs: `results/2026-09-28/slint-windows-macos-checks/macos-exit-immediately-*.log`, from a debug build of the probe app against each revision.

**Experiment.** `61f2ce99f` fixes both.
An unclaimed result stays on the worker until the main thread claims or releases it, and `Drop` releases without joining.
Case g exits with 0 on Windows in three of three runs, and `exit-immediately` exits with 0 on macOS in three of three archived runs.
In one Windows run the `released on drop` trace line is missing, which fits Rust's best-effort destructor guarantee.
The exit code was 0 either way.

## Unit Tests and Checks

**Experiment.** `cargo test -p i-slint-backend-winit --no-default-features --features renderer-femtovg-wgpu --lib --locked` passes on the VM at `61f2ce99f` with no local changes.
All 16 tests pass, including `d3d12_debug_layer_flags_decline_only_on_windows` and `only_prepares_the_platforms_surface_independent_backend` under `target_os = "windows"`.
Log: `results/2026-09-28/slint-windows-run2/windows-unit-tests.log`.

**Experiment.** On macOS at `61f2ce99f` the 15 Winit unit tests pass.
The software-only, WebAssembly and core checks pass.
`cargo fmt --all --check` and `git diff 73e341928 --check` are clean.
Clippy reports only the two existing lints in `internal/backends/winit/lib.rs`, at line 1409 now and at upstream line 1397.
The WebAssembly check reports the existing unused-import warning in the FemtoVG snapshot code.
Log: `results/2026-09-28/slint-windows-macos-checks/macos-validation.log`.
Probe-app smoke runs on macOS render identical snapshots across three repetitions per scenario: `macos-61f2ce99f__*.log`.

## Conclusions

- The DX12 gate reaches the worker only with an explicit `WGPU_BACKEND` that masks to `DX12` and no debug-layer flags.
  On WARP, Slint then renders the same pixels with the same adapter, limits and features as the ordinary path.
- The probe-rejection release in `4b7fea77e` works on Windows, where WARP-only machines hit it.
- The worker design at `97066a9e8` aborts at exit on macOS and, once extended, on Windows, whenever a backend exists but no window ever resumed.
  `61f2ce99f` removes that abort on both platforms.
- The candidate's first-window startup time can't be judged from these runs.

## Limits and Open Questions

- **Hypothesis.** WARP is a user-mode software driver.
  Hardware drivers, hybrid GPUs and a real `D3D12CreateDevice` cost weren't exercised.
- **Hypothesis.** The debug-layer hazard is avoided by declining, not tested.
  Case d declines, so the worker never overlapped a validating probe instance.
  The harness's W2 test covers whether the overlap is actually harmful.
- In case a `SLINT_WGPU_CPU=1` makes the probe return before it creates an instance (`internal/core/graphics/wgpu_30.rs`, `any_wgpu30_adapters_with_gpu`).
  So the worker and a probe instance ran concurrently only in case c, where both enumerated WARP.
- After the fix, a released but still running worker can be terminated by `ExitProcess` or by `exit` on macOS while it destroys its result.
  The process is exiting then, so this was judged acceptable.
  It was not separately exercised.
- DPI scaling, `WGPU_DX12_PRESENTATION_SYSTEM` and missing shader compilers were not varied here.
  They are harness tests W3, W8 and W9.
