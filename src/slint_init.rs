//! Copies of Slint's WGPU 30 initialization settings.
//!
//! Each function names the Slint function it mirrors.
//! Paths are relative to the Slint repository.
//! `docs/slint-correspondence.md` lists every correspondence and its known differences.

use crate::report::Failure;

/// Mirrors `default_backends_to_avoid` in `internal/core/graphics/wgpu_30.rs`.
pub fn default_backends_to_avoid() -> wgpu::Backends {
    let mut avoid = wgpu::Backends::GL;
    #[cfg(not(target_vendor = "apple"))]
    avoid.insert(wgpu::Backends::METAL);
    #[cfg(not(target_family = "windows"))]
    avoid.insert(wgpu::Backends::DX12);
    avoid
}

/// Mirrors `mask_backends` in `internal/core/graphics/wgpu_30.rs`.
pub fn mask_backends(requested: wgpu::Backends, avoid: wgpu::Backends) -> wgpu::Backends {
    let masked = requested & !avoid;
    if masked.is_empty() {
        requested
    } else {
        masked
    }
}

/// The backend mask Slint requests when no graphics API is configured.
/// Mirrors the `None` arm of `async_init_instance_adapter_device_queue_surface`.
/// `failure` substitutes a mask the platform can't serve.
pub fn requested_backends(failure: Failure) -> wgpu::Backends {
    match failure {
        Failure::EmptyMask => wgpu::Backends::empty(),
        Failure::UnsupportedMask => unsupported_backend(),
        _ => wgpu::Backends::from_env().unwrap_or_default(),
    }
}

/// A backend with no compiled HAL on this platform.
fn unsupported_backend() -> wgpu::Backends {
    if cfg!(target_vendor = "apple") {
        wgpu::Backends::DX12
    } else {
        wgpu::Backends::METAL
    }
}

/// Mirrors the empty-mask check in the `None` arm of
/// `async_init_instance_adapter_device_queue_surface`.
pub fn check_backends(backends: wgpu::Backends) -> Result<(), String> {
    if backends.is_empty() {
        Err("The requested graphics API (None) is not supported under wgpu on this platform".into())
    } else {
        Ok(())
    }
}

/// Mirrors `default_instance_descriptor` in `internal/core/graphics/wgpu_30.rs`.
pub fn default_instance_descriptor(backends: wgpu::Backends) -> wgpu::InstanceDescriptor {
    wgpu::InstanceDescriptor {
        backends,
        flags: wgpu::InstanceFlags::from_build_config().with_env(),
        backend_options: wgpu::BackendOptions::from_env_or_default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        display: None,
    }
}

/// Mirrors the `new_instance_with_webgpu_detection` call in Slint.
pub fn create_instance(backends: wgpu::Backends) -> wgpu::Instance {
    block(wgpu::util::new_instance_with_webgpu_detection(
        default_instance_descriptor(backends),
    ))
}

/// Mirrors `init_default_adapter_device_queue` (adapter half), including its error text.
pub fn select_adapter(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<wgpu::Adapter, String> {
    block(wgpu::util::initialize_adapter_from_env_or_default(
        instance, surface,
    ))
    .map_err(|e| format!("Failed to find an appropriate adapter: {e}"))
}

/// Mirrors `default_device_descriptor` in `internal/core/graphics/wgpu_30.rs`.
/// `Failure::Limits` raises two limits past anything an adapter offers.
pub fn device_descriptor(
    adapter: &wgpu::Adapter,
    failure: Failure,
) -> wgpu::DeviceDescriptor<'static> {
    let mut required_limits = adapter.limits();
    if failure == Failure::Limits {
        required_limits.max_texture_dimension_2d = u32::MAX;
        required_limits.max_bind_groups = u32::MAX;
    }
    wgpu::DeviceDescriptor {
        label: None,
        required_features: adapter.features() - wgpu::Features::all_experimental_mask(),
        required_limits,
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::default(),
    }
}

/// Mirrors `init_default_adapter_device_queue` (device half), including its error text.
pub fn request_device(
    adapter: &wgpu::Adapter,
    failure: Failure,
) -> Result<(wgpu::Device, wgpu::Queue), String> {
    block(adapter.request_device(&device_descriptor(adapter, failure)))
        .map_err(|e| format!("Failed to create device: {e}"))
}

/// Mirrors the preconditions of `PendingWgpuDevice::start` in
/// `internal/backends/winit/wgpu_device.rs` for the default renderer and no configured API.
pub fn slint_would_prepare(effective: wgpu::Backends) -> bool {
    would_prepare_for(effective, wgpu::Backends::METAL)
}

/// The same preconditions with `gate` in place of `METAL`.
/// `DX12` is the Windows extension in `docs/research/dx12-worker-safety.md`, section 6.
pub fn would_prepare_for(effective: wgpu::Backends, gate: wgpu::Backends) -> bool {
    std::env::var_os("WGPU_ADAPTER_NAME").is_none()
        && effective & wgpu::Instance::enabled_backend_features() == gate
}

/// The surface configuration Slint's femtovg WGPU backend applies to a new opaque window.
/// Mirrors `configure_surface_from_init_result` in `internal/renderers/femtovg/wgpu.rs`.
pub fn surface_config(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    width: u32,
    height: u32,
) -> Result<(wgpu::SurfaceConfiguration, wgpu::SurfaceCapabilities), String> {
    let mut config = surface
        .get_default_config(adapter, width, height)
        .ok_or_else(|| {
            "surface is not supported by the adapter (get_default_config returned None)".to_string()
        })?;
    let caps = surface.get_capabilities(adapter);
    let format = caps
        .formats
        .iter()
        .find(|f| {
            matches!(
                f,
                wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
            )
        })
        .copied()
        .or_else(|| caps.formats.first().copied())
        .ok_or_else(|| "surface advertises no formats".to_string())?;
    config.format = format;
    // `composite_alpha_mode(false, ..)` in the same file.
    if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
    }
    Ok((config, caps))
}

/// Mirrors `poll_once` in `internal/core/graphics/wgpu_30.rs`, falling back to
/// `pollster` when the future is pending so the harness records instead of aborting.
pub fn block<F: std::future::Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut ctx = std::task::Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut ctx) {
        std::task::Poll::Ready(v) => v,
        std::task::Poll::Pending => {
            crate::report::note_pending_poll();
            pollster::block_on(future)
        }
    }
}
