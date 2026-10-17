# Extending Slint's WGPU Worker Preparation Beyond Metal

Report date: 2026-09-28.

## Question

Slint's `wgpu-worker-preparation` branch (`97066a9e8`) creates FemtoVG's default WGPU instance, adapter, device and queue on a worker thread while Winit initializes.
It does so only on macOS, and only when the effective backend mask is exactly `METAL`.
This report asks whether the same preparation can extend to other backends, starting with DX12 on Windows.

It covers correctness, lifetime and failure behavior, and it says what is known about performance.
It makes no performance claim beyond what the earlier Metal evidence supports.

Claim labels follow the research pages: **Source-established** (read from pinned source), **Documented** (vendor documentation), **Experiment** (an archived run) and **Hypothesis** (not checked).

## Short Answer

- The Metal-only branch is correct on the one Mac tested, except for an exit abort in debug builds.
  This work found and fixed that abort on branch `wgpu-worker-preparation-fixes`.
- A DX12 extension is correct on WARP in a Windows ARM64 VM.
  It reaches only users who set `WGPU_BACKEND` to a value that masks to `DX12`, because Slint's default Windows mask is `VULKAN | DX12`.
  The follow-up branch is `wgpu-worker-dx12-followup`.
- There is no performance evidence for DX12.
  WARP in a VM can't provide it, and Slint's probe already creates the D3D12 device on the main thread in the default configuration.
- Vulkan and the mixed Windows default mask need a surface to select exactly.
  No speculative worker strategy preserves both selection and error behavior there.
  They were inspected only.

## Branches

Slint worktree `<slint-worktree>`.
None of these branches is published.

| Branch | Tip | Content |
| --- | --- | --- |
| `wgpu-worker-preparation` | `97066a9e8` | The original Metal-only worker, used by the earlier timing campaigns |
| `wgpu-worker-preparation-fixes` | `b1dd5ac4f` | `97066a9e8` plus `4b7fea77e`, `2a4f76e1f` and `b1dd5ac4f`, Metal-only |
| `wgpu-worker-dx12-followup` | `11fe95b24` | The fixes plus one commit that adds the DX12 gate |
| `wgpu-worker-dx12` | `ab27d194a` | The earlier DX12 history, with the same tree as `11fe95b24` |

The three fixes:

- `4b7fea77e` releases a prepared device when the adapter probe rejects the WGPU renderer.
- `2a4f76e1f` stops `Drop` from joining the worker, since `Drop` runs in thread-local destructors at exit.
- `b1dd5ac4f` stops the probe-rejection release from joining, so the fallback renderer doesn't wait for an unused device.

## Supported Conclusions

1. **Experiment.** On Metal, surface-free and surface-aware selection agree in every harness scenario.
   That includes abandonment, forced failures and several windows from one device.
   Evidence: [`macos-metal-default`](results/2026-09-28/macos-metal-default/summary.md), [`macos-metal-explicit`](results/2026-09-28/macos-metal-explicit/summary.md).
2. **Source-established.** DX12 selection is surface-independent in WGPU 30.0.1.
   Enumeration ignores the surface, and `surface_capabilities` always returns `Some` for a Win32 surface.
   Evidence: [DX12 worker safety](docs/research/dx12-worker-safety.md), [Windows mixed default](docs/research/windows-mixed-default.md).
3. **Experiment, WARP only.** With `WGPU_BACKEND=dx12`, a worker-created DX12 instance, adapter, device and queue render the same pixels as the ordinary path.
   That holds in the harness and in Slint itself, with the same adapter, limits and features.
   Evidence: [`windows-vm-dx12`](results/2026-09-28/windows-vm-dx12/summary.md), [Slint on Windows](docs/slint-windows-integration.md).
