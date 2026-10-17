# DX12 Worker Preparation: Threading, Lifetime and Failure Safety

Question: is it correct to extend Slint's worker device preparation from an effective `METAL` mask on macOS to an effective `DX12` mask on Windows?

Prior research settled selection only.
DX12 enumeration ignores the surface and `surface_capabilities` always returns `Some`, so the surface-free winner equals the surface-aware winner.
This page covers threading, COM, window messages, lifetime, abandonment, failure ordering, environment handling and reach.

## Labels and Pins

Every claim carries one label.

- **Source**: established by reading pinned source.
- **Doc**: established by vendor documentation.
- **Experiment**: established by a harness run, with its command and output location.
- **Hypothesis**: not established, with the test that would settle it.

Pins:

- WGPU 30.0.1, git `40f4a34ebaf56f9a046231f54125ad046239d3f3`, cited as `W:` below.
  `W:` expands to `https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/`.
  Line numbers were checked against the crates.io registry copy of `wgpu-30.0.1`, `wgpu-core-30.0.1`, `wgpu-hal-30.0.1` and `wgpu-types-30.0.1`.
- Slint branch `wgpu-worker-preparation` at `97066a9e8bc29371339bd1b5279650bb811c1928`, upstream base `73e341928b4e857464b3ee7909ce7373cfd447e6`.
  The branch isn't published, so Slint citations name the worktree path `<slint-worktree>` and a line range at that commit.
- Winit 0.30.13, the version in Slint's `Cargo.lock`, cited as `https://github.com/rust-windowing/winit/blob/v0.30.13/<path>#L<n>`.
- Harness results in this repository under `results/2026-09-28/`.
  They were run with `scripts/run-matrix.py` in its default `release` profile, so the D3D12 debug layer was off.

## Verdict

The DX12 worker is sound on threading and COM grounds, going by source and vendor documentation.
It isn't ready to ship.
Two Slint-specific hazards have no Metal analogue and are untested.
The first is a debug-layer ordering hazard between the worker and Slint's main-thread GPU probe.
The second is a prepared WARP device that stays alive whenever the probe rejects WARP.
The gate also reaches very few users, because Slint's default Windows mask is `VULKAN | DX12`.
Only an explicit `WGPU_BACKEND` that masks to `DX12` qualifies.

## 1. What the Worker Would Run

### Instance Creation

