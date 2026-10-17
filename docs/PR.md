# Proposed Pull Requests

Two proposals for Slint, drafted from the evidence in [REPORT.md](../REPORT.md).
No PR has been opened, and no branch is published.
Both follow `.github/pull_request_template.md` in the Slint repository, whose checklist closes each body.

The Slint worktree is `<slint-worktree>`.

## A. Metal Device Preparation

Status: ready for review as a local candidate.
Hold any broad startup improvement claim pending representative natural-cache evidence.

Branch: `wgpu-worker-preparation-fixes`, tip `b1dd5ac4f`, based on upstream `73e341928`.

Title: `winit: Prepare the default Metal device during backend initialization`

Body:

Prepare FemtoVG's default WGPU instance, adapter, device and queue on one owned worker while Winit initializes its backend.
The first window that resumes takes the prepared objects.
Limit this to macOS with Metal as the only enabled backend, where WGPU 30.0.1 selects the same adapter without a surface.
Keep explicit API configurations, explicit adapter names, the hardware probe, other renderers and other platforms on their existing paths.

Thread creation failure keeps synchronous initialization available.
Consumed errors and panics keep their existing behavior.
There are no new dependencies and no speculative rendering.

A prepared device that no window claims is released without a join, and the worker destroys it inside its own autorelease pool.
This covers two cases:

- When the adapter probe rejects the WGPU renderer, the device is released at once, and the fallback renderer doesn't wait for it.
- When the backend is dropped at exit, `Drop` doesn't join the worker.
  Slint's global context is a thread-local, so this runs in thread-local destructors.
  A join there aborted debug builds with SIGABRT when `main` returned with a component but no shown window.

Validation:

- 13 Winit unit tests pass, including tests for the release paths.
- `cargo fmt --all --check` is clean.
  Clippy reports only the two existing lints in `internal/backends/winit/lib.rs`.
- On an Apple M4 Max, a debug build of the first version of this change aborted in 200 of 200 exit runs.
  With the release fixes, 1,800 runs exited with 0 across release, traced and debug builds.
  108 of them exited while the worker was still creating the device.
- Seven successful paired GPU smoke cases produced identical snapshots, adapters, device features and limits on an Apple M4 Max.
  They ran before the release fixes, which don't touch the path where a window claims the device.

Preparation alone helps when device creation is expensive enough to matter.
The final minimal-window campaign saved a median 6.6 ms across 24 randomized pairs, with a primary-endpoint interval of -1.8 to 10.8 ms.
The fuller scene and catalog results remain inconclusive.
A separately labeled `MTL_SHADER_CACHE_SIZE=0` diagnostic saved 62.9 ms across 12 pairs, with an interval of 41.7 to 86.4 ms.
That agrees with instrumented device work of roughly 50 to 60 ms finishing before demand.
The override is undocumented, and it doesn't establish controlled cold-cache behavior or how often users meet this cost.
CPU caches and filesystem caches were neither reset nor characterized.
These campaigns ran at `97066a9e8`, before the release fixes.
The fixes add one channel message to the claimed path, which wasn't timed again.

Checklist:

- [x] If the change modifies a visible behavior, it changes the documentation accordingly.
  Not applicable: no documented behavior changes.
- [x] If possible, the change is auto-tested.
  Winit unit tests cover the handoff, both release paths and the gate.
  GPU runs are manual.
- [x] If the change fixes or closes an existing issue, the commit message references it.
  Not applicable: no issue exists.
- [ ] If the change is noteworthy, the commit message contains `ChangeLog: ...`.
  Open for maintainers.
  A candidate on squash: `ChangeLog: winit: Create the default FemtoVG WGPU device on macOS while the backend initializes`.

Before opening:

- Rerun the software-only, WebAssembly and core WGPU builds at `b1dd5ac4f`.
  They last ran at `97066a9e8` and `61f2ce99f`.
- Squash or keep the four commits as maintainers prefer.
  `2a4f76e1f` and `b1dd5ac4f` fix behavior that `97066a9e8` introduced.

## B. Optional DX12 Follow-Up

Status: optional.
It is correct on WARP, but its reach is narrow and it has no performance evidence.
Open it only after a timing run on a hardware DX12 driver, or if maintainers want the gate anyway.

Branch: `wgpu-worker-dx12-followup`, tip `11fe95b24`, one commit on top of proposal A.

Title: `winit: Prepare the default DX12 device during backend initialization`

Body:

Extend the Metal worker to Windows when the effective WGPU backend mask is exactly `DX12`.
DX12 adapter selection doesn't depend on the surface in wgpu-hal 30.0.1.
Enumeration takes no surface, and `dx12::Adapter::surface_capabilities` always returns `Some`.

This reaches few users.
The default Windows mask is `VULKAN | DX12`, so only a `WGPU_BACKEND` that masks to `DX12` qualifies, on the opt-in `renderer-femtovg-wgpu`.
Explicit graphics API configurations and `WGPU_ADAPTER_NAME` stay excluded as on macOS.

The gate also declines when the instance flags contain `VALIDATION` or `GPU_BASED_VALIDATION`.
Both configure the D3D12 debug layer, which Direct3D expects before any device exists.
The adapter probe and later windows create such instances on the main thread while the worker's device may exist.
On WARP, enabling the layer again with the same flags didn't remove the worker's device.
The decline stays until a hardware driver confirms this.
Debug builds enable validation by default, so they keep the ordinary path unless `WGPU_VALIDATION=0`.

The worker needs no COM initialization, since DXGI and D3D12 objects come from their creation functions.
The autorelease pool stays macOS-only.
Releasing without a join matters on Windows too.
`ExitProcess` can terminate the worker before the main thread's thread-local destructors run, and joining it there fails fast.

There is no performance evidence.
Every Windows run used WARP in an ARM64 VM, which can't show hardware driver costs.
Without `SLINT_WGPU_CPU`, the adapter probe already enumerates DX12 adapters on the main thread.
That enumeration calls `D3D12CreateDevice`, the costliest native step, before the worker's device is used.
The worker moves only the rest: the second instance, the shader compiler load, the queue and the two validation pipelines.

Validation:

- 16 Winit unit tests pass on Windows ARM64, including the DX12 gate tests, and 15 pass on macOS.
- On WARP with `WGPU_BACKEND=dx12 SLINT_WGPU_CPU=1`, the first window uses the prepared device.
  Adapter, limits, features and snapshot match the ordinary path.
- The default mask and `WGPU_VALIDATION=1` decline.
  A probe rejection falls back to the software renderer with the same snapshot.
- Returning from `main` without a shown window exited with 0 in 200 of 200 runs.
  The same scenario aborted with `0xC0000409` in 50 of 50 runs of an earlier version that joined in `Drop`.

Checklist:

- [x] If the change modifies a visible behavior, it changes the documentation accordingly.
  Not applicable: no documented behavior changes.
- [x] If possible, the change is auto-tested.
  Unit tests cover the Windows gate and the debug-layer decline.
  GPU runs are manual and WARP only.
- [x] If the change fixes or closes an existing issue, the commit message references it.
  Not applicable: no issue exists.
- [ ] If the change is noteworthy, the commit message contains `ChangeLog: ...`.
  Probably not noteworthy, given its reach.

Not tested:

- Hardware DX12 drivers, hybrid GPUs and any timing.
- The main thread's COM apartment and DPI scaling.

Known difference:

- With `WGPU_DX12_COMPILER=dxc` and no `dxcompiler.dll`, the worker reports a different first error than the ordinary path.
  Both fail, and the gate doesn't decline this case.
  This was seen in the harness, not in Slint.