4. **Experiment, WARP only.** Slint's main-thread probe didn't remove the worker's device under the D3D12 debug layer, at any point of worker progress.
   A device was removed only when the layer was off at device creation and turned on later.
   That breaks the ordinary path too.
   Evidence: [`windows-vm-dx12-debug-hazards`](results/2026-09-28/windows-vm-dx12-debug-hazards/summary.md), [`windows-vm-dx12-control`](results/2026-09-28/windows-vm-dx12-control/summary.md).
5. **Experiment, WARP only.** Blocking joins without a message pump returned, and abandonment under validation raised no panic.
   Evidence: [`windows-vm-dx12-lifetime`](results/2026-09-28/windows-vm-dx12-lifetime/summary.md), [`windows-vm-dx12-validation`](results/2026-09-28/windows-vm-dx12-validation/summary.md).
6. **Experiment.** Before `4b7fea77e`, a device prepared on a WARP-only machine lived until exit after the probe rejected WGPU.
   Evidence: [`windows-vm-default-probe`](results/2026-09-28/windows-vm-default-probe/summary.md).
7. **Experiment, WARP only.** Environment variables left selection, features and limits unchanged.
   Evidence: [`windows-vm-dx12-env-parity.md`](results/2026-09-28/windows-vm-dx12-env-parity.md).
8. **Experiment.** A missing DXC with `WGPU_DX12_COMPILER=dxc` changes the first error.
   The worker reports adapter selection, while the ordinary path reports surface creation.
   The follow-up gate doesn't decline this case.
   Evidence: [`windows-vm-dx12-compiler-dxc`](results/2026-09-28/windows-vm-dx12-compiler-dxc/summary.md).
9. **Experiment.** On Metal and on WARP, `request_device` builds 2 indirect-validation compute pipelines in release builds, on whichever thread creates the device.
   Evidence: [`macos-metal-counters`](results/2026-09-28/macos-metal-counters/summary.md), [`windows-vm-dx12-env-baseline`](results/2026-09-28/windows-vm-dx12-env-baseline/summary.md).
10. **Experiment, one VM.** A stock Windows 11 install used FXC, because no `dxcompiler.dll` was on the search path.
    Evidence: [`windows-vm-dx12-compiler-fxc`](results/2026-09-28/windows-vm-dx12-compiler-fxc/summary.md).
11. **Experiment.** On Linux GL, a surface-free adapter can't present, so a worker there would need a surface check.
    Evidence: [`docker-linux-arm64-gl-llvmpipe`](results/2026-09-28/docker-linux-arm64-gl-llvmpipe/summary.md).

## Platforms

| Platform | Driver | Status | What ran |
| --- | --- | --- | --- |
| macOS 26, Apple M4 Max | Metal | Tested | Harness matrix and counters, Slint probe app, exit stress, earlier timing campaigns at `97066a9e8` |
| Windows 11 ARM64 VM (VMware) | WARP through DX12 | Tested | Harness matrix and hazard runs, Slint probe app, exit stress, Winit unit tests |
| Ubuntu 24.04 arm64 in Docker Desktop | Mesa lavapipe, Vulkan | Tested for the harness only | Harness matrix. Slint never prepares on Linux. |
| Same | Mesa llvmpipe, GL | Tested for the harness only | Surface-free selection diverges from the ordinary path |
| Windows | Vulkan | Inspected only | The VM has no Vulkan ICD, so `windows-vm-vulkan` is unavailable |
| Windows | Hardware DX12 (NVIDIA, AMD, Intel, Qualcomm) | Untested | Nothing |
| Any | Hybrid or multi-GPU machines | Untested | Nothing. Software drivers expose one adapter each. |
| Windows x64, Linux x86_64 | Any | Untested | The CI workflow hasn't run |

## Backend Differences

Cells are Source-established from the research pages unless marked.

