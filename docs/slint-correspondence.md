# Slint Correspondence

This page maps each harness setting to the Slint code it copies.
Slint references are to the implementation worktree at `97066a9e8` (branch `wgpu-worker-preparation`, based on upstream `73e341928`).
That branch isn't published, so Slint references name a file, a function, and a line at that commit.
WGPU references are pinned to wgpu 30.0.1 (`40f4a34`).

Every claim carries a label:

- **Source-established**: read from pinned source, with a link or file reference.
- **Experiment**: observed with this harness, with the command and output location.
- **Hypothesis**: not yet checked.

## Backend Mask

| Harness (`src/slint_init.rs`) | Slint (`97066a9e8`) | Label |
| --- | --- | --- |
| `default_backends_to_avoid` | `internal/core/graphics/wgpu_30.rs`, `default_backends_to_avoid`, L152 | Source-established |
| `mask_backends` | same file, `mask_backends`, L165 | Source-established |
| `requested_backends` (no failure) | same file, `async_init_instance_adapter_device_queue_surface`, L332, `None` arm: `Backends::from_env().unwrap_or_default()` | Source-established |
| `check_backends` and its message | same function, empty-mask check in the `None` arm | Source-established |
| `slint_would_prepare` | `internal/backends/winit/wgpu_device.rs`, `PendingWgpuDevice::start`, L18, and `can_prepare`, L63 | Source-established |
| `would_prepare_for(.., DX12)` | none at `97066a9e8`: the same preconditions with `DX12` in place of `METAL`, as `docs/research/dx12-worker-safety.md` section 6 proposes | Source-established absence |

`slint_would_prepare` covers the default renderer with no configured graphics API.
It checks `WGPU_ADAPTER_NAME` and requires the effective mask, intersected with the compiled backends, to be exactly `METAL`.
Each record stores it as `backends.slint_would_prepare_on_worker`.

Source-established: on Windows the default mask is `VULKAN | DX12` after masking.
Experiment: the Windows ARM64 VM records `effective_and_enabled: Backends(VULKAN | DX12)` and `slint_would_prepare_on_worker: false` (`results/2026-09-28/windows-vm-default/`).

## Instance

| Harness | Slint | Label |
| --- | --- | --- |
| `default_instance_descriptor` | `wgpu_30.rs`, `default_instance_descriptor`, L284: `InstanceFlags::from_build_config().with_env()`, `BackendOptions::from_env_or_default()`, default memory budget thresholds, `display: None` | Source-established |
| `create_instance` | `wgpu::util::new_instance_with_webgpu_detection`, as in `prepare_default_metal_device` (L316) and the `None` arm | Source-established |

