# Results

Each matrix directory holds the case JSON records, `manifest.json` with the toolchain and environment, and `summary.json` / `summary.md` from `compare.py`.
Personal checkout and Cargo registry paths have been replaced with relative paths or placeholders.
Raw logs, crash reports, generated task wrappers, command transcripts and Windows event exports are excluded from the public archive.
References to those files in the historical write-ups describe the original experiments; the files themselves remain local.
Platform, adapter, driver and toolchain facts are retained as experimental context.
Every run used a clean tree, 2 windows for `multi-window`, 3 iterations for `repeated`, and a 60 second watchdog.
They're correctness smoke runs, and their timings support no performance claims.

The first table's runs used harness commit `ec86914`.
The hazard runs used `cbb9b2a`, except `windows-vm-dx12-compiler-dxc`, which used `8623b8a` so its record keeps `wgpu_core::instance` debug lines.
Each `manifest.json` names its commit.
Their summaries were regenerated with `compare.py` at `9e91e22`, which expects an instance-only worker to finish before an adapter or device exit.
The Windows hazard runs used the D3D12 debug layer from the Graphics Tools optional feature.

## 2026-09-28

| Directory | Platform | Driver and mask | Result |
| --- | --- | --- | --- |
| `macos-metal-default` | macOS 26, Apple M4 Max, Rust 1.96.0 | Metal, default mask | PASS |
| `macos-metal-explicit` | same | Metal, `WGPU_BACKEND=metal` | PASS |
| `windows-vm-default` | Windows 11 ARM64 VM (VMware), Rust 1.98.1, interactive session | WARP on DX12, default mask | PASS |
| `windows-vm-dx12` | same | WARP, `WGPU_BACKEND=dx12` | PASS |
| `windows-vm-vulkan` | same | `WGPU_BACKEND=vulkan` | UNAVAILABLE |
| `windows-vm-gl` | same | `WGPU_BACKEND=gl` | UNAVAILABLE |
| `windows-vm-ssh-session0` | same, over SSH (session 0) | WARP, default mask | FAIL |
| `docker-linux-arm64-vulkan-lavapipe` | Ubuntu 24.04 arm64 in Docker Desktop, Xvfb | Mesa 25.2.8 lavapipe, `WGPU_BACKEND=vulkan` | PASS |
| `docker-linux-arm64-gl-llvmpipe` | same | Mesa llvmpipe, `WGPU_BACKEND=gl` | FAIL |

The Windows runs used generated scheduled-task wrappers; those wrappers and their console logs are excluded.

### Hazard Runs

These test `docs/research/dx12-worker-safety.md` W1, W2, W4 to W7, W9 and W10, and the pipeline counters from `docs/research/pipeline-compilation-and-caching.md`.
All Windows rows are the same VM, WARP only, in the interactive session, release unless noted.

| Directory | Suite or variants | Environment | Tests | Result |
| --- | --- | --- | --- | --- |
| `windows-vm-dx12-debug-hazards` | `base hazards`, debug build | `WGPU_BACKEND=dx12` | W2, W4, W5, W6 | PASS |
| `windows-vm-dx12-validation` | `base probe` | `WGPU_BACKEND=dx12 WGPU_VALIDATION=1` | W2, W5 | PASS |
| `windows-vm-dx12-control` | `control` | `WGPU_BACKEND=dx12 WGPU_VALIDATION=0` | W2 positive control | FAIL, as intended |
| `windows-vm-dx12-lifetime` | `lifetime` | `WGPU_BACKEND=dx12` | W4, W6 | PASS |
| `windows-vm-default-probe` | `probe` | default mask | W7 | PASS |
| `windows-vm-dx12-probe-slint-cpu` | `probe` | `WGPU_BACKEND=dx12 SLINT_WGPU_CPU=1` | W7 | PASS |
| `windows-vm-dx12-env-baseline` | `single multi-window failure-limits` | `WGPU_BACKEND=dx12` | W10 baseline | PASS |
| `windows-vm-dx12-compiler-fxc` | same | `WGPU_DX12_COMPILER=fxc` | W9 | PASS |
| `windows-vm-dx12-compiler-dxc` | same | `WGPU_DX12_COMPILER=dxc`, no `dxcompiler.dll` | W9 | UNAVAILABLE |
| `windows-vm-dx12-power-low`, `-power-high` | same | `WGPU_POWER_PREF=low`, `high` | W10 | PASS |
| `windows-vm-dx12-wgpu-debug` | same | `WGPU_DEBUG=1` | W10 | PASS |
| `windows-vm-dx12-gpu-based-validation` | same | `WGPU_GPU_BASED_VALIDATION=1` | W10 | PASS |
| `windows-vm-dx12-adapter-name` | same | `WGPU_ADAPTER_NAME=basic` | W10 | PASS |
| `windows-vm-dx12-gl-mask` | `single` | `WGPU_BACKEND=dx12,gl` | W1 | PASS |
| `windows-vm-dx12-no-indirect-validation` | `single` | `WGPU_VALIDATION_INDIRECT_CALL=0` | counters | PASS |
| `windows-vm-dx12-visual` | `single multi-window` | `WGPU_DX12_PRESENTATION_SYSTEM=Visual` | part of W3 | PASS |
| `macos-metal-counters` | `single probe-after-device join-block-after-device-after-window early-exit-in-device` | default | counters | PASS |
| `macos-metal-no-indirect-validation` | `single` | `WGPU_VALIDATION_INDIRECT_CALL=0` | counters | PASS |