| Aspect | Metal | DX12 | Vulkan |
| --- | --- | --- | --- |
| Adapter selection | Surface-independent under a `METAL` mask | Surface-independent under a `DX12` mask. The Windows default `VULKAN \| DX12` tries Vulkan adapters first, so it is surface-dependent. | Surface-dependent: queue family 0 must support presenting to the real surface |
| Thread ownership | Creation and destruction need objc2 autorelease pools on the thread that runs them | No COM initialization needed (Documented). The worker's DXGI factory is used from the main thread. No per-thread pool. | Inspected only |
| Exit behavior | A join inside a pool during thread-local destruction aborts in debug builds (Experiment) | `ExitProcess` can terminate the worker before thread-local destructors run, so a join there aborts (Experiment) | Not run |
| Device-internal pipelines | 2 compute pipelines (Experiment) | 2 compute pipelines, with a DX12-specific draw constant (Experiment for the count) | 2 compute pipelines, not counted on lavapipe |
| Renderer pipeline creation | FemtoVG creates pipelines lazily on the main thread at first flush. MSL and `newLibraryWithSource` per stage. | Same timing. HLSL, then FXC or DXC on a miss in a per-device bytecode map. | Same timing. SPIR-V and a new `VkShaderModule` per stage. |
| Retained objects | The prepared device and queue until claimed or released. Indirect validation adds 1 buffer, 5 bind group layouts and 2 pipeline layouts per device (Experiment). | Same counts (Experiment). Each exposed adapter keeps an `ID3D12Device`, a per-adapter singleton in the process (Documented). | The prepared `VkDevice` and queue |
| Driver caches | System Metal shader cache (Documented) | D3D12 runtime and driver caches (Documented). Unknown for WARP. | Driver caches and the Mesa disk cache (Documented) |
| Explicit cache APIs | `MTLBinaryArchive`, unused by WGPU | `ID3D12PipelineLibrary`, unused by WGPU | `VkPipelineCache` through WGPU `PipelineCache`, the only one WGPU implements. FemtoVG passes `cache: None`. |
| Failure behavior | Forced failures match the ordinary error class (Experiment) | A backend that fails `init` is dropped, and the worker then reports a different first error (Experiment) | A failing `vkCreateWin32SurfaceKHR` panics. A missing surface extension hands selection to DX12 in the mixed mask. |

## The Exit Abort

**Experiment.** The Metal-only branch at `97066a9e8` aborts on macOS in debug builds when `main` returns with a component but no shown window.
It exited with 134 in 200 of 200 debug runs, after two thread-local access panics.
Release builds exited with 0 in 400 of 400 runs.
Upstream `73e341928` exits with 0, because it never creates a device before a window resumes.
Evidence: [exit stress](results/2026-09-28/exit-stress/README.md), [`slint-windows-macos-checks`](results/2026-09-28/slint-windows-macos-checks/).

**Source-established.** Slint's global context is a thread-local, so `PendingWgpuDevice::drop` runs in the main thread's thread-local destructors.
It joined the worker inside an objc2 autorelease pool.
objc2 0.5.2 and 0.6.4 track pools in a thread-local only under `debug_assertions`, and that thread-local may already be destroyed.

**Experiment.** The DX12 extension at `adde532c9` makes the same scenario abort in release builds on Windows.
It exited with `0xC0000409` in 50 of 50 runs.
**Documented.** Windows may terminate other threads before the main thread's thread-local destructors run.
**Source-established.** Joining a terminated thread panics with `threads should not terminate unexpectedly`.

The fix is `2a4f76e1f`, with `b1dd5ac4f` for the probe-rejection path.
An unclaimed result stays on the worker, and `Drop` and `discard` release the claim without joining.
The worker then destroys its result on its own thread, inside its own pool on macOS.

**Experiment.** Across the fixed builds, 2,200 runs had no abort and no crash report.
That is 1,800 on macOS, in release, traced and debug builds, and 400 on Windows.
In the traced macOS runs, 108 exited while the worker was still creating the Metal device.
Only 5 of 600 traced runs printed that the worker finished destroying its result, so exit usually ends the worker first.
Metal or D3D12 teardown is then cut short at exit, which the runs tolerated on one Mac and on WARP.

## Performance Status