Source-established: on native targets `new_instance_with_webgpu_detection` only removes `BROWSER_WEBGPU` and calls `Instance::new`
([init.rs L116](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/util/init.rs#L116)).

## Adapter and Device

| Harness | Slint | Label |
| --- | --- | --- |
| `select_adapter` | `init_default_adapter_device_queue`, L294: `initialize_adapter_from_env_or_default(instance, surface)` | Source-established |
| `device_descriptor` | `default_device_descriptor`, L273: adapter features minus `all_experimental_mask()`, `adapter.limits()`, experimental features disabled, `MemoryHints::MemoryUsage`, default trace | Source-established |
| error texts `Failed to find an appropriate adapter: …` and `Failed to create device: …` | `init_default_adapter_device_queue` | Source-established |
| `block` | `poll_once`, L547, with a `pollster` fallback that the record counts in `pending_polls` | Source-established |

Source-established: without `WGPU_ADAPTER_NAME`, `initialize_adapter_from_env_or_default` calls `request_adapter` with `PowerPreference::from_env().unwrap_or_default()` and the given surface
([init.rs L57](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/util/init.rs#L57)).
With `WGPU_ADAPTER_NAME` set and no match, `initialize_adapter_from_env` panics
([init.rs L44](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/util/init.rs#L44)).

Experiment: every archived run records `pending_polls: 0`, so no WGPU call returned `Pending` on these platforms.

## GPU Probe

| Harness (`src/probe.rs`) | Slint | Label |
| --- | --- | --- |
| `any_adapters_with_gpu` | `internal/core/graphics/wgpu_30.rs`, `any_wgpu30_adapters_with_gpu`, L180, `None` arm: returns true when `SLINT_WGPU_CPU` is set, otherwise builds a second instance with the same descriptor, enumerates, and rejects when every adapter is `DeviceType::Cpu` | Source-established |
| `--probe concurrent`, run after `EventLoop::new` | `WGPUFemtoVGRenderer::new_suspended`, `internal/backends/winit/renderer/femtovg.rs` L203, after `SharedBackendData::new` started the worker | Source-established |
| `--on-probe-reject keep` | the renderer factory fails and `take()` never runs, so the prepared result lives until `SharedBackendData` drops | Source-established |

Differences, all intentional:

- The probe uses `wgpu::Instance::new`, as Slint's probe does, not `new_instance_with_webgpu_detection`.
  On native targets the two are equivalent.
- `--on-probe-reject continue` renders after a rejection, which Slint never does.
  It stands in for a GPU adapter passing the probe, so WARP can exercise the probe's timing.
- `--on-probe-reject drop` is the fix proposed in `docs/research/dx12-worker-safety.md`, section 6, not current Slint behavior.
- `--probe-validation on` departs from Slint on purpose, as a control.

## Worker Thread

| Harness (`src/worker.rs`) | Slint | Label |
| --- | --- | --- |
| thread name `slint-wgpu-device` | `PendingWgpuDevice::start` | Source-established |
| `autoreleasepool` around the job and around the abandoned join | `start` and `impl Drop for PendingWgpuDevice`, L52 | Source-established |
| `Pending::take` joins once | `PendingWgpuDevice::take` | Source-established |
| started before `EventLoop::new` | `internal/backends/winit/lib.rs` L496, before the event loop builder runs | Source-established |

Differences, all intentional:

- A worker panic becomes an error record.
  Slint resumes the panic on the consuming thread.
- `abandon` waits with a timeout before joining, so a stuck worker is reported instead of blocking.
  Slint's `Drop` joins without a timeout.
- The harness runs worker strategies on every platform.
  Slint prepares only when `slint_would_prepare` is true.

## Surface

| Harness | Slint | Label |
| --- | --- | --- |
| `surface_config` | `internal/renderers/femtovg/wgpu.rs`, `configure_surface_from_init_result`, L397: `get_default_config`, first of `Rgba8Unorm` or `Bgra8Unorm` else `formats[0]`, opaque alpha when advertised | Source-established |
| configure skipped for a zero-sized window | same function | Source-established |
| `create_surface` error text | `async_init_instance_adapter_device_queue_surface` | Source-established |

Source-established: `get_default_config` takes the first advertised present mode
([surface.rs L83](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/api/surface.rs#L83)).
Slint's first resize then sets `AutoVsync` (`wgpu.rs` L339).
The harness records the initial configuration only.

Experiment: Metal advertises `[Fifo, Immediate]` and configures `Fifo`.
DX12 on WARP advertises `[Mailbox, Fifo, Immediate]` and configures `Mailbox`.
This happens under every strategy, so it doesn't bear on the worker question.

Slint consumes a prepared result as `WGPUConfiguration::Manual` and creates the surface from its instance without checking surface support (`internal/backends/winit/renderer/femtovg.rs` L266 and the `Manual` arm of `async_init_instance_adapter_device_queue_surface`).
Source-established.
The harness's `worker-device` strategy adds `adapter.is_surface_supported` and falls back to surface-aware selection when it is false.
Each record logs the check as a `prepared_adapter_surface_check` event.

## What the Harness Adds

- The scene: a clear to linear `(0.1, 0.2, 0.3)` plus an orange triangle in a 64×64 texture of the surface format.
- `multi-window` shares one device across all windows under every strategy.
  Slint gives only the first window the prepared device, and later windows initialize ordinarily.
- `worker-instance` and `worker-after-select` are alternatives the branch doesn't implement.
  They show whether a narrower worker would behave differently.
