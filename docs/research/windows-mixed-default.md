# Windows Mixed-Mask Default

This page analyzes the WGPU configuration a Windows user of Slint's `femtovg-wgpu` renderer actually gets.
The effective backend mask there is `VULKAN | DX12`.
The question is whether any worker preparation strategy can preserve adapter selection exactly for that mask.

Research date: 2026-09-28.
WGPU references are pinned to wgpu 30.0.1 (`40f4a34`) and were checked against the registry copies of `wgpu`, `wgpu-core`, `wgpu-hal` and `wgpu-types` 30.0.1.
Slint references name a file, a function and a line in the implementation worktree at `97066a9e8` (branch `wgpu-worker-preparation`, based on upstream `73e341928`).
No timing measurements were taken for this page, and none of its claims are about speed.

Every claim carries one label:

- **Source-established**: read from pinned source, with a link or a Slint file reference.
- **Documented**: stated in vendor documentation, with a link.
- **Experiment**: observed with this harness, with the run directory.
- **Hypothesis**: not yet checked.

## Bottom Line

With the mixed mask, ordinary selection puts every surface-compatible Vulkan adapter ahead of every DX12 adapter.
Vulkan compatibility depends on the real surface, so the adapter a surface-free worker picks is not known to be the ordinary winner until the surface exists.
Two strategies preserve selection exactly by construction: preparing only the instance, and moving only `request_device` after ordinary selection.
A cached enumeration followed by a late surface filter also preserves it, under a fixed-snapshot assumption and with a fallback to ordinary `request_adapter` when nothing matches.
A speculative device on the surface-free winner can be made to select the same adapter, but it cannot guarantee both that no skipped adapter's failure ever surfaces and that no unused GPU is ever initialized.
For the mixed mask, no such speculative strategy exists.

## Who Reaches This Path