Every Windows row except `gl-mask` and `default-probe` also sets `WGPU_BACKEND=dx12`.
`windows-vm-dx12-env-parity.md` is the `compare-runs.py --same adapter device` table over the W10 runs and `compiler-fxc`.

### Slint-Integrated Runs

These run Slint itself through the probe app in `slint-integration/app`, not the harness.
They aren't `run-matrix.py` runs, so each directory has its own layout.

| Directory | Platform | What it holds | Write-up |
| --- | --- | --- | --- |
| `slint-windows-run1` | Windows 11 ARM64 VM, WARP, interactive session | Cases a to g for `baseline` (`97066a9e8`) and `candidate` (`adde532c9`), three runs each | `docs/slint-windows-integration.md` |
| `slint-windows-run2` | same | The same cases for `fixed` (`61f2ce99f`), and the Winit unit tests on Windows | same |
| `slint-windows-macos-checks` | macOS 26, Apple M4 Max | The exit scenario at `73e341928`, `97066a9e8` and `wgpu-worker-dx12`, probe-app smoke runs at `61f2ce99f`, and the macOS build checks | same |
| `exit-stress` | macOS and the Windows VM | 200 exits per set for the fixed builds and the known-bad controls, with exit-code histograms and crash-report counts | `exit-stress/README.md` |

In `exit-stress`, no fixed build aborted in 2,200 runs.
The known-bad controls aborted in 200 of 200 macOS debug runs and in 50 of 50 Windows release runs.

## Findings

### Metal: All Strategies Agree

Experiment (`macos-metal-default`, `macos-metal-explicit`).
Every strategy selects Apple M4 Max, configures `Bgra8Unorm` with `Fifo`, produces the same pixel hash, and presents.
The prepared adapter supports the surface in every window, including simultaneous windows from one device.
Abandoned workers finish and join at all three gate points, and ordinary initialization works afterwards.
Forced failures produce the same error class as ordinary.
`slint_would_prepare_on_worker` is true, so this is the configuration the Slint branch uses.

### Windows VM: One Adapter, Same Result

Experiment (`windows-vm-default`, `windows-vm-dx12`).
The VM's display device is `VMware SVGA 3D` (driver 9.17.8.9, from `Win32_VideoController`).
WGPU enumerates only `Microsoft Basic Render Driver` (WARP, DX12, device type `Cpu`) for both masks.
All strategies select it, configure `Bgra8Unorm` with `Mailbox`, match pixel hashes, and present.
`slint_would_prepare_on_worker` is false, since the default mask is `VULKAN | DX12`.
With one adapter, surface-free and surface-aware selection can't disagree, so this doesn't test divergence.

Experiment (`windows-vm-vulkan`, `windows-vm-gl`).
No Vulkan or GL adapter exists on the VM.
Each strategy fails with a no-backend error.

### Windows Session 0: Configure Fails for Every Strategy

Experiment (`windows-vm-ssh-session0`).
Over SSH, window creation succeeds, but `Surface::configure` fails with `Invalid surface` under every strategy.
Offscreen rendering still matches.
The scheduled-task wrapper in `scripts/windows/run-interactive.ps1` avoids this.

### Linux GL: Surface-Free Selection Picks an Adapter That Can't Present

Experiment (`docker-linux-arm64-gl-llvmpipe`, for example `worker-device__single.json`).
With Slint's instance descriptor and `WGPU_BACKEND=gl` under Xvfb, ordinary selection finds no adapter compatible with the window surface.
`worker-device` gets the llvmpipe GL adapter without a surface.
Its `is_surface_supported` check then returns false, and the fallback finds no compatible adapter either.
Without that check, as in the Slint branch, the prepared adapter would reach `Surface::configure`.

