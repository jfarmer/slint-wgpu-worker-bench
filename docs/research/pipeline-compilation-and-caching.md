# Pipeline Compilation and Caching Across Metal, DX12 and Vulkan

This page compares where shader and pipeline compilation happens in WGPU 30.0.1 on Metal, DX12 and Vulkan.
It covers how that work affects Slint's FemtoVG WGPU startup and the macOS worker preparation on branch `wgpu-worker-preparation`.
Research date: 2026-09-28.

## Pins and Labels

- WGPU 30.0.1, git `40f4a34ebaf56f9a046231f54125ad046239d3f3`.
  Line numbers were checked against the crates.io registry copies of `wgpu`, `wgpu-core`, `wgpu-hal` and `wgpu-types` 30.0.1.
- FemtoVG 0.27.0, cited at `0eb55780894a9200e9a70ca3d16bd1c641281a44` as in the prior evidence, and checked against the registry copy.
- Slint worktree `<slint-worktree>` at `97066a9e8`.
  That branch isn't published, so Slint references name a file, a function and a line.

Every claim carries one label:

- **Source-established**: read from pinned source, with a link or file reference.
- **Documented**: stated in vendor documentation, with a link.
- **Experiment**: observed in an archived run, with the command and output location.
  No new runs were made for this page.
- **Hypothesis**: not yet checked.

## Summary

- Source-established: `request_device` eagerly builds two internal compute pipelines on all three backends, for indirect dispatch and indirect draw validation.
  This is the same code in `wgpu-core` for every backend, so DX12 and Vulkan have a direct analogue of the Metal device cost.
- Source-established: on DX12, the shader compiler DLL is loaded during instance creation, not device creation.
  With Slint's settings that is `dxcompiler.dll` if the loader finds it, otherwise `d3dcompiler_47.dll` (FXC).
- Source-established: no backend compiles to GPU code at `create_shader_module` for FemtoVG's shader.
  Metal and DX12 translate and compile at pipeline creation.
  Vulkan always defers SPIR-V generation to pipeline creation too, because `wgpu-hal` sets its separate-entry-point workaround unconditionally.
- Source-established: FemtoVG creates one shader module in its constructor and materializes render pipelines lazily during the first flush.
  All its pipelines share three entry points and one pipeline layout, and vary only in fixed-function state.
- Source-established: DX12 is the only backend with an in-process `wgpu-hal` cache of compiled shader bytecode.
  Metal and Vulkan call the platform compiler for every stage of every pipeline and rely on driver caches.
- Source-established: WGPU's explicit `PipelineCache` works only on Vulkan in 30.0.1.
  Slint's device would already have the feature enabled on Vulkan, but FemtoVG passes `cache: None`.
- Hypothesis: the worker should keep doing only instance, adapter and device work.
  Warming FemtoVG pipelines from the worker needs scene knowledge, duplicates work, and relies on driver caches that differ per backend.

## 1. Device-Internal Pipelines During `request_device`

### Shared `wgpu-core` Work