**Metal.** The earlier campaigns at `97066a9e8` stand as reported in `<metal-evidence>/REPORT.md`.
The ordinary minimal-window saving was a median 6.6 ms over 24 pairs, with an interval of -1.8 to 10.8 ms, so it is inconclusive.
The `MTL_SHADER_CACHE_SIZE=0` diagnostic saved 62.9 ms over 12 pairs, with an interval of 41.7 to 86.4 ms.
That override is undocumented and doesn't show how often users meet the cost.
The fixes change only the path where no window claims the device.
The claimed path gains one channel message, and it wasn't timed again.

**DX12.** Inconclusive, because nothing was measured.
The only DX12 numbers are phase times from single smoke runs, which support no claim.

No meaningful measurement is possible on WARP in a VM:

- WARP is a CPU rasterizer with its own shader JIT.
  Its device and pipeline costs don't resemble a hardware driver's, and it has no vendor shader cache.
- The VM shares host CPUs under its own scheduling, which adds noise to startup intervals of a few milliseconds.
- WARP reaches WGPU in Slint only with `SLINT_WGPU_CPU=1`, which users don't set by default.

**Source-established.** The probe already creates the D3D12 device on the main thread.
Without `SLINT_WGPU_CPU`, `any_wgpu30_adapters_with_gpu` runs in `new_suspended` and enumerates DX12 adapters on a throwaway instance.
DX12 enumeration calls `D3D12CreateDevice` for every adapter.
**Documented.** A D3D12 device is a singleton per adapter while it lives, so the probe and the worker share one device when they overlap.

So on a machine with a GPU, the costliest native step either runs on the main thread in the probe or is shared with it.
The worker can move only the rest off the main thread.
That rest is the second instance, the DXC or FXC load, the command queue and heaps, and the two FXC-compiled validation pipelines.
**Hypothesis.** Whether that rest is large enough to measure on hardware is unknown.
Whether the probe waits on a device the worker is still creating is unknown too.

## Remaining Gaps

- Hardware DX12 drivers, including the debug layer and DXGI window messages on a kernel-mode display driver.
- Any timing of the DX12 path on a physical GPU.
- Hybrid and multi-GPU machines, where surface-free and surface-aware selection could disagree.
- Vulkan on Windows, and the mixed `VULKAN | DX12` mask with a real Vulkan adapter.
- The main thread's COM apartment (W3) and DPI scaling (W8).
- lavapipe device counters, two Vulkan ICDs, x86_64 hosts and the CI workflow.
- Teardown cut short at exit on hardware drivers.
- The software-only, WebAssembly and core checks ran at `97066a9e8` and `61f2ce99f`, but not at `b1dd5ac4f` or `11fe95b24`.

## Recommendation

- Submit the Metal-only PR from `wgpu-worker-preparation-fixes`, keeping the earlier measurement statements.
  Mention the debug-build exit abort, since a reviewer testing `97066a9e8` in debug would hit it.
- Hold the DX12 follow-up as optional.
  It is correct on WARP, but its reach is narrow and it has no performance evidence.
  Open it only if a timing run on a hardware DX12 driver shows a benefit worth the extra code, or if maintainers want the gate anyway.
- Don't extend the worker to Vulkan or to the mixed Windows default speculatively.
  If that is pursued, start from the exact strategies in [Windows mixed default](docs/research/windows-mixed-default.md): instance only, or the device after ordinary selection.

## Sources

- Research: [DX12 worker safety](docs/research/dx12-worker-safety.md), [Windows mixed default](docs/research/windows-mixed-default.md), [pipeline compilation and caching](docs/research/pipeline-compilation-and-caching.md).
- Slint-integrated runs: [Slint on Windows](docs/slint-windows-integration.md), [exit stress](results/2026-09-28/exit-stress/README.md).
- Harness runs: [results index](results/README.md).
- PR drafts: [docs/PR.md](docs/PR.md).
- Earlier Metal evidence: `<metal-evidence>/REPORT.md` and `IMPLEMENTATION.md`, outside this repository.