Source-established: the mask comes from `mask_backends(Backends::from_env().unwrap_or_default(), default_backends_to_avoid())`.
On Windows `default_backends_to_avoid` removes `GL` and `METAL`, and the compiled backends are `VULKAN | GL | DX12`, so the effective mask is `VULKAN | DX12`
(`internal/core/graphics/wgpu_30.rs`, `default_backends_to_avoid` L152 and `mask_backends` L165).
`Backends::default()` is `Backends::all()` ([backend.rs L140](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/backend.rs#L140)).

Experiment: the Windows ARM64 VM records `effective_and_enabled: Backends(VULKAN | DX12)` (`results/2026-09-28/windows-vm-default/`).

Source-established: `renderer-femtovg-wgpu` is an opt-in Cargo feature.
The worker gate `can_prepare` accepts only the `femtovg-wgpu` renderer, no configured graphics API, and no `WGPU_ADAPTER_NAME` (`internal/backends/winit/wgpu_device.rs`, `can_prepare` L63).
`PendingWgpuDevice::start` then requires the compiled effective mask to equal `METAL` (same file, L18), so Windows never prepares on the branch as written.

Source-established: before any window exists, `WGPUFemtoVGRenderer::new_suspended` calls `any_wgpu30_adapters_with_gpu` on the main thread
(`internal/backends/winit/renderer/femtovg.rs` L203, `internal/core/graphics/wgpu_30.rs` L180).
That probe creates a complete WGPU instance with the same mask, enumerates adapters without a surface, and drops the instance.
It returns false when every adapter has device type `Cpu`, unless `SLINT_WGPU_CPU` is set, and the renderer factory then moves on to the next renderer.
So the ordinary Windows path already pays the surface-independent instance and enumeration work once on the main thread, and then pays it again for the real instance.

Consequence, Source-established from the probe above: on a VM whose only adapter is WARP, Slint's `femtovg-wgpu` does not use WGPU at all by default.
The harness's WARP results therefore describe the `SLINT_WGPU_CPU=1` configuration, not the default one.

## Selection Semantics for `VULKAN | DX12`

### Instance Construction

Source-established: `wgpu_core::Instance::new` tries HAL backends in the fixed order Vulkan, Metal, DX12, GLES, skipping any backend outside the mask
([instance.rs L84-L90](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L84-L90)).
A backend whose `init` fails is logged at debug level and left out of `instance_per_backend`
([instance.rs L132](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L132)).
So a machine without `vulkan-1.dll`, or whose loader finds no driver, gets an instance with only DX12 active.

Source-established: Vulkan `init` loads the loader library with `ash::Entry::load`, queries the instance version, filters layers and extensions, and calls `vkCreateInstance`
([vulkan/instance.rs L583](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/instance.rs#L583),
[L835](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/instance.rs#L835)).
`VK_KHR_win32_surface` is requested only when the loader reports it
([L259](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/instance.rs#L259)).

Documented: the Windows loader discovers drivers through display-adapter registry keys, loads each driver library, and sorts physical devices so that the system preference comes first.
Only drivers implementing loader interface version 6 or later are sorted, and unsorted devices, typically software drivers, follow in an indeterminate order
([LoaderDriverInterface, Physical Device Sorting](https://github.com/KhronosGroup/Vulkan-Loader/blob/main/docs/LoaderDriverInterface.md#physical-device-sorting)).

Source-established: DX12 `init` loads `d3d12.dll`, creates the device factory and optional debug layer, creates the DXGI factory, checks tearing support, and loads DXC or falls back to FXC
([dx12/instance.rs L17](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/instance.rs#L17)).

### Per-Backend Surface Creation

Source-established: `Instance::create_surface` asks every active backend for a surface.
It fails only when every backend fails, and otherwise stores the surfaces that succeeded and logs the rest at debug level
([instance.rs L198-L258](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L198-L258)).

Source-established: for a Win32 handle, the backends fail differently.

| Backend | Failure behavior |
| --- | --- |
| DX12 | Wraps the HWND and returns `Ok`. Only a non-Win32 window handle returns `Err` ([dx12/instance.rs L129](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/instance.rs#L129)). |
| Vulkan | Returns `Err` when `hinstance` is missing or `VK_KHR_win32_surface` wasn't enabled ([L912](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/instance.rs#L912), [L491](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/instance.rs#L491)). A failing `vkCreateWin32SurfaceKHR` panics through `expect("Unable to create Win32 surface")` instead of returning `Err` ([L507](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/instance.rs#L507)). |

Source-established: when a backend has no surface, `get_capabilities_with_raw` returns `NotSupportedByBackend`, so all of that backend's adapters fail the compatibility check
([instance.rs L678](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L678)).
A Vulkan surface creation failure therefore silently hands selection to DX12.

### Filtering and Ordering

Source-established: `request_adapter` walks `instance_per_backend` in its insertion order, so Vulkan before DX12
([instance.rs L480](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L480)).
For each backend it enumerates adapters, passing the surface as a hint, applies the fallback filter when requested, and drops adapters whose surface capabilities fail
([L527-L545](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L527-L545)).
It then adjusts limits for indirect validation, filters features and limits, and applies `adapter_allowed`.
All three are surface-independent.
With `PowerPreference::None` the list keeps enumeration order.
`LowPower` and `HighPerformance` apply a stable sort by device type, with `Cpu` last in both
([L571-L605](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L571-L605)).
The first remaining adapter wins.
When none remains, the result is `RequestAdapterError::NotFound` with per-backend diagnostic bitsets.

Source-established: `PowerPreference` defaults to `None`, and Slint's default path uses `PowerPreference::from_env().unwrap_or_default()`
([adapter.rs L78](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/adapter.rs#L78),
[init.rs L66](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/util/init.rs#L66)).
Even with `WGPU_POWER_PREF=high`, a Vulkan and a DX12 view of the same discrete GPU tie, and the stable sort keeps Vulkan first.

Source-established: both Vulkan and DX12 HAL enumeration ignore the surface hint
([vulkan/instance.rs L940](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/instance.rs#L940),
[dx12/instance.rs L166](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/instance.rs#L166)).
So the surface affects selection only through per-backend surface creation and the capability filter.

### Vulkan Surface Filter

Source-established: Vulkan `surface_capabilities` returns `None`, which removes the adapter, when any of these holds
([swapchain/native.rs L60](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/swapchain/native.rs#L60)):

- the adapter's `can_present` flag is false
- `vkGetPhysicalDeviceSurfaceSupportKHR` for queue family 0 returns false or an error ([L67](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/swapchain/native.rs#L67))
- the surface capabilities, present modes or formats query returns an error

Only queue family 0 is checked.
Source-established: `expose_adapter` also hides any device whose first queue family lacks graphics, and any device whose driver reports conformance version 0 unless it is MoltenVK or `ALLOW_UNDERLYING_NONCOMPLIANT_ADAPTER` is set
([vulkan/adapter.rs L2311-L2358](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/adapter.rs#L2311-L2358)).
Those filters are surface-independent.

Documented: presentation support is a property of a physical device, a queue family and a specific surface
([vkGetPhysicalDeviceSurfaceSupportKHR](https://docs.vulkan.org/refpages/latest/refpages/source/vkGetPhysicalDeviceSurfaceSupportKHR.html)).

Source-established: the Optimus rule marks an Intel integrated GPU with Mesa older than 21.2 as unable to present when an NVIDIA discrete GPU and the `VK_LAYER_NV_optimus` layer are present.
It applies only on Linux
([vulkan/instance.rs L959-L993](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/instance.rs#L959-L993)).
The rule is computed at enumeration without a surface, but it only takes effect through the surface filter.
A surface-free selection would still choose such an adapter.

### DX12 Surface Filter

Source-established: DX12 `surface_capabilities` always returns `Some` for a Win32 surface.
A failing `GetClientRect` changes only the reported extent
([dx12/adapter.rs L1287-L1321](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/adapter.rs#L1287-L1321)).
DX12 adapters are never removed by the surface filter, provided the DX12 surface was created.

Source-established: DXGI adapters are enumerated with `EnumAdapters1` in index order.
Intel Haswell devices and the non-software duplicate WARP adapter are skipped
([dxgi/factory.rs L12-L50, L83](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/auxil/dxgi/factory.rs#L12-L50)).
Each remaining adapter is exposed only if `D3D12CreateDevice` at feature level 11_0 succeeds, and that device is kept inside the adapter
([dx12/adapter.rs L65-L103](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/adapter.rs#L65-L103)).

Documented: `EnumAdapters1` returns the adapter showing the desktop primary first, then other adapters with outputs, then adapters without outputs.
The factory snapshots the adapter set at creation
([EnumAdapters1](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-idxgifactory1-enumadapters1)).
Since Windows 8, the Microsoft Basic Render Adapter makes WARP present in DXGI adapter enumeration
([WARP guide](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/directx-warp)).

### `WGPU_ADAPTER_NAME`

Source-established: `initialize_adapter_from_env` enumerates `Backends::all()` without a surface hint, which on this instance means Vulkan adapters and then DX12 adapters.
It skips adapters that fail `is_surface_supported` and takes the first whose lowercase name contains the lowercase variable.
It panics when the variable is set and nothing matches
([init.rs L9-L44](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/util/init.rs#L9-L44)).
It ignores `WGPU_POWER_PREF`.
When the variable is unset, `initialize_adapter_from_env_or_default` falls through to `request_adapter`
([init.rs L57](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/util/init.rs#L57)).
The public `enumerate_adapters` passes `apply_limit_buckets: false` ([backend/wgpu_core.rs L82](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/backend/wgpu_core.rs#L82)).

The name path is surface-dependent in the same way as the default path, and the Slint gate already excludes it.

### Which Adapter Wins

The ordinary winner is the first Vulkan adapter, in loader order, whose surface check passes.
If no Vulkan adapter passes, or the Vulkan backend or its surface is missing, it is the first DX12 adapter in DXGI order.

| Machine | Likely ordinary winner | Label |
| --- | --- | --- |
| Desktop with one NVIDIA, AMD or Intel GPU and its vendor Vulkan ICD | That GPU through Vulkan | Source-established ordering. Hypothesis that the vendor ICD reports queue family 0 as presentable on a Win32 surface. |
| Hybrid laptop, integrated plus discrete, both with Vulkan ICDs | The first Vulkan physical device in the loader's system-preference order, commonly the one DXGI lists first | Documented loader sorting. Hypothesis that both can present to any HWND through Windows cross-adapter presentation, so the filter removes neither. |
| Machine without a Vulkan loader or ICD | The first DXGI adapter through DX12, normally the GPU driving the primary display | Source-established backend skip, Documented DXGI order |
| Windows on ARM64 with a Qualcomm Adreno driver | Adreno through Vulkan if the installed driver package includes a Vulkan ICD that exposes a conformant device, otherwise Adreno through DX12 | Hypothesis. Driver packages differ, and the conformance filter can hide a device. |
| Machine with Microsoft's OpenCL, OpenGL and Vulkan Compatibility Pack | Depends on whether its Vulkan-on-D3D12 driver reports a nonzero conformance version | Hypothesis |
| VM with only WARP | WARP through DX12, but only with `SLINT_WGPU_CPU=1`. Without it the Slint probe rejects WGPU. | Experiment (`windows-vm-default`, `windows-vm-dx12`) and Source-established probe |

Experiment: on the Windows 11 ARM64 VM, WGPU exposes only `Microsoft Basic Render Driver` (Dx12, `Cpu`).
The display device `VMware SVGA 3D` isn't exposed, and no Vulkan or GL adapter exists (`windows-vm-default`, `windows-vm-vulkan`, `windows-vm-gl`).
Hypothesis: SVGA 3D is absent because `D3D12CreateDevice` at feature level 11_0 fails for it, which makes `expose` return `None`.

The harness's Windows run is therefore a one-candidate case.
All four strategies pass with the default mask and with `WGPU_BACKEND=dx12`, but a single candidate can't diverge.

## Surface-Independent Work

This section characterizes the work without timing it.
Each row is Source-established from the linked code unless marked otherwise.

| Stage | Vulkan | DX12 | Needs the surface |
| --- | --- | --- | --- |
| Loader and library | Load `vulkan-1.dll`. Documented: the loader reads registry keys and loads each ICD library. | Load `d3d12.dll`, create the device factory, optionally enable the debug layer. Load `dxcompiler.dll` or FXC. | No |
| Instance | `vkCreateInstance`, which calls every ICD's instance creation | `CreateDXGIFactory2`, tearing support query | No |
| Enumeration | `vkEnumeratePhysicalDevices`, then property, feature, memory and queue-family queries per device | `EnumAdapters1`, then `D3D12CreateDevice` and feature queries for every adapter | No, the hint is ignored |
| Surface creation | `vkCreateWin32SurfaceKHR` | Wrap the HWND | Yes |
| Selection filter | Present support for queue family 0, capabilities, formats, present modes | Always passes | Yes |
| Device | `vkCreateDevice` and queue, then wgpu-core device resources | `CreateCommandQueue` on the device created during enumeration, then wgpu-core device resources | No, once the adapter is fixed |

Source-established: wgpu-core device construction builds the indirect-validation compute pipelines whenever `VALIDATION_INDIRECT_CALL` is set and the adapter supports indirect execution and compute
([device/resource.rs L501-L512](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L501-L512)).
This code is backend-independent, so on DX12 it reaches the DXC or FXC compiler and on Vulkan the driver's pipeline compiler.

Two facts shape what a worker could save.
First, on DX12 the expensive per-adapter `D3D12CreateDevice` happens during enumeration, not during `request_device`.
Second, Slint's probe already performs loader, instance and enumeration work on the main thread before the real instance repeats it.
Hypothesis: the real instance's repetition runs warmer than the probe, so moving only the repeated instance creation overlaps less than a cold count would suggest.

## Staging Options

The four options come from `research/non-metal-staging.md` in the evidence repository (`<metal-evidence>/`).
"Exact" below means the same adapter, the same device descriptor and the same error behavior as ordinary initialization, for the same enumeration snapshot.

| Option | Exact selection | Work it can overlap | Added complexity |
| --- | --- | --- | --- |
| 1. Instance only | Yes | Loader, ICD loading, `vkCreateInstance`, DXGI factory, `d3d12.dll`, DXC load | Low. Transfer an `Instance` and create the surface from it. |
| 2a. Early enumeration, then ordinary `request_adapter` | Yes | Only warming effects, since selection enumerates again | Low, but the benefit is Hypothesis only |
| 2b. Early enumeration, then late filter over the saved list | Yes under a fixed snapshot, with an ordinary fallback on no match | Option 1 plus all enumeration, including every `D3D12CreateDevice` and Vulkan property query | Moderate and version-sensitive |
| 3. Device after surface-aware selection | Yes | `request_device` only | Low for correctness. Useful overlap needs lifecycle changes in Slint. |
| 4. Speculative surface-free winner, validated later | Selection yes with fallback. Error and wake behavior no. | Everything except the surface filter | High |

### Option 1: Instance Only

Source-established: instance creation takes no surface, and Slint's descriptor is fully determined by the mask and environment (`default_instance_descriptor`, `wgpu_30.rs` L284).
The main thread then runs the unchanged `create_surface` and `initialize_adapter_from_env_or_default`.
Selection is exact because the selection code is unchanged.
The only difference is when the DXGI factory and the Vulkan loader take their adapter snapshot, and a hotplug between those moments is the only way to observe it.

Experiment: `worker-instance` matches `ordinary` on Metal, WARP and lavapipe (`results/2026-09-28/`).

Variant, Hypothesis on benefit: the probe instance has the same descriptor as the real one in the default configuration (`wgpu_30.rs` L180 and L284).
Keeping the probe's instance for the real selection instead of creating a second one would remove the repeated work without a thread.
That changes resource lifetime, not selection, and still needs its own tests.

### Option 2: Early Enumeration

2a repeats enumeration in `request_adapter`, so it is exact and saves only whatever warming the first pass leaves behind.

2b keeps the enumerated `Vec<Adapter>` and selects after the surface exists.
Source-established: with the public API, this reproduces `request_adapter` for Slint's options when it does the following.

1. Enumerate on the same instance with the same mask. The public call already applies the same limit adjustment, feature filtering and `adapter_allowed`, with no limit bucketing.
2. Keep adapters where `adapter.is_surface_supported(&surface)` is true. This covers a backend without a surface, because the check then fails with `NotSupportedByBackend`.
3. Apply the same stable sort when `WGPU_POWER_PREF` is set.
4. Take the first adapter. If none remains, call ordinary `request_adapter` instead.

Step 4 is required because `RequestAdapterError` is `#[non_exhaustive]` with hidden fields, so Slint can't construct the same error text
([adapter.rs L250](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/adapter.rs#L250)).
The fixed-snapshot premise is the residual gap.
A GPU added, removed or reset between enumeration and surface creation is visible to ordinary selection but not to the saved list.
The sort and filter must be re-verified on every WGPU upgrade.
Unselected DX12 adapters each hold a live `ID3D12Device` until the list is dropped, so the list must be released promptly after selection.

### Option 3: Device After Selection

Source-established: after ordinary selection, `request_device` depends only on the adapter and Slint's device descriptor (`default_device_descriptor`, `wgpu_30.rs`).
Moving it to a worker cannot change which adapter is used.
Experiment: `worker-after-select` matches `ordinary` on every archived platform.
Source-established: `async_init_instance_adapter_device_queue_surface` returns the device together with the surface, so the resuming caller can't proceed to surface configuration without it (`wgpu_30.rs` L332).
There is little independent work to overlap unless the lifecycle changes, as non-metal-staging.md also notes.

### Option 4: Speculative Winner

The worker enumerates without a surface, selects candidate A with the same policy, and creates a device on A.
After the surface exists, the main thread checks `A.is_surface_supported(&surface)`.

Source-established argument: surface-free and surface-aware `request_adapter` enumerate the same list and sort it the same way, and the surface filter only removes adapters.
So if A is first without the filter and survives it, A is also first with it.
If A fails, the worker's device is discarded and ordinary selection runs.
Under the fixed snapshot, this selects exactly the ordinary adapter.
Experiment: the harness's `worker-device` does exactly this and passes wherever only one adapter exists.

Selection is not the whole contract.
The next section covers what is lost.

## Can Speculation Be Correct?

The requirement is a strategy that never reports an error from an adapter ordinary selection would have skipped, and never initializes a different GPU unnecessarily.
For the mixed mask, no such strategy exists.

The reasons, each Source-established from the sections above unless marked:

- With any Vulkan adapter present, A is a Vulkan adapter, and its surface support is unknowable before the HWND exists.
  Any work done on A before that point is work ordinary selection might not do.
- Rust-level errors from `request_device` on A can be held and discarded, and an unwinding panic can be caught on the worker.
  Slint's current `PendingWgpuDevice::take` resumes worker panics on the consuming thread instead (`internal/backends/winit/wgpu_device.rs`).
- Native faults, aborts and hangs inside a Vulkan driver's `vkCreateDevice` can't be contained.
  Slint's `impl Drop for PendingWgpuDevice` joins the worker without a timeout, so a hang on a skipped adapter would block shutdown (same file).
- Drivers and WGPU log during device creation, and those messages can't be recalled.
- On Vulkan, `vkCreateDevice` is extra work on A beyond enumeration.
  Hypothesis: on a hybrid laptop it can power up a discrete GPU that ordinary selection would not have used.
- On DX12, enumeration already creates an `ID3D12Device` for every adapter in both paths, so the extra DX12 work is the command queue and wgpu-core resources.

A narrower, exact case does exist.
If the worker's enumeration returns no Vulkan adapters, because the Vulkan backend failed to initialize or has no devices, then the candidates are DX12 only.
DX12 surface creation for a Win32 handle cannot fail and DX12 never filters by surface, so A is the ordinary winner under the fixed snapshot.
This is a runtime check on the enumeration result, not on the compiled mask.
It helps only machines without a working Vulkan driver, which the table above suggests are uncommon outside VMs and older hardware (Hypothesis).
It also inherits the DXGI threading concerns from non-metal-staging.md.

## Changing the Windows Mask

Removing Vulkan from the Windows default would make the effective mask `DX12`, where selection is surface-independent.
That changes which adapter and API existing users get whenever a Vulkan adapter currently wins, and it changes environment handling.
The task rules forbid silently changing adapter preference, environment handling, features or limits, so this page treats it as out of scope.
A user who sets `WGPU_BACKEND=dx12` already opts into a DX12-only mask, which is the narrower extension non-metal-staging.md describes.

## Linux Vulkan

Source-established: on Linux `default_backends_to_avoid` removes `GL`, `METAL` and `DX12`, so the default effective mask is `VULKAN` alone.
The same `request_adapter` code applies.

Documented: since loader 1.3.204, Linux physical devices are sorted by type (discrete, integrated, virtual, others) and then by PCI location.
`VK_LOADER_DEVICE_SELECT` and `VK_LOADER_DISABLE_SELECT` change that order
([LoaderApplicationInterface, Physical Device Ordering](https://github.com/KhronosGroup/Vulkan-Loader/blob/main/docs/LoaderApplicationInterface.md#physical-device-ordering)).
CPU drivers such as lavapipe and SwiftShader sort last.

Surface-dependent removal on Linux is more likely than on Windows.

- Source-established: surface creation fails with `Err` when the ICD set lacks `VK_KHR_xlib_surface`, `VK_KHR_xcb_surface` or `VK_KHR_wayland_surface` for the handle type ([vulkan/instance.rs L884](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/instance.rs#L884)). With the Vulkan-only mask, that makes ordinary initialization fail outright.
- Source-established: the Optimus rule above removes old-Mesa Intel GPUs from presentation.
- Hypothesis: a GPU that can't present to the compositor's display, for example in some PRIME or headless-render-node setups, reports no present support for queue family 0 on that surface.

Experiment: lavapipe under Xvfb passes for all strategies (`docker-linux-arm64-vulkan-lavapipe`).
It is a single-adapter case.

## Harness Tests That Would Expose Divergence

None of these have been run.
Commands follow the harness README, and `--env` passes environment variables to every case.
Some tests need a Vulkan test layer that doesn't exist yet.
It would be a small implicit layer that forces `vkGetPhysicalDeviceSurfaceSupportKHR` to `VK_FALSE` for a device named in an environment variable, or hides `VK_KHR_*_surface` from instance extension enumeration.
It is a controlled test double, not a driver validation.

| # | Platform and drivers | Setup | Expected ordinary result | What exposes divergence |
| --- | --- | --- | --- | --- |
| 1 | Linux, lavapipe and SwiftShader, Xvfb | `VK_DRIVER_FILES=<lvp_icd.json>:<vk_swiftshader_icd.json>` | First sorted CPU device | `worker-device` and `ordinary` record the same order in `adapters_surface_free`. Repeat with the files reversed and with `VK_LOADER_DISABLE_SELECT=1` to check that order doesn't depend on thread or timing. |
| 2 | Same | Test layer denies present support for the first device | Second device | `worker-device` records `prepared_adapter_surface_check` false and falls back. The record shows a device created on an adapter ordinary never opened. |
| 3 | Same | Test layer denies present support for both | `NotFound` error | Every strategy reports the same error class, and `worker-device` reports no device-creation error from the discarded adapter. |
| 4 | Same | `WGPU_ADAPTER_NAME=swiftshader` and `WGPU_POWER_PREF=low` in separate runs | SwiftShader, then the stable-sorted first | `worker-device` must match, and the record's `slint_would_prepare_on_worker` must stay false for the name case. |
| 5 | Linux, lavapipe with Wayland (Weston headless) | Wayland session instead of Xvfb | lavapipe | Same comparison on a second WSI path. |
| 6 | Windows, WARP plus a Vulkan CPU ICD such as lavapipe from a Mesa build for Windows | Default mask, `SLINT_WGPU_CPU` semantics | lavapipe through Vulkan, ahead of WARP | This is the first real mixed-mask case. `adapters_surface_free` must list Vulkan before DX12, and every strategy must pick lavapipe. |
| 7 | Same | Test layer denies present support for lavapipe | WARP through DX12 | `worker-device` prepares lavapipe, fails the check, and falls back to WARP. This is the cross-backend divergence the Metal gate avoids. |
| 8 | Same | Test layer hides `VK_KHR_win32_surface` | WARP through DX12 | Vulkan surface creation returns `Err` for one backend only. `worker-device` again prepared the wrong backend. |
| 9 | Windows, WARP only | `WGPU_BACKEND=dx12`, then default mask with no Vulkan ICD | WARP | Confirms the runtime-degenerate case: the prepared adapter always passes. Already covered by `windows-vm-dx12` and `windows-vm-default`. |
| 10 | Windows, any | Instrument the harness to also record the Slint probe's result with and without `SLINT_WGPU_CPU` | Probe false for WARP only | Shows which configurations reach WGPU at all. |

Test 6 needs a Vulkan ICD for Windows ARM64 or x64.
Whether a Mesa lavapipe or SwiftShader build installs and presents on the ARM64 VM is Hypothesis until tried.
The CI workflow's Windows job on x64 runners is the easier host for tests 6 to 8.

Software drivers expose one CPU adapter each.
They exercise ordering, filtering and fallback logic, but not hybrid-GPU routing, GPU power-up, or vendor driver behavior.
Those need real multi-GPU hardware.

## Experiment Outcomes

The runs below came after this page was written.
Paths are under `results/2026-09-28/` unless they start with `docs/`.
The Windows runs used one Windows 11 ARM64 VM whose only adapter is WARP, so none of them has a second candidate.
No run exercised the mixed mask with a Vulkan adapter present, which is the case this page is about.

### Settled

- **Experiment, "Who Reaches This Path".** The consequence that Slint's `femtovg-wgpu` doesn't use WGPU on a WARP-only machine by default holds.
  Without `SLINT_WGPU_CPU`, the harness copy of the probe rejected WARP with the default mask and with `dx12` (`windows-vm-default-probe`).
  With `SLINT_WGPU_CPU=1`, the probe returned before creating an instance (`windows-vm-dx12-probe-slint-cpu`).
  In Slint itself, case c fell back to the software renderer (`slint-windows-run1`, `slint-windows-run2`).
- **Experiment, "Who Reaches This Path".** A DX12 gate declines the default mask in Slint.
  In case b of `docs/slint-windows-integration.md`, the follow-up branch traced `declined` and took the ordinary path.
- **Experiment, harness test 9.** `WGPU_BACKEND=dx12`, `dx12,gl` and the default mask all select WARP in every strategy.
  `dx12_gate_would_prepare` is true only for the first two (`windows-vm-dx12`, `windows-vm-dx12-gl-mask`, `windows-vm-default`).
- **Experiment, harness test 10.** The harness now records the probe's result.
  It is false for WARP without `SLINT_WGPU_CPU`, and with it the probe returns true without creating an instance, as the table predicted.
- **Experiment, "`WGPU_POWER_PREF`".** With one adapter, `low` and `high` select the same adapter, features and limits (`windows-vm-dx12-env-parity.md`).
  This can't test the tie between Vulkan and DX12 views of one GPU.

### Refined

- **Experiment, "Instance Construction".** A DX12 backend that fails `init` does leave the instance without that backend.
  With `WGPU_DX12_COMPILER=dxc` and no `dxcompiler.dll`, `wgpu_core::instance` logs `failed to create Dx12 backend` and no adapter exists (`windows-vm-dx12-compiler-dxc`).
  Surface-first strategies then fail at surface creation, and a surface-free worker fails at adapter selection.
  So the first error of a speculative strategy can differ even when no adapter is skipped.
  `docs/research/dx12-worker-safety.md` records this as a correction.

### Not Run

- No archived run covers harness tests 1 to 8.
  They need a second Vulkan ICD on Linux or Windows, and some need the test layer described above.
- The Hypothesis that SVGA 3D is absent because `D3D12CreateDevice` fails for it is untested.
- The Hypothesis that the real instance runs warmer than the probe's is untested, and no timing was taken.