Source-established: `InstanceFlags::from_build_config` returns `VALIDATION_INDIRECT_CALL` in release builds and `debugging()` in debug builds, which also includes it
([wgpu-types/src/instance.rs L268](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/instance.rs#L268)).
The doc comment above that function says release builds return `empty()`, which the body contradicts.
`WGPU_VALIDATION_INDIRECT_CALL` overrides it through `with_env`
([L324](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/instance.rs#L324)).

Source-established: Slint builds its instance descriptor with `InstanceFlags::from_build_config().with_env()` and `BackendOptions::from_env_or_default()`
(`internal/core/graphics/wgpu_30.rs`, `default_instance_descriptor`, L284).
The macOS worker uses the same descriptor through `prepare_default_metal_device` (L316).

Source-established: `Device::new` enables indirect validation when the flag is set, the adapter reports `INDIRECT_EXECUTION` and `COMPUTE_SHADERS`, and at least two storage buffers per stage are allowed
([wgpu-core/src/device/resource.rs L501](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L501)).
It then calls `IndirectValidation::new` before returning
([L508](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L508)).

Source-established: `IndirectValidation::new` builds `Dispatch` and `Draw` eagerly
([wgpu-core/src/indirect_validation/mod.rs L32](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/indirect_validation/mod.rs#L32)).

- `Dispatch::new` formats a WGSL source with `max_compute_workgroups_per_dimension` baked in, parses and validates it with naga, and calls the HAL's `create_shader_module` and `create_compute_pipeline`
  ([dispatch.rs L42](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/indirect_validation/dispatch.rs#L42),
  [L121](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/indirect_validation/dispatch.rs#L121),
  [L212](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/indirect_validation/dispatch.rs#L212)).
- `Draw::new` parses `validate_draw.wgsl` and creates a compute pipeline with two pipeline-overridable constants, `supports_indirect_first_instance` and `write_d3d12_special_constants`
  ([draw.rs L61](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/indirect_validation/draw.rs#L61),
  [L565](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/indirect_validation/draw.rs#L565),
  [L612](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/indirect_validation/draw.rs#L612)).
  The second constant is true only on DX12, so the DX12 draw shader differs from the others.

Source-established: `create_device_and_queue_from_hal` calls `Device::new` and then `late_init_resources_with_queue`
([wgpu-core/src/instance.rs L876](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L876),
[L883](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/instance.rs#L883)).
`late_init_resources_with_queue` creates a `TimestampNormalizer` and writes the default external texture parameters
([resource.rs L646](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L646)).
The normalizer returns without a pipeline unless `AUTOMATIC_TIMESTAMP_NORMALIZATION` is set
([timestamp_normalization/mod.rs L86](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/timestamp_normalization/mod.rs#L86)).
Slint's flags don't set it, so no timestamp pipeline is built.

Source-established: `wgpu-core` creates internal pipelines only in `indirect_validation` and `timestamp_normalization`.
The only other HAL pipeline calls in the crate serve user pipelines
([resource.rs L4124](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L4124),
[L4988](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L4988)).
So there are no clear or blit helper pipelines, and `command/clear.rs` creates none.

### Per-Backend Consequences

| Backend | Work in `request_device` for the two validation pipelines | Eager or lazy | Label |
| --- | --- | --- | --- |
| Metal | Naga to MSL, `newLibraryWithSource`, `newComputePipelineStateWithDescriptor` for each | Eager | Source-established |
| DX12 | Naga to HLSL, `D3DCompile` (FXC) or DXC `Compile`, `CreateComputePipelineState` for each | Eager | Source-established |
| Vulkan | Naga to SPIR-V, `vkCreateShaderModule`, `vkCreateComputePipelines` with a null cache for each | Eager | Source-established |

Sources: Metal [load_shader L162](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/metal/device.rs#L162) and [compute pipeline L1798](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/metal/device.rs#L1798).
DX12 [load_shader L282](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L282) and [compute pipeline L2198](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L2198).
Vulkan [compile_stage L699](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/device.rs#L699) and [vkCreateComputePipelines L2257](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/device.rs#L2257).

Source-established: apart from these two pipelines, the DX12 HAL's device setup creates command signatures for indirect commands, which involve no shader compilation
([dx12/device.rs L244](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L244)).
No Vulkan or Metal HAL code creates other internal pipelines during device open.

Hypothesis: WARP and lavapipe report both downlevel flags and enough storage buffers, so indirect validation was active in the archived Windows and Linux runs.
The records don't show this, and the instrumentation section says how to check it.

### What This Means for the Metal 50 to 60 ms

Experiment (prior evidence, `<metal-evidence>/research/device-preparation-scenarios.md`): on an Apple M4 Max, Slint's instrumented device interval had a median near 2.8 ms by default, 1.5 ms with `WGPU_VALIDATION_INDIRECT_CALL=0`, and 57.8 ms with `MTL_SHADER_CACHE_SIZE=0`.
Four pairs per condition, run as separate blocks, so the differences aren't isolated compiler times.

Hypothesis: most of the 50 to 60 ms under the override is the two validation pipelines missing the Metal compiler cache.
The first is a two-pipeline source-to-AIR-to-GPU compile, and the default-condition drop with validation off points the same way.
A run with both `MTL_SHADER_CACHE_SIZE=0` and `WGPU_VALIDATION_INDIRECT_CALL=0` would settle it.

Hypothesis: DX12 and Vulkan pay the same structural cost of two eager compute pipeline compiles.
Its size depends on the HLSL compiler (FXC is a CPU compiler in-process), on the driver back end, and on driver caches.
Nothing in the archived runs isolates it.

Experiment (this harness, `results/2026-09-28/*/ordinary__single.json`, `phases`): single, unrepeated smoke runs recorded these device intervals.
They support no performance claim and are listed only to show which phase the harness can already see.

| Run | Instance | Adapter | Device | First pipeline |
| --- | ---: | ---: | ---: | ---: |
| `macos-metal-default`, ordinary | 0.10 ms | 0.19 ms | 5.48 ms | 1.07 ms |
| `windows-vm-dx12`, ordinary (WARP) | 6.04 ms | 11.18 ms | 18.24 ms | 3.31 ms |
| `docker-linux-arm64-vulkan-lavapipe`, ordinary | 13.06 ms | 1.16 ms | 16.70 ms | 0.71 ms |

The harness's first pipeline is a trivial triangle pipeline (`src/render.rs`), not a FemtoVG pipeline.

## 2. The Shader Path Per Backend

### Metal: WGSL to MSL to AIR to GPU Code

Source-established: Metal `create_shader_module` stores the naga module and compiles nothing
([metal/device.rs L1268](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/metal/device.rs#L1268)).

Source-established: at pipeline creation, `load_shader` applies pipeline constants, writes MSL with `naga::back::msl::write_string`, and calls `newLibraryWithSource_options_error`, once per stage
([L257](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/metal/device.rs#L257),
[L283](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/metal/device.rs#L283)).
A render pipeline then calls `newRenderPipelineStateWithDescriptor_error`
([L1751](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/metal/device.rs#L1751)).

Source-established: the Metal HAL keeps no library cache of its own and never uses `MTLBinaryArchive`.
A search of `wgpu-hal/src/metal/` finds no binary archive use, and `create_pipeline_cache` returns an empty struct
([L1908](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/metal/device.rs#L1908)).

Documented: Metal compiles source to an intermediate representation, then compiles that to GPU-specific code for every pipeline state.
The system Metal shader cache stores the results to speed up later pipeline creation
([WWDC20 session 10615](https://developer.apple.com/videos/play/wwdc2020/10615/)).

### DX12: WGSL to HLSL to DXBC or DXIL to GPU Code

Source-established: `Dx12Compiler` has four variants, and `#[default]` is `Auto`
([wgpu-types/src/backend.rs L779](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/backend.rs#L779)).
The doc comment on `Fxc` still says "(default)", which the attribute contradicts.
`WGPU_DX12_COMPILER` accepts `Fxc`, `Dxc` or `DynamicDxc`, and `StaticDxc`
([L816](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/backend.rs#L816)).

Source-established: the DX12 instance resolves the compiler during `Instance::new`
([wgpu-hal/src/dx12/instance.rs L52](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/instance.rs#L52)).
`Auto` uses static DXC when the `static_dxc` cfg is set.
Otherwise it tries to load `dxcompiler.dll` and falls back to FXC only when that load fails
([L64](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/instance.rs#L64)).

Source-established: the `static_dxc` cfg requires Windows, the `static-dxc` feature, MSVC, and a non-aarch64 target
([wgpu-hal/build.rs L32](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/build.rs#L32)).
`static-dxc` isn't a default feature of `wgpu` 30.0.1.
Slint's workspace declares `wgpu-30` with `default-features = false` (root `Cargo.toml`, L158), and the FemtoVG renderer re-enables default features (`internal/renderers/femtovg/Cargo.toml`, L44).
No Slint source mentions `static-dxc`, `Dx12Compiler` or `WGPU_DX12_COMPILER`.
So Slint on Windows uses dynamic DXC when `dxcompiler.dll` loads, and FXC otherwise.

Source-established: dynamic DXC loads `dxcompiler.dll` through `libloading`, creates an `IDxcCompiler3`, and reads its version to cap the shader model
([shader_compilation.rs L49](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/shader_compilation.rs#L49)).
FXC loads `d3dcompiler_47.dll` and resolves `D3DCompile`
([L163](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/shader_compilation.rs#L163)).
With a bare file name, `libloading` uses the standard Windows DLL search order.

Documented: `D3DCompiler_47.dll` ships with Windows 8.1 and Windows 10 and later
([Chuck Walbourn, HLSL, FXC, and D3DCompile](https://walbourn.github.io/hlsl-fxc-and-d3dcompile/)).
Hypothesis: a stock Windows 11 install has no `dxcompiler.dll` on the search path, so Slint applications that don't ship one use FXC.
The archived Windows records don't log which compiler loaded.

Source-established: DX12 `create_shader_module` stores the naga module and compiles nothing
([dx12/device.rs L1866](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L1866)).
At pipeline creation, `load_shader` writes HLSL and builds a key from the HLSL source, entry point, stage and shader model
([L390](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L390)).
A hit in the device's in-memory `shader_cache` skips the compiler
([L411](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L411)).
A miss calls FXC or DXC and inserts the result, keeping recently used entries once the map exceeds 200
([L427](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L427),
[L447](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L447)).
The render pipeline then calls `CreateGraphicsPipelineState`
([L2174](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L2174)).

Documented: the D3D12 runtime and drivers keep implicit shader caches, including a runtime-managed cache of driver compilations and a cache of DXBC to DXIL conversions
([D3D12_SHADER_CACHE_KIND_FLAGS](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/ne-d3d12-d3d12_shader_cache_kind_flags)).
Hypothesis: with FXC, the driver compile at `CreateGraphicsPipelineState` may include a DXBC to DXIL conversion that the conversion cache can serve on later launches.

### Vulkan: WGSL to SPIR-V to GPU Code

Source-established: Vulkan `create_shader_module` keeps the naga module as `Intermediate` when the `SEPARATE_ENTRY_POINTS` workaround is set or the module has overrides
([vulkan/device.rs L1855](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/device.rs#L1855)).
The adapter sets that workaround for every device, with a `TODO: only enable for particular devices` comment
([vulkan/adapter.rs L2299](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/adapter.rs#L2299)).
So `create_shader_module` never writes SPIR-V for a naga input in 30.0.1.

Source-established: `compile_stage` applies pipeline constants, writes SPIR-V for one entry point, and creates a fresh `VkShaderModule` for every stage of every pipeline
([L699](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/device.rs#L699)).
`vkCreateGraphicsPipelines` then receives the WGPU `PipelineCache` if one was passed, otherwise `VK_NULL_HANDLE`
([L2164](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/device.rs#L2164)).
The Vulkan HAL has no in-process cache of SPIR-V or pipelines.

### Where Compilation Happens

| Backend | At `create_shader_module` | At `create_*_pipeline` | In-process `wgpu-hal` cache | Label |
| --- | --- | --- | --- | --- |
| Metal | Store naga module | MSL write, `newLibraryWithSource` per stage, PSO | None | Source-established |
| DX12 | Store naga module | HLSL write per stage, FXC or DXC on cache miss, PSO | Per-device HLSL to bytecode map | Source-established |
| Vulkan | Store naga module | SPIR-V write and `VkShaderModule` per stage, PSO | None | Source-established |

Source-established: in every backend, `wgpu-core` parses and validates WGSL with naga at `create_shader_module`
([wgpu-core/src/device/resource.rs L2363](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L2363)).
That is CPU work at FemtoVG construction.

## 3. FemtoVG Pipeline Creation in Slint

Source-established: Slint constructs FemtoVG's renderer on the main thread in `configure_surface_from_init_result`, after `surface.configure`
(`internal/renderers/femtovg/wgpu.rs`, L397, with `WGPURenderer::new` at L441).
On macOS, the prepared device reaches that function through the worker's `WGPUConfiguration::Manual` result
(`internal/backends/winit/renderer/femtovg.rs`, L266, `pending_wgpu_device.take()`).

Source-established: `WGPURenderer::new` calls `create_shader_module` once for `wgpu/shader.wgsl` and starts with an empty `pipeline_cache` map
([src/renderer/wgpu.rs L355](https://github.com/femtovg/femtovg/blob/0eb55780894a9200e9a70ca3d16bd1c641281a44/src/renderer/wgpu.rs#L355)).
Pipelines are created on demand during a flush, keyed by `PipelineState`
([L2023](https://github.com/femtovg/femtovg/blob/0eb55780894a9200e9a70ca3d16bd1c641281a44/src/renderer/wgpu.rs#L2023)).
`materialize` calls `create_render_pipeline` with `cache: None`
([L1531](https://github.com/femtovg/femtovg/blob/0eb55780894a9200e9a70ca3d16bd1c641281a44/src/renderer/wgpu.rs#L1531)).

Source-established: `PipelineState` holds shader type, glyph texture use, render-to-texture, color target state (format and blend), topology, cull mode, and stencil state
([L1459](https://github.com/femtovg/femtovg/blob/0eb55780894a9200e9a70ca3d16bd1c641281a44/src/renderer/wgpu.rs#L1459)).
The WGSL has no `override` declarations and reads the shader type from a uniform.
Every pipeline uses `vs_main` or `vs_main_texture` with `fs_main`, one pipeline layout, and one vertex buffer layout.
So the generated MSL, HLSL or SPIR-V for a given entry point is identical across FemtoVG pipelines.
Only fixed-function state differs.

Consequences per backend, all Source-established from the paths above:

- Metal: each new FemtoVG pipeline calls `newLibraryWithSource` twice on one of three distinct sources, then builds a PSO.
  Reuse of the library compile depends on Apple's caches.
- DX12: each new pipeline rewrites HLSL, but at most three FXC or DXC compiles happen per device.
  Every pipeline still pays `CreateGraphicsPipelineState`.
- Vulkan: each new pipeline writes SPIR-V twice, creates two shader modules, and calls `vkCreateGraphicsPipelines` without a cache.

Experiment (femtovg-cache-bench, `~/github/femtovg-cache-bench/results/metal-cache-control-probe/README.md`, `python3 scripts/probe-metal-cache.py --build --out runs/my-metal-probe`): on an M4 Max, a first frame that creates 17 FemtoVG pipelines took about 14 to 15 ms with the system cache and about 487 ms with `MTL_SHADER_CACHE_SIZE=0`.
Repeated creation of the same 17 settled near 5.8 ms per frame under both settings.
Three launches per condition, so this is diagnostic.

Experiment (femtovg-cache-bench, `REPORT.md`, `python3 scripts/run-report.py --out runs/my-pr-report`): with warm caches, recreating 65 pipelines per frame cost about 27 ms per frame on the same machine.
The primary report's override diagnostic created 17 pipelines in 15.8 ms unset and 614 ms with the override.

Experiment (femtovg-cache-bench, `docs/application-context.md`): one Slint application created 32 FemtoVG pipelines at startup.
That application isn't reproduced in the bench.

Hypothesis: on Metal, a cold first frame costs more than the cold validation pipelines in `request_device`.
It is roughly 17 to 32 pipelines against 2, and each FemtoVG pipeline has two stages.
The worker doesn't touch that cost.

### Could or Should the Worker Create FemtoVG Pipelines

Speculative rendering is out of scope.
The narrower question is whether the worker could create FemtoVG pipelines or warm caches for them.

- Source-established: the worker can't fill FemtoVG's map.
  The map is private to `WGPURenderer`, and `WGPURenderer` holds `Rc` values, so it can't be built on the worker and moved.
  FemtoVG also creates its own shader module and has no API to accept pipelines or a `PipelineCache`.
- Source-established: `PipelineState` includes the surface format and stencil state.
  Slint picks the format after `get_capabilities` on the real surface (`configure_surface_from_init_result`), which the worker doesn't have.
- Experiment (`results/2026-09-28/macos-metal-*`): every Metal run configured `Bgra8Unorm`.
  Hypothesis: that format is stable on macOS, so it is not the blocking constraint there.
- Hypothesis: the worker could create throwaway pipelines with FemtoVG-identical descriptors to warm the Metal system cache or the DX12 per-device bytecode map.
  This duplicates PSO work, needs FemtoVG's exact WGSL and state set, and depends on the scene.
  On Vulkan it helps only if the driver caches internally.
- Hypothesis: moving FemtoVG's WGSL parse and validation to the worker needs a FemtoVG API change and saves CPU work that no archived run isolates.

Recommendation, Hypothesis: keep the worker to instance, adapter and device.
Measure the first-frame pipeline count and cost before proposing any FemtoVG change.

## 4. Driver Caches and Explicit Cache APIs

### WGPU `PipelineCache`

Source-established: `Features::PIPELINE_CACHE` lists Vulkan as supported and DX12 and Metal as unimplemented
([wgpu-types/src/features.rs L1163](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/features.rs#L1163)).
Only the Vulkan adapter reports it
([vulkan/adapter.rs L677](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/adapter.rs#L677)).
The DX12 and Metal HALs return an empty `PipelineCache`
([dx12/device.rs L2269](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L2269),
[metal/device.rs L1908](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/metal/device.rs#L1908)).

Source-established: `PIPELINE_CACHE` isn't in `all_experimental_mask`
([features.rs L1855](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/features.rs#L1855)).
Slint requests the adapter's features minus that mask (`internal/core/graphics/wgpu_30.rs`, `default_device_descriptor`, L273).
So Slint's Vulkan device already has the feature, and only FemtoVG's `cache: None` stands in the way.

Source-established: `Device::create_pipeline_cache` validates saved data against the adapter and a key built from the driver version
([wgpu-core/src/device/resource.rs L5098](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/device/resource.rs#L5098),
[vulkan/adapter.rs L2890](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/adapter.rs#L2890)).
With `fallback: true`, stale data becomes an empty cache.
`wgpu::util::pipeline_cache_key` returns a file name for Vulkan and `None` for other backends
([wgpu/src/util/mod.rs L163](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/util/mod.rs#L163)).
`PipelineCache::get_data` returns the bytes to save
([wgpu/src/api/pipeline_cache.rs L87](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu/src/api/pipeline_cache.rs#L87)).
The internal validation pipelines always pass `cache: None`
([draw.rs L612](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-core/src/indirect_validation/draw.rs#L612)).

Documented: a `VkPipelineCache` lets pipeline construction results be reused between pipelines and between runs, when the application saves and reloads its data
([VkPipelineCache](https://docs.vulkan.org/refpages/latest/refpages/source/VkPipelineCache.html)).

### Platform Caches

| Cache | Scope | Persists across processes | Used by WGPU 30.0.1 | Label |
| --- | --- | --- | --- | --- |
| Metal system shader cache | System, implicit | Yes | Implicitly, by every `newLibraryWithSource` and PSO call | Documented ([WWDC20 10615](https://developer.apple.com/videos/play/wwdc2020/10615/)) |
| `MTLBinaryArchive` | Application | Yes, when serialized | No | Documented (same talk), Source-established for non-use |
| D3D12 implicit caches (runtime cache for driver, DXBC to DXIL conversions, driver-managed) | System and driver | Yes | Implicitly, at every PSO creation | Documented ([kind flags](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/ne-d3d12-d3d12_shader_cache_kind_flags)) |
| `ID3D12PipelineLibrary` and `D3D12_CACHED_PIPELINE_STATE` | Application | Yes, when serialized | No, `CachedPSO` is always default | Documented ([CreatePipelineLibrary](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12device1-createpipelinelibrary)), Source-established ([dx12/device.rs L2219](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L2219)) |
| WGPU DX12 bytecode map | Device | No | Yes | Source-established |
| Vendor Vulkan driver caches, Mesa on-disk cache | Driver | Yes | Implicitly | Documented for Mesa ([Mesa environment variables](https://docs.mesa3d.org/envvars.html)) |
| `VkPipelineCache` through WGPU `PipelineCache` | Application | Yes, when saved | Only when the caller passes one | Source-established |

Documented: `ID3D12PipelineLibrary` blobs fail with `D3D12_ERROR_DRIVER_VERSION_MISMATCH` or `D3D12_ERROR_ADAPTER_NOT_FOUND` after a driver or hardware change
([CreatePipelineLibrary](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12device1-createpipelinelibrary)).

### Cold and Warm Startup

Hypothesis: on all three backends, warm startup is the common case for a repeatedly launched application.
Cold startup follows a first install, a driver or OS update, or cache eviction.
The worker hides only the part of the cold cost inside `request_device`.

The controls available differ:

- Metal: no documented reset.
  `MTL_SHADER_CACHE_SIZE=0` is undocumented and not a verified global reset, as the femtovg-cache-bench probe shows by the in-process reuse it leaves in place.
- D3D12: `ID3D12Device9::ShaderCacheControl` can disable, enable or clear the implicit caches, but only in developer mode
  (Documented, [ShaderCacheControl](https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12device9-shadercachecontrol)).
  Hypothesis: it isn't known whether WARP uses those caches.
- Mesa: `MESA_SHADER_CACHE_DISABLE=true` disables the on-disk cache, and `MESA_SHADER_CACHE_DIR` redirects it
  (Documented, [Mesa environment variables](https://docs.mesa3d.org/envvars.html)).
  A fresh directory per launch gives a disk-cold run, and a reused one a disk-warm run.
  Hypothesis: lavapipe and llvmpipe use the Mesa disk cache in the container's Mesa 25.2.8.
- Vulkan through WGPU: a harness-owned `PipelineCache` with and without saved data measures the application cache path directly.

## 5. What Software Drivers Can and Can't Tell Us

Experiment (`results/2026-09-28/windows-vm-default`, `windows-vm-dx12`): the Windows ARM64 VM exposes only WARP ("Microsoft Basic Render Driver", DX12, `Cpu`), not the VMware SVGA 3D display device.
No Vulkan or GL adapter exists there (`windows-vm-vulkan`, `windows-vm-gl`).
Experiment (`docker-linux-arm64-vulkan-lavapipe`): lavapipe passes all strategies.
Experiment (`docker-linux-arm64-gl-llvmpipe`): the surface-free llvmpipe adapter can't present, as recorded in `results/README.md`.

They can tell us:

- Source-established: the WGPU-side work is the same code on any adapter.
  That covers naga translation, the DX12 bytecode map, per-pipeline SPIR-V writes, and the two eager validation pipelines.
  Software drivers exercise all of it.
- Hypothesis: FXC and DXC run on the CPU regardless of adapter, so on WARP their cost is close to what a physical GPU sees for the HLSL step.
  Logging which compiler loaded and timing `load_shader` would confirm the split.
- Hypothesis: the pipeline counts, and whether a phase runs on the worker or the main thread, carry over directly.
- Hypothesis: Mesa's disk cache behaves the same way for lavapipe as for hardware Mesa drivers, so disk-cold versus disk-warm can be tested there.

They can't tell us:

- Hypothesis: back-end compile cost on a physical GPU.
  WARP and lavapipe compile to CPU code through their own JIT, with different costs and cache behavior than NVIDIA, AMD, Intel, Qualcomm or Apple compilers.
- Hypothesis: whether a vendor's implicit cache hits on a second launch.
  NVIDIA and AMD Windows drivers keep their own caches, which WARP doesn't have.
- The README's limits also apply: GPU wake-up, hybrid routing, and vendor scheduling are out of reach.
- Experiment: the archived timings are single, unrepeated runs, and `results/README.md` says they support no performance claims.

## Harness Instrumentation Needed

The harness already records `instance`, `adapter`, `device`, `surface`, `configure`, `first_pipeline`, `first_frame_submitted` and `present`, with the thread for each (`src/report.rs`, `src/main.rs` L567).
These additions would answer the open hypotheses above.
None of them was built or run for this page.

1. **Internal pipeline count.**
   Build with the `wgpu` `counters` feature and record `device.get_internal_counters().hal.compute_pipelines` and `shader_modules` right after `request_device`.
   Source-established: the counter exists and each HAL increments it
   ([wgpu-types/src/counters.rs L116](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-types/src/counters.rs#L116),
   [dx12/device.rs L2234](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/dx12/device.rs#L2234),
   [metal/device.rs L1875](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/metal/device.rs#L1875),
   [vulkan/device.rs L2271](https://github.com/gfx-rs/wgpu/blob/40f4a34ebaf56f9a046231f54125ad046239d3f3/wgpu-hal/src/vulkan/device.rs#L2271)).
   A value of 2 confirms indirect validation is active.
2. **Validation toggle.**
   Add a matrix variant with `WGPU_VALIDATION_INDIRECT_CALL=0` so the device phase can be compared with and without the two pipelines on each backend.
3. **DX12 compiler identity.**
   Record which compiler loaded by capturing `wgpu_hal` debug logs ("Using FXC for shader compilation" or "Using dynamic DXC"), and add `WGPU_DX12_COMPILER=fxc` and `=dxc` variants.
   Record whether `dxcompiler.dll` and `d3dcompiler_47.dll` resolve on the search path.
4. **Sub-phase scopes.**
   `wgpu-hal` wraps the expensive calls in `profiling::scope!`, for example `Fxc::D3DCompile`, `ID3D12Device2::CreatePipelineState`, `vkCreateGraphicsPipelines` and `naga::spv::write_vec`.
   Hypothesis: enabling a `profiling` backend feature in the harness exposes them as spans inside `device` and `first_pipeline`.
5. **FemtoVG-shaped first pipeline.**
   Replace or supplement the triangle with FemtoVG's `shader.wgsl` and a fixed list of `PipelineState` descriptors, such as the 17-state set from femtovg-cache-bench.
   Record `femtovg_module` (WGSL parse and validation), then per-pipeline create time and the count.
   Keep FemtoVG's `cache: None` as the baseline.
6. **First frame.**
   Keep `first_frame_submitted`, and add a completion fence through `Queue::on_submitted_work_done` so GPU completion is separate from submission.
7. **Cache state per run.**
   Record the cache condition in each record, since it changes the numbers more than the strategy does.
   - Metal: unset versus `MTL_SHADER_CACHE_SIZE=0`, labeled diagnostic.
   - Mesa: `MESA_SHADER_CACHE_DIR` set to a fresh directory versus a reused one, and `MESA_SHADER_CACHE_DISABLE=true`.
   - Vulkan through WGPU: a harness-owned `PipelineCache`, empty versus loaded from the previous run.
   - D3D12: `ShaderCacheControl` clear or disable, only on a developer-mode machine, through the DX12 HAL's raw device.
8. **Repetition.**
   Timing claims need repeated, interleaved launches with the cache condition fixed, as the prior evidence and femtovg-cache-bench protocols do.
   The current matrix is a correctness smoke run and should stay one.

## Experiment Outcomes

The harness later gained instrumentation items 1, 2 and 3 above, and the runs below used it.
They are correctness runs with the `wgpu` `counters` feature, not timing runs.
Paths are under `results/2026-09-28/`, and `results/README.md` indexes them.

### Settled

- **Experiment, "Per-Backend Consequences".** The Hypothesis that indirect validation was active on WARP holds.
  Right after `request_device`, every device on Metal (Apple M4 Max) and on WARP holds 2 compute pipelines, 2 shader modules and no render pipelines.
  That holds on the worker thread and on the main thread (`macos-metal-counters`, `windows-vm-dx12-env-baseline`).
- **Experiment, instrumentation item 2.** With `WGPU_VALIDATION_INDIRECT_CALL=0` the same devices hold no compute pipelines and no shader modules.
  They also drop from 4 buffers, 6 bind group layouts and 2 pipeline layouts to 3, 1 and 0 (`macos-metal-no-indirect-validation`, `windows-vm-dx12-no-indirect-validation`).
- **Experiment, "Shared `wgpu-core` Work".** Release builds carry `VALIDATION_INDIRECT_CALL` alone in `build.instance_flags`.
  This matches the function body, not its doc comment.
- **Experiment, "DX12: WGSL to HLSL".** The Hypothesis that stock Windows 11 uses FXC holds on the one VM tested.
  `C:\Windows\System32\D3DCompiler_47.dll` is present and no `dxcompiler.dll` is on the search path.
  With the default `Auto` and with `WGPU_DX12_COMPILER=fxc`, `wgpu-hal` logs `Using FXC for shader compilation` and only `d3dcompiler_47.dll` loads (`windows-vm-dx12-env-baseline`, `windows-vm-dx12-compiler-fxc`).
  The sentence "The archived Windows records don't log which compiler loaded" no longer applies, since records now carry `dx12_compiler`.
- **Experiment, instrumentation item 3.** Forcing `WGPU_DX12_COMPILER=dxc` without `dxcompiler.dll` makes the DX12 backend fail at instance creation (`windows-vm-dx12-compiler-dxc`).

### Not Settled

- The lavapipe counters weren't measured, because Docker Desktop quit during those runs.
  The Vulkan row of "Per-Backend Consequences" stays Source-established only.
- The Hypothesis that most of the Metal 50 to 60 ms under `MTL_SHADER_CACHE_SIZE=0` is the two validation pipelines is untested.
  It needs a timing run, and none was made.
- Instrumentation items 4 to 8 weren't built.
  So the cost of FXC compiles, FemtoVG-shaped pipelines and cache states remains unmeasured on every backend.
- Whether WARP uses the D3D12 implicit caches is still unknown.