**Source.** `wgpu_hal::dx12::Instance::init` runs these steps in order ([W:wgpu-hal/src/dx12/instance.rs#L17-L127](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/instance.rs#L17-L127)):

1. Load `d3d12.dll` (L19).
2. Build a `DeviceFactory`, which is `Legacy` unless an Agility SDK is configured (L24-L25).
3. Call `enable_debug_layer` (L27).
   With `VALIDATION` or `GPU_BASED_VALIDATION` set, it calls `D3D12GetDebugInterface` and `ID3D12Debug::EnableDebugLayer`, which is process-global on the legacy path ([W:wgpu-hal/src/dx12/device_creation.rs#L96-L133](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device_creation.rs#L96-L133)).
4. Load `dxgi.dll` and call `CreateDXGIFactory2`, with `DXGI_CREATE_FACTORY_DEBUG` only under `VALIDATION` ([W:wgpu-hal/src/auxil/dxgi/factory.rs#L144-L181](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/auxil/dxgi/factory.rs#L144-L181)).
5. Create `IDXGIFactoryMedia` and query tearing support (L32-L49).
6. Load a shader compiler (L52-L97).
   `Auto` tries `dxcompiler.dll` through `DxcCreateInstance`, then falls back to FXC's `D3DCompile` export.
   Neither goes through `CoCreateInstance` ([W:wgpu-hal/src/dx12/shader_compilation.rs#L299-L326](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/shader_compilation.rs#L299-L326)).
7. Build a `DCompLib` handle (L117).
   The DirectComposition device is created lazily at surface configuration, on the thread that configures ([W:wgpu-hal/src/dx12/dcomp.rs#L42-L74](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/dcomp.rs#L42-L74)).

**Source.** `wgpu-core` builds HAL instances in the order Vulkan, Metal, Dx12, Gles, and skips any backend not in the mask ([W:wgpu-core/src/instance.rs#L83-L92](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L83-L92)).
Under a `DX12` mask no other backend is loaded.

### Enumeration and Adapter Selection

**Source.** Enumeration calls `EnumAdapters1` in index order ([W:wgpu-hal/src/auxil/dxgi/factory.rs#L83-L110](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/auxil/dxgi/factory.rs#L83-L110)).
`Adapter::expose` then calls `D3D12CreateDevice` on every adapter to read its capabilities ([W:wgpu-hal/src/dx12/adapter.rs#L65-L100](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/adapter.rs#L65-L100)).
So the expensive native device creation happens during enumeration, not during `request_device`.

**Doc.** D3D12 devices are singletons per adapter in a process.
A second `D3D12CreateDevice` call for the same adapter LUID returns the existing device ([D3D12CreateDevice remarks](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-d3d12createdevice)).
The worker's device and any concurrent main-thread probe device are therefore the same COM object.

### Device Creation

**Source.** `Adapter::open` creates the direct command queue, an idle fence and a Win32 event ([W:wgpu-hal/src/dx12/adapter.rs#L1058-L1112](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/adapter.rs#L1058-L1112)).
`Device::new` registers a vectored exception handler under `VALIDATION`, creates the `gpu-allocator` allocator, another fence and the descriptor heaps ([W:wgpu-hal/src/dx12/device.rs#L42-L220](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L42-L220)).

**Source.** `wgpu-core` creates internal pipelines at device creation.
Indirect validation pipelines are built when `VALIDATION_INDIRECT_CALL` is set and the adapter supports compute and indirect execution ([W:wgpu-core/src/device/resource.rs#L501-L517](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L501-L517)).
`late_init_resources_with_queue` builds the timestamp normalizer and the default external-texture parameter buffer ([W:wgpu-core/src/device/resource.rs#L646-L662](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L646-L662), called from [W:wgpu-core/src/instance.rs#L883](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L883)).
These compile shaders with the loaded DXC or FXC on the worker.

**Source.** `InstanceFlags::from_build_config()` returns `DEBUG | VALIDATION | VALIDATION_INDIRECT_CALL` when `debug_assertions` is on, and `with_env` lets `WGPU_VALIDATION` and related variables override it ([W:wgpu-types/src/instance.rs#L253-L330](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/instance.rs#L253-L330)).
Slint uses exactly this for its default instance.
So debug builds of Slint apps run the D3D12 debug layer whenever the Graphics Tools optional feature is installed.

### COM Apartments

**Source.** Winit calls `OleInitialize` on the thread that creates a window with drag and drop enabled, making that thread an STA ([winit window.rs#L1168](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/windows/window.rs#L1168)).
It also lazily calls `CoInitializeEx(COINIT_APARTMENTTHREADED)` in a thread-local for taskbar calls ([winit window.rs#L1439-L1446](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/windows/window.rs#L1439-L1446)).
Neither WGPU's DX12 backend nor its DXGI helpers call `CoInitialize`, `OleInitialize` or `CoCreateInstance` (a search of `wgpu-hal/src/dx12`, `wgpu-hal/src/auxil`, `wgpu-core/src` and `wgpu/src` finds none).

**Doc.** Microsoft says COM initialization is needed only for objects created through `CoCreateInstance`.
Objects created through DirectX creation functions don't need it: "If you're creating objects indirectly, then the object creation method handles this task" ([Programming DirectX with COM](https://learn.microsoft.com/en-us/windows/win32/prog-dx-with-com)).

**Hypothesis.** The worker needs no `CoInitializeEx`, and objects created on it can be used from the STA main thread without marshaling.
DXGI and D3D12 objects aren't apartment-bound registered classes, and WGPU already relies on this across threads.
Test W3 below checks that the worker-created factory makes a working swap chain on the STA thread.

### Send, Sync and Thread-Local State

**Source.** Every DX12 HAL type that crosses the handoff has `unsafe impl Send` and `Sync`: `Instance`, `Adapter`, `Device` and `Queue` ([W:wgpu-hal/src/dx12/mod.rs#L603-L604](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/mod.rs#L603-L604), [#L714-L715](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/mod.rs#L714-L715), [#L802-L803](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/mod.rs#L802-L803), [#L857-L858](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/mod.rs#L857-L858)).
The public `wgpu` types are `Send + Sync` on native targets.
The handoff type `Result<WGPUConfiguration, Box<dyn Error + Send + Sync>>` already compiles on macOS and has no platform-specific bound.

**Source.** The `Surface` created on the main thread holds a clone of the instance's `IDXGIFactory` ([W:wgpu-hal/src/dx12/instance.rs#L150-L158](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/instance.rs#L150-L158)).
The main thread calls `CreateSwapChainForHwnd` and `MakeWindowAssociation` on that factory during configuration ([W:wgpu-hal/src/dx12/mod.rs#L1571-L1670](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/mod.rs#L1571-L1670)).
A factory created on the worker is thus used from the main thread.
WGPU documents the factory as free-threaded for its own cross-thread use ([W:wgpu-hal/src/auxil/dxgi/hdr.rs#L119-L124](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/auxil/dxgi/hdr.rs#L119-L124)).

**Source.** `wgpu-core`'s thread-locals exist only for lock-order checking and snatch-lock tracing ([W:wgpu-core/src/lock/ranked.rs#L87-L89](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/lock/ranked.rs#L87-L89), [W:wgpu-core/src/snatch.rs#L155-L157](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/snatch.rs#L155-L157)).
Both are set and cleared within one call, and the ranked checker is compiled only with `wgpu_validate_locks`.
No state carries across the handoff.

**Source.** The only process-global DX12 state touched on the worker is the vectored exception handler, which is counted under a mutex ([W:wgpu-hal/src/auxil/dxgi/exception.rs#L12-L28](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/auxil/dxgi/exception.rs#L12-L28)), plus the debug layer switch.

**Doc.** Any thread may submit to any command queue, and the runtime serializes submissions ([Design Philosophy of Command Queues and Command Lists](https://learn.microsoft.com/en-us/windows/win32/direct3d12/design-philosophy-of-command-queues-and-command-lists)).

**Experiment.** On the Windows 11 ARM64 VM, `worker-device` with `WGPU_BACKEND=dx12` hands a worker-created instance, adapter, device and queue to the main thread, then configures, renders and presents.
It matches `ordinary` in every scenario.
Command: `py -3.12 scripts\run-matrix.py --out runs\win-dx12 --env WGPU_BACKEND=dx12` in the interactive session.
Output: `results/2026-09-28/windows-vm-dx12/summary.md`.
WARP ("Microsoft Basic Render Driver", Dx12, Cpu) is the only adapter there.

## 2. DXGI Window Messages and the Join

**Doc.** DXGI sends window messages with `SendMessage` to the window associated with a swap chain.
A deadlock happens when a thread blocks the message pump while another thread's DXGI call waits for that pump ([DXGI Best Practices, Multithreading and DXGI](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/dxgi-best-practices)).
The documented triggers are swap-chain calls such as `Present`, `ResizeBuffers`, `ResizeTarget` and `SetFullscreenState` ([DXGI overview, Multithread considerations](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/d3d10-graphics-programming-guide-dxgi)).

**Source.** The worker makes no call that takes a window handle.
Factory creation, `EnumAdapters1`, `D3D12CreateDevice`, `CreateCommandQueue`, `CreateFence` and `CreateDescriptorHeap` take no `HWND`.
Surface creation, swap-chain creation, `MakeWindowAssociation`, `GetClientRect` and `Present` all stay on the main thread in Slint's `resume` path.
See `init_instance_adapter_device_queue_surface_then` in `internal/core/graphics/wgpu_30.rs` L490-L544 and `WGPUFemtoVGRenderer::resume` in `internal/backends/winit/renderer/femtovg.rs` L244-L308.

**Source.** Slint joins the worker on the main thread in `PendingWgpuDevice::take`, from inside `resume`, after `create_window` returns (`internal/backends/winit/wgpu_device.rs` L45-L49, `femtovg.rs` L250-L272).
At that moment the main thread isn't pumping messages.
The worker also starts before `EventLoop::new` (`internal/backends/winit/lib.rs` L496 vs L531), so it runs while Winit sets DPI awareness and creates its message-target window ([winit event_loop.rs#L184-L210](https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/windows/event_loop.rs#L184-L210)).

**Hypothesis.** The join can't deadlock through DXGI window messages.
The worker never holds an `HWND` or a swap chain, so DXGI has no window to message on its behalf.
The residual risk is a driver or DXGI internal that sends a synchronous message to a window it finds on its own, such as a broadcast.
Microsoft documents no such behavior for factory or device creation.
Test W4 blocks the main thread without pumping until the worker finishes.

**Experiment.** The harness `worker-device` strategy already joins on the main thread in `resumed`, before any message pumping after window creation.
It passed every scenario with WARP (`results/2026-09-28/windows-vm-dx12/summary.md` and `results/2026-09-28/windows-vm-default/summary.md`).
WARP is a user-mode driver, so this doesn't cover a hardware user-mode driver or a kernel-mode display driver.

**Hypothesis.** Early factory creation doesn't change DPI-dependent results.
The DXGI factory from the worker exists before `SetProcessDpiAwarenessContext` runs.
WGPU's surface extent comes from `GetClientRect` at configure time ([W:wgpu-hal/src/dx12/adapter.rs#L1287-L1310](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/adapter.rs#L1287-L1310)), and HDR queries use their own factory ([W:wgpu-hal/src/auxil/dxgi/hdr.rs#L110-L117](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/auxil/dxgi/hdr.rs#L110-L117)).
Test W8 checks swap-chain size at a non-100% scale.

## 3. Abandonment and Destruction

**Source.** `PendingWgpuDevice::drop` joins the worker before dropping its result (`wgpu_device.rs` L52-L61).
So the main thread never destroys objects while the worker is mid-call.
It waits for the worker to finish, then drops the finished `WGPUConfiguration` on the main thread.

**Source.** Dropping a `wgpu-core` queue calls the HAL `wait_for_idle` ([W:wgpu-core/src/device/queue.rs#L275-L290](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/queue.rs#L275-L290)).
On DX12 that signals a fence and waits on a Win32 event with `INFINITE` ([W:wgpu-hal/src/dx12/mod.rs#L1853-L1866](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/mod.rs#L1853-L1866)).
The wait uses a kernel event, not window messages, and nothing is in flight on an unconsumed device.
Device drop releases internal pipelines and buffers through the HAL ([W:wgpu-core/src/device/resource.rs#L306-L330](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L306-L330)).
All of these are reference-count releases with no thread affinity.

**Source.** Under `VALIDATION`, dropping the HAL adapter calls `ReportLiveDeviceObjects` and unwraps its result ([W:wgpu-hal/src/dx12/adapter.rs#L33-L46](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/adapter.rs#L33-L46)).
This happens on whichever thread drops the adapter, in both paths.

**Hypothesis.** Windows has no analogue of the Metal autorelease pool.
COM objects are released on the last `Release`, with no per-thread pool that must be drained.
The Windows join and drop should be a plain `join()` with the result dropped afterwards.
Adding `CoInitializeEx` or a message pump to the worker would add apartment state the ordinary path never has.

**Experiment.** `abandon-before-adapter`, `abandon-before-device` and `abandon-after-device` pass on the VM for all worker strategies.
In each, the worker's result is dropped on the main thread and an ordinary initialization afterwards still renders (`results/2026-09-28/windows-vm-dx12/summary.md`).

**Hypothesis.** Process exit with an unjoined worker is a Windows-specific risk.
`PendingWgpuDevice` is joined only when `SharedBackendData` is dropped.
If the backend is never dropped and the app exits before the first window resumes, `ExitProcess` terminates the worker.
It could stop inside `D3D12CreateDevice` or driver loading, before `DLL_PROCESS_DETACH` runs for `d3d12.dll` and the driver.
Microsoft warns that DXGI raises an exception when a factory's last reference is released from `DllMain` ([DXGI overview, DXGI Responses from DLLMain](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/d3d10-graphics-programming-guide-dxgi)).
The worker doesn't release from `DllMain`, so this is adjacent rather than direct evidence.
Test W6 exits immediately after backend creation.

## 4. Fallback and Failure Ordering in Slint

### Current Windows Behavior

**Source.** `default_backends_to_avoid()` returns `GL | METAL` on Windows (`internal/core/graphics/wgpu_30.rs` L149-L159).
`mask_backends` subtracts it and keeps the requested set only if the result would be empty (L161-L171).

**Source.** Before a window exists, `WGPUFemtoVGRenderer::new_suspended` calls `any_wgpu30_adapters_with_gpu` on the main thread (`femtovg.rs` L203-L219).
Unless `SLINT_WGPU_CPU` is set, the probe creates a throwaway instance, enumerates, and returns false when every adapter is `DeviceType::Cpu` (`wgpu_30.rs` L173-L241).
On false the renderer factory fails with `WGPU: No GPU adapters found`.
The fallback chain then tries the next compiled renderer (`internal/backends/winit/lib.rs` L172-L205).
The selector runs the same probe when a `WGPU30` API was requested (`internal/backends/selector/api.rs` L312-L324).
Explicit APIs are already excluded from the gate.

**Source.** The ordinary `resume` path creates the instance, then the surface, then the adapter, then the device (`wgpu_30.rs` L330-L463).
Errors appear in that order, wrapped as `WGPU initialization failed: ...` (L509-L519).
With a prepared result, `resume` joins first and wraps the worker's error in the same prefix (`femtovg.rs` L260-L272).
The surface is then created from the worker's instance through the `Manual` branch (`wgpu_30.rs` L367-L376).

### What a DX12 Worker Would Change

**Source.** The probe and the worker run concurrently.
The probe runs in `new_suspended` after `SharedBackendData::new` has started the worker.
With `WGPU_BACKEND=dx12`, both enumerate the same adapters, and both call `D3D12CreateDevice`, which returns the same singleton device.
This is a double probe, as the ordinary path also has.
Two instances now overlap in time instead of running one after the other.

**Hypothesis.** The debug-layer ordering is the one Windows-specific hazard.
**Doc.** Calling `EnableDebugLayer` after the D3D12 device exists "will cause the D3D12 runtime to remove the device" ([ID3D12Debug::EnableDebugLayer](https://learn.microsoft.com/en-us/windows/win32/api/d3d12sdklayers/nf-d3d12sdklayers-id3d12debug-enabledebuglayer)).
**Source.** Every DX12 instance calls `EnableDebugLayer` under `VALIDATION`, which is on by default in debug builds.
With the worker, the probe's instance can call it after the worker's device already exists.
The ordinary path already does the same with a second window, since each `resume` creates a new instance while the first window's device is alive.
If the runtime ignored a repeated enable, that case would already be safe.
Nothing documents that, and the harness never ran with the debug layer.
Test W2 settles it.

**Source.** WARP rejection leaves a prepared device alive.
When only WARP exists and `SLINT_WGPU_CPU` is unset, the probe fails and no WGPU renderer is created.
`take()` is never called, and the finished WARP device lives until `SharedBackendData` drops.
The ordinary path would never create a device in this case.
This is a resource and behavior difference, not a crash.
It's the VM's default configuration, and it's common in VMs, RDP sessions without GPU passthrough and CI.
Metal on macOS never exposes a CPU adapter, so the macOS branch has no counterpart.

**Source.** Error ordering changes only in theory.
With a prepared result, adapter and device errors surface before surface creation.
The DX12 `create_surface` only checks for a Win32 handle and stores it ([W:wgpu-hal/src/dx12/instance.rs#L129-L164](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/instance.rs#L129-L164)).
Winit on Windows always provides a Win32 handle.
So no surface error can precede an adapter error in the ordinary path, and the first error is the same.
The message prefix is identical in both paths.

**Source.** Instance failure behaves identically.
If `d3d12.dll`, the DXGI factory or both compilers fail, `wgpu-core` logs and leaves the DX12 backend out.
`request_adapter` then fails with no adapters in both paths ([W:wgpu-core/src/instance.rs#L480-L520](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L480-L520)).

**Experiment.** In session 0 over SSH, `Surface::configure` fails with `Invalid surface` under every strategy, including `ordinary` (`results/2026-09-28/windows-vm-ssh-session0/`).
The worker doesn't change that failure's class or phase.

### Environment Variables

**Source.** `WGPU_BACKEND` is read on the main thread in `PendingWgpuDevice::start` to compute the gate mask (`wgpu_device.rs` L29-L32).
The same mask is passed to the worker ([W:wgpu-types/src/backend.rs#L153-L160](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/backend.rs#L153-L160)).

**Source.** `WGPU_ADAPTER_NAME` disables preparation in `can_prepare` (`wgpu_device.rs` L63-L74).
Its no-match panic stays on the ordinary path.

**Source.** `WGPU_POWER_PREF`, the instance-flag variables (`WGPU_VALIDATION`, `WGPU_DEBUG`, `WGPU_GPU_BASED_VALIDATION`, `WGPU_VALIDATION_INDIRECT_CALL` and others), and the DX12 backend options (`WGPU_DX12_COMPILER`, `WGPU_DX12_PRESENTATION_SYSTEM`) are read on the worker.
They're read through `default_instance_descriptor` and `initialize_adapter_from_env_or_default` (`wgpu_30.rs` L284-L310, [W:wgpu/src/util/init.rs#L57-L70](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/util/init.rs#L57-L70), [W:wgpu-types/src/instance.rs#L300-L330](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/instance.rs#L300-L330)).
The functions are the same ones the ordinary path uses, so the values match whenever the environment is unchanged between backend creation and the first `resume`.

**Hypothesis.** A variable changed with `std::env::set_var` between backend creation and the first window takes effect on the ordinary path but not on the worker path.
This timing difference exists on macOS too.
It isn't a Windows-specific risk.

## 5. Reach

**Source.** Slint's femtovg WGPU 30 dependency uses default features, which include `dx12`, `metal`, `gles` and `vulkan` ([W:wgpu/Cargo.toml](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/Cargo.toml), `internal/renderers/femtovg/Cargo.toml` L44).
Cargo features are additive, so a downstream crate can't remove Vulkan.
`Instance::enabled_backend_features()` therefore includes `VULKAN` on Windows ([W:wgpu/src/api/instance.rs#L108-L130](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/api/instance.rs#L108-L130)).

**Source.** With no `WGPU_BACKEND`, the effective mask is `VULKAN | DX12`, and a gate testing `== DX12` fails.
The gate tests the mask, not installed drivers.
A machine with no Vulkan ICD still has `VULKAN` in its mask and is still excluded.
The VM shows this: no Vulkan adapter exists, yet the default mask is `VULKAN | DX12` (`results/2026-09-28/windows-vm-default/summary.md`).

**Source.** The gate passes only when `WGPU_BACKEND` masks to exactly `DX12`.
The qualifying values are `dx12`, `dx12,gl`, `dx12,metal` and `dx12,gl,metal`, because `GL` and `METAL` are subtracted.
Explicit `RequestedGraphicsAPI::Direct3D` and `WGPU30` configurations are excluded by `can_prepare`.
A Skia build is excluded unless the renderer is named `femtovg-wgpu`.
`renderer-femtovg-wgpu` is opt-in.

**Hypothesis.** Few users would reach the DX12 gate.
The benefit depends on users setting `WGPU_BACKEND=dx12` on an opt-in renderer.
Covering the default Windows configuration needs the `VULKAN | DX12` mask.
That mask needs the separate surface-dependent Vulkan proof from `non-metal-staging.md`.

## 6. What a DX12 Gate Would Change

This section describes the change and leaves the worktree untouched.

`internal/backends/winit/lib.rs`:

- Widen `#[cfg(all(target_os = "macos", feature = "renderer-femtovg-wgpu"))]` on `mod wgpu_device`, on the `SharedBackendData::pending_wgpu_device` field, on the `PendingWgpuDevice::start` call in `SharedBackendData::new`, and on the field initializer.
  The new predicate is `any(target_os = "macos", target_os = "windows")`.
  A `cfg_aliases` entry such as `wgpu_device_preparation` in `build.rs` keeps the four sites in sync.

`internal/backends/winit/renderer/femtovg.rs`:

- Widen the three `#[cfg(target_os = "macos")]` sites the same way.
  They're the `WGPUFemtoVGRenderer::shared_backend_data` field, its initializer in `new_suspended`, and the `take()` block in `resume`.
- In `new_suspended`, when `any_wgpu30_adapters_with_gpu` returns false, take and drop the pending result before returning the error.
  This releases a rejected WARP device at once.
  It also serves macOS.

`internal/backends/winit/wgpu_device.rs`:

- In `PendingWgpuDevice::start`, replace the `!= wgpu::Backends::METAL` comparison with a per-platform constant.
  It's `METAL` under `target_os = "macos"` and `DX12` under `target_os = "windows"`.
- Wrap the worker body in `objc2::rc::autoreleasepool` only under `target_os = "macos"`.
  On Windows, call the preparation function directly with no `CoInitializeEx`.
  `objc2` is a macOS-only dependency (`internal/backends/winit/Cargo.toml` L144-L146), so the call must be `cfg`-gated to compile.
- In `Drop for PendingWgpuDevice`, keep the join and use the autorelease pool only on macOS.
  On Windows, `let _ = task.join();` is the whole body.
  The drop of the returned `WGPUConfiguration` happens after the join on the same thread, as today.
- Keep `can_prepare` unchanged.
  Add a test for the Windows constant alongside `only_prepares_the_default_femtovg_device`.

`internal/core/graphics/wgpu_30.rs`:

- Rename `prepare_default_metal_device` to a backend-neutral name and widen its `#[cfg(all(target_os = "macos", feature = "unstable-wgpu-30"))]` to include Windows.
  The `unstable-wgpu-30` requirement stays, since `WGPUConfiguration::Manual` needs it.
- Change the `assert_eq!(..., wgpu::Backends::METAL)` to the same per-platform constant.
  Update the comment above it, which names only Metal as surface-independent.
- Leave `init_default_adapter_device_queue`, `default_instance_descriptor` and `mask_backends` unchanged, so both paths keep sharing them.

## Risks Blocking Ship

1. **Debug-layer ordering.** A main-thread probe instance calling `EnableDebugLayer` after the worker's device exists may remove that device in debug builds.
   Unknown until test W2 runs with Graphics Tools installed.
2. **Leaked WARP device on probe rejection.** Needs the `new_suspended` drop described above, or an argument that holding it is acceptable.
3. **No Slint-integrated Windows run.** The harness copies Slint's settings but doesn't run Slint's probe, fallback chain or `resume`.
4. **Hardware-driver coverage.** WARP is the only adapter tested.
   Hardware user-mode drivers and hybrid GPUs are unexercised.
5. **Process exit with an unjoined worker.** Unknown until test W6 runs.

## Tests the Harness Must Run on the Windows ARM64 VM

Run each in the interactive session through `scripts\windows\run-interactive.ps1`, since session 0 fails at `Surface::configure` for every strategy.

- **W1, baseline repeat.** Run `worker-device` against `ordinary` with `WGPU_BACKEND=dx12` and with `WGPU_BACKEND=dx12,gl`.
  Confirms the mask arithmetic and that both reach the same WARP adapter.
- **W2, debug layer.** Install the Graphics Tools optional feature and run with `--profile debug` and with `WGPU_VALIDATION=1` in release.
  Add a strategy step that creates a second DX12 instance on the main thread, concurrently with the worker, and enumerates.
  That mirrors `any_wgpu30_adapters_with_gpu`.
  Record `GetDeviceRemovedReason` after first present, all debug-layer messages, and whether `D3D12CreateDevice` fails.
  Compare with an `ordinary` run that makes the probe instance first and a second window later.
- **W3, cross-apartment use.** Confirm the main thread is an STA before configuring, with a window created with drag and drop enabled.
  Confirm the worker-created factory creates the swap chain and presents on it.
  Run with `WGPU_DX12_PRESENTATION_SYSTEM` set to both HWND and Visual, so DirectComposition is created on the STA thread.
- **W4, join without pumping.** Block the main thread in the join before the event loop pumps anything, with the worker delayed at each abandon point.
  Assert completion within the watchdog timeout.
  Repeat with a window already created and shown before the join.
- **W5, abandonment under validation.** Repeat `abandon-before-adapter`, `abandon-before-device` and `abandon-after-device` with `WGPU_VALIDATION=1`.
  Assert no panic from `ReportLiveDeviceObjects` and that a later ordinary device still renders.
- **W6, early process exit.** Start the worker, then exit the process at once from `main` without dropping the handle.
  Do it before the worker finishes and while it's inside device creation.
  Assert the process exits with its intended code within the timeout.
- **W7, WARP rejection.** Without `SLINT_WGPU_CPU`, mirror Slint's probe and its rejection of CPU adapters.
  Record whether a prepared device exists after rejection and when it's released.
  With `SLINT_WGPU_CPU=1`, confirm the worker and ordinary paths select the same WARP adapter.
- **W8, DPI.** Set display scaling to 150% and compare configured swap-chain size and `GetClientRect` between `ordinary` and `worker-device`.
- **W9, failure parity.** Repeat `failure-limits`, `failure-empty-mask` and `failure-unsupported-mask`.
  Add a run with `WGPU_DX12_COMPILER=dxc` and no `dxcompiler.dll`, so instance creation fails.
  Assert the same error class and phase as `ordinary`.
- **W10, environment parity.** Run with `WGPU_POWER_PREF=low` and `high`, `WGPU_DEBUG=1` and `WGPU_GPU_BASED_VALIDATION=1`.
  Assert identical adapter, features and limits.
  Run with `WGPU_ADAPTER_NAME` set and assert the gate declines.

## Experiment Outcomes

The sections above were written before the Windows runs.
This section records what the runs settled, and it corrects claims they contradicted.
Harness runs are indexed in `results/README.md`.
Slint-integrated runs are in `docs/slint-windows-integration.md` and `results/2026-09-28/exit-stress/README.md`.
All Windows runs used one Windows 11 ARM64 VM whose only adapter is WARP.
None of them covers a hardware driver.

### Tests W1 to W10

| Test | Status | Outcome | Evidence |
| --- | --- | --- | --- |
| W1 | Settled on WARP | `dx12` and `dx12,gl` both mask to `DX12`, and every strategy selects WARP. | `windows-vm-dx12`, `windows-vm-dx12-gl-mask` |
| W2 | Settled on WARP | A probe instance that enables the debug layer again didn't remove the worker's device. | `windows-vm-dx12-debug-hazards`, `windows-vm-dx12-validation`, `windows-vm-dx12-control` |
| W3 | Partly run | DirectComposition presentation works from a worker-created device. The main thread's apartment wasn't recorded. | `windows-vm-dx12-visual` |
| W4 | Settled on WARP | Plain joins that block for 300 to 750 ms without pumping all return. | `windows-vm-dx12-lifetime` |
| W5 | Settled on WARP | Abandonment under the debug layer caused no `ReportLiveDeviceObjects` panic. | `windows-vm-dx12-debug-hazards`, `windows-vm-dx12-validation` |
| W6 | Settled, with a Slint-specific failure | `exit` from `main` during a worker call exits cleanly. Slint's thread-local `Drop` aborted. | `windows-vm-dx12-lifetime`, `slint-windows-run1`, `exit-stress` |
| W7 | Settled on WARP | A rejected WARP device lived until process exit. | `windows-vm-default-probe`, `windows-vm-dx12-probe-slint-cpu` |
| W8 | Not run | DPI scaling wasn't varied. | none |
| W9 | Settled on WARP | A missing DXC produces a different first error on the worker. | `windows-vm-dx12-compiler-dxc` |
| W10 | Settled on WARP | Every variable left adapter, features and limits unchanged. | `windows-vm-dx12-env-parity.md` |

Paths are under `results/2026-09-28/`.

### W2: Debug-Layer Overlap

**Experiment.** In debug builds and with `WGPU_VALIDATION=1`, every device ran under the debug layer (`debug_device: true`).
The probe ran with the worker in instance creation, in enumeration, in `request_device`, and after it.
`GetDeviceRemovedReason` returned `S_OK` after the probe and after the first present in every strategy.

**Experiment.** In the positive control, only the probe enabled the layer, after a device made without it existed.
The device was removed with `0x887a0007` in `worker-device` and in `ordinary` alike (`windows-vm-dx12-control`).

This settles the Hypothesis in section 4 for WARP.
The overlap isn't a worker-specific hazard.
Removal follows the layer's state at device creation, and the ordinary path breaks the same way when flags change between instances.
The follow-up branch still declines when the flags contain `VALIDATION` or `GPU_BASED_VALIDATION`, until a hardware driver confirms this.

### W6: Exit With an Unjoined Worker

**Experiment.** `std::process::exit(7)` during instance creation, enumeration or `request_device` exited with 7 well inside the watchdog (`windows-vm-dx12-lifetime`).

**Experiment.** In Slint itself, returning from `main` with a component but no shown window aborted with `0xC0000409` at `adde532c9`.
It did so in 3 of 3 runs in `slint-windows-run1` and in 50 of 50 runs in `exit-stress/windows/windows-control-adde532c9-exit-immediately`.
The panic is `threads should not terminate unexpectedly`, from the join in `PendingWgpuDevice::drop`.

**Experiment.** With `Drop` and `discard` releasing without a join, the follow-up exited with 0 in 400 of 400 runs (`exit-stress/windows/`).

The Hypothesis in section 3 named the right risk but the wrong mechanism.
Nothing released a DXGI factory from `DllMain`.
Slint's backend lives in a thread-local, and `ExitProcess` had terminated the worker before that destructor joined it.

### W7: Rejected WARP Device

**Experiment.** Without `SLINT_WGPU_CPU`, the probe rejected WARP with the default mask and with `dx12`.
With the branch's behavior (`--on-probe-reject keep`), the prepared device was released only at exit.
`ordinary` created no device.
With `drop`, the device was released at the rejection.

**Experiment.** In Slint, `4b7fea77e` releases the device when the probe rejects the renderer, and case c falls back to the software renderer (`slint-windows-run1`, `slint-windows-run2`).
`b1dd5ac4f` then stops that release from joining, so the fallback doesn't wait for a device nobody uses.

### W9: Differing First Error

**Experiment.** With `WGPU_DX12_COMPILER=dxc` and no `dxcompiler.dll`, the DX12 backend fails to initialize and the instance has no backend.
Surface-first strategies fail at surface creation, while `worker-device` fails at adapter selection.
For Slint, a user would see `Failed to find an appropriate adapter` instead of `Error creating wgpu window surface`.

### Corrections

- **Correction, Experiment (W9).** Section 4 says instance failure behaves identically and that the first error is the same.
  The outcome is the same, but the first error differs when the instance ends up with no backend.
  `create_surface` fails when every backend fails, so the ordinary path reports a surface error first.
- **Correction, Experiment (W6).** Section 3 says the Windows join and drop should be a plain `join()` with the result dropped afterwards.
  Section 6 proposes `let _ = task.join();` as the whole Windows `Drop` body.
  That join aborts the process at exit, as shown above.
  `Drop` must release the claim without joining.
- **Correction, Source.** Step 3 of "Instance Creation" says `VALIDATION` or `GPU_BASED_VALIDATION` calls `EnableDebugLayer`.
  In wgpu-hal 30.0.1 only `VALIDATION` does.
  `GPU_BASED_VALIDATION` calls `ID3D12Debug1::SetEnableGPUBasedValidation` ([W:wgpu-hal/src/dx12/device_creation.rs#L127-L136](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device_creation.rs#L127-L136)).
  Both configure the debug layer, which Direct3D expects before any device exists.
- **Correction, Experiment (W2).** Section 4 calls debug-layer ordering "the one Windows-specific hazard".
  On WARP, it isn't specific to the worker.
  The Windows-specific failure the runs found was the exit abort under W6.

### Status of the Blocking Risks

1. Debug-layer ordering: not reproduced on WARP with matching flags. The follow-up still declines under validation flags.
2. Leaked WARP device on probe rejection: fixed by `4b7fea77e` and `b1dd5ac4f`.
3. No Slint-integrated Windows run: done, see `docs/slint-windows-integration.md`.
4. Hardware-driver coverage: still open. WARP is the only adapter tested.
5. Process exit with an unjoined worker: an abort was found and fixed, see W6.

### Still Hypothesis

- The COM apartment claims in section 1, beyond the DirectComposition run (W3).
- DPI independence of early factory creation (W8).
- The absence of DXGI broadcast messages during factory or device creation on hardware drivers.
  WARP joins didn't deadlock, which doesn't cover a kernel-mode display driver.
- The reach estimate in section 5.
  The gate behaves as described, but no data says how many users set `WGPU_BACKEND=dx12`.