Source-established: the GL backend picks its EGL display at instance creation from `InstanceDescriptor::display`
([egl.rs L769](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/gles/egl.rs#L769)).
Slint passes `display: None`, so Mesa's surfaceless platform is chosen when available
([egl.rs L846](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/gles/egl.rs#L846)).
A surface is presentable only when the context supports native windows
([egl.rs L982](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/gles/egl.rs#L982)).
Hypothesis: the surfaceless display offers no window-capable EGL config, so no GL surface is presentable.

This doesn't affect the Slint branch as written.
Its defaults avoid GL, and it prepares only when the mask is exactly `METAL`.
It shows why extending the worker beyond Metal needs a surface check, or a display handle at instance creation.

### Linux Vulkan: Same Result

Experiment (`docker-linux-arm64-vulkan-lavapipe`).
Lavapipe exposes one adapter.
All strategies select it, configure `Bgra8Unorm` with `Immediate`, match pixel hashes, and present.

### Known Difference: No Loaded Backend

Experiment (every run's `failure-unsupported-mask`).
With a mask that has no compiled backend, surface-first strategies fail at surface creation.
`worker-device` fails at adapter selection, since no surface exists yet.
`compare.py` labels this a known difference.

### Across Drivers

Experiment: the pixel hash is identical within each run.
Metal stores the clear color's blue channel as 77, while WARP and lavapipe store 76, so hashes differ across drivers.
Every record shows `pending_polls: 0`, so no WGPU call returned `Pending`.

### W2: The Probe Doesn't Remove the Worker's Device

Experiment (`windows-vm-dx12-debug-hazards`, `windows-vm-dx12-validation`, the `probe-*` variants).
Every record that reached a device shows `d3d12sdklayers.dll` loaded, and every presented window's device reports `debug_device: true`.
So the D3D12 debug layer wrapped these devices, unlike release runs without `WGPU_VALIDATION=1`.
The probe ran with the worker in instance creation, in adapter enumeration (`D3D12CreateDevice`), in `request_device`, and after it finished (`probe.worker_stage_at_probe`).
Each probe instance called `EnableDebugLayer` again.
`GetDeviceRemovedReason` returned `S_OK` after the probe and after the first present, in every strategy.
The info queue held only warnings #820 and #1318, the same in `ordinary`.
The device rendered the expected pixels again after the present.

Experiment (`windows-vm-dx12-control`).
With `WGPU_VALIDATION=0` and `--probe-validation on`, only the probe enables the debug layer, after a device made without it exists.
The device is then lost: `GetDeviceRemovedReason` returns `0x887a0007` (`DXGI_ERROR_DEVICE_RESET`), and the lost-device callback fires.
This happens in `worker-device` with the probe after the worker's device, and in `ordinary` with the probe after the first window's device.
It doesn't happen when the probe runs before any device exists.
So the harness detects the removal Microsoft documents, and the hazard depends on the layer's state at device creation, not on which thread made the device.

Conclusion, Source-established plus Experiment.
Slint builds every instance from the same `InstanceFlags::from_build_config().with_env()`.
The worker's own instance enables the layer before its enumeration creates any device, so no device predates the first `EnableDebugLayer`.
A repeated `EnableDebugLayer` on an already enabled layer removed nothing here.
Removal needs flags that differ between instances in one process, such as `WGPU_VALIDATION` changed at run time, or another component enabling the layer later.
That breaks the ordinary path the same way.
WARP only: a hardware driver's debug layer wasn't tested.

### W4 and W6: Blocking Joins and Early Exits Complete

Experiment (`windows-vm-dx12-lifetime`, and the same variants in `windows-vm-dx12-debug-hazards`).
With the worker gated at each point, the main thread blocked in a plain join for 300 to 750 ms, before `EventLoop::new` and with a shown window.
Every join returned once the helper released the gate, and the joined device then rendered and presented.
No case needed the watchdog.

Experiment (same runs, `early-exit-*`).
`std::process::exit(7)` while the worker was in instance creation, in enumeration, or in `request_device` exited with code 7.
Each of these worker cases took under 0.2 s of wall time, far inside the 60 s watchdog.
The record shows the worker still in that call after one full record write, except once in release, where it had moved from instance creation into enumeration.
This covers `exit` from `main`'s thread with an unjoined worker only.
Slint's backend lives in a thread-local, whose destructor runs at exit, and `docs/slint-windows-integration.md` covers what that does.

### W5: Abandonment Under Validation

Experiment (`windows-vm-dx12-debug-hazards`, `windows-vm-dx12-validation`, `abandon-*`).
Abandoned workers joined and dropped their results at all three gate points with the debug layer on.
No panic came from `ReportLiveDeviceObjects`, and ordinary initialization rendered afterwards.

### W7: A Rejected WARP Device Lives Until Exit

Experiment (`windows-vm-default-probe`).
Without `SLINT_WGPU_CPU`, the probe rejects WARP in every strategy, with the default mask and with `dx12` (`probe.rejected: true`).
With `--on-probe-reject keep`, as in the Slint branch, `worker-device` had finished a device about 59 ms into the run.
It was released only at process exit, about 426 ms in, after a 300 ms stand-in window (`prepared_release.when: "exit"`, `had_device: true`).
In a real application that is the whole application lifetime.
`ordinary` created no device at all.
With `drop`, the join and drop happen at the rejection, and the device is gone about 111 ms in.
With `SLINT_WGPU_CPU=1` (`windows-vm-dx12-probe-slint-cpu`), the probe creates no instance, and every strategy renders on WARP.

### W9: FXC Loads, and a Missing DXC Fails Earlier on the Worker

Experiment (`windows-vm-dx12-env-baseline`, `windows-vm-dx12-compiler-fxc`).
The VM has `C:\Windows\System32\D3DCompiler_47.dll` and no `dxcompiler.dll` on the search path.
With the default `Auto` and with `fxc`, `wgpu-hal` logs `Using FXC for shader compilation` and only `d3dcompiler_47.dll` is loaded (`dx12_compiler: "fxc"`, `dx12_modules`).

Experiment (`windows-vm-dx12-compiler-dxc`).
With `WGPU_DX12_COMPILER=dxc`, `wgpu_core::instance` logs `failed to create Dx12 backend ... Failed to load dynamic DXC` (`LoadLibraryExW`, OS error 126).
The instance then has no backend.
Surface-first strategies fail at surface creation (`surface-creation-failed`), while `worker-device` fails at adapter selection (`adapter-not-found`).
`compare.py` labels this the known no-backend difference.
For Slint, the user would see `Failed to find an appropriate adapter` instead of `Error creating wgpu window surface`.
The `docs/research/dx12-worker-safety.md` claim that instance failure behaves identically holds for the outcome, not for the message.

### W10: Environment Parity

Experiment (`windows-vm-dx12-env-parity.md`).
`WGPU_POWER_PREF=low` and `high`, `WGPU_DEBUG=1`, `WGPU_GPU_BASED_VALIDATION=1`, `WGPU_ADAPTER_NAME=basic` and `WGPU_DX12_COMPILER=fxc` all select the same adapter with the same device features and limits in every strategy.
`dx12_gate_would_prepare` is true for `dx12` and `dx12,gl`, and false with `WGPU_ADAPTER_NAME` set and with the default `VULKAN | DX12` mask.
With one adapter, power preference can't change the order.

### Pipeline Counters

Experiment (`macos-metal-counters`, `windows-vm-dx12-env-baseline`, and both `no-indirect-validation` runs).
Right after `request_device`, every device on Metal and on WARP holds 2 compute pipelines and 2 shader modules, and no render pipelines.
With `WGPU_VALIDATION_INDIRECT_CALL=0` it holds none.
So the two indirect-validation pipelines exist in release builds on both backends, whichever thread creates the device.
Release instance flags are `VALIDATION_INDIRECT_CALL` alone (`build.instance_flags`).
Lavapipe wasn't measured, since Docker Desktop quit during these runs.

### Presentation Through DirectComposition

Experiment (`windows-vm-dx12-visual`).
With `WGPU_DX12_PRESENTATION_SYSTEM=Visual`, every strategy configures and presents, including simultaneous windows from a worker-created device.
The harness doesn't record the main thread's COM apartment, so the rest of W3 is open.

## Not Covered

- A machine with more than one adapter, where surface-free and surface-aware selection could disagree.
  That's the case the Slint branch's Metal-only restriction guards against, and software drivers can't reproduce it.
- The GitHub workflow, which hasn't run.
- Docker on x86_64, and the `gfx-rs/ci-build` Mesa used in CI.
- Two Vulkan ICDs (`scripts/docker-two-icd.sh`) and lavapipe's device counters.
  The first SwiftShader build exhausted the Docker VM's memory, and Docker Desktop then quit before the 4-job rebuild ran.
- W3's apartment check, W8 at non-100% scaling, and any hardware DX12 driver.
