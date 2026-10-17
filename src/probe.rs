//! A copy of Slint's main-thread GPU probe, placed at a chosen point of worker progress.

use crate::report;
use crate::slint_init;
use crate::worker::{self, Stage};
use serde_json::json;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Point {
    None,
    /// Right after the worker starts, where Slint's `new_suspended` runs it.
    Concurrent,
    AfterInstance,
    AfterAdapter,
    AfterDevice,
    /// In `resumed`, once the first window's device exists.
    /// Ordinary Slint reaches the same order when a later window creates its own instance.
    AfterFirstDevice,
}

impl Point {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "none" => Self::None,
            "concurrent" => Self::Concurrent,
            "after-instance" => Self::AfterInstance,
            "after-adapter" => Self::AfterAdapter,
            "after-device" => Self::AfterDevice,
            "after-first-device" => Self::AfterFirstDevice,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Concurrent => "concurrent",
            Self::AfterInstance => "after-instance",
            Self::AfterAdapter => "after-adapter",
            Self::AfterDevice => "after-device",
            Self::AfterFirstDevice => "after-first-device",
        }
    }
    /// The worker stage to wait for before probing, for points before the event loop.
    pub fn worker_stage(self) -> Option<Stage> {
        match self {
            Self::Concurrent => Some(Stage::Spawned),
            Self::AfterInstance => Some(Stage::InstanceDone),
            Self::AfterAdapter => Some(Stage::AdapterDone),
            Self::AfterDevice => Some(Stage::DeviceDone),
            Self::None | Self::AfterFirstDevice => None,
        }
    }
}

/// What the harness does when the probe rejects WGPU, as it does when every adapter is a CPU adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnReject {
    /// Record the rejection and render anyway, as if a GPU adapter had passed.
    Continue,
    /// Skip WGPU and keep the prepared result until process exit, as the Slint branch does.
    Keep,
    /// Skip WGPU and join and drop the prepared result at once.
    Drop,
}

impl OnReject {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "continue" => Self::Continue,
            "keep" => Self::Keep,
            "drop" => Self::Drop,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Continue => "continue",
            Self::Keep => "keep",
            Self::Drop => "drop",
        }
    }
}

/// Mirrors the `None` arm of `any_wgpu30_adapters_with_gpu` in `internal/core/graphics/wgpu_30.rs`.
/// Returns whether Slint would go on to use WGPU.
///
/// `force_validation` departs from Slint: it adds `VALIDATION` to the probe's instance only,
/// so the probe turns the D3D12 debug layer on after a device made without it exists.
pub fn any_adapters_with_gpu(
    backends: wgpu::Backends,
    force_validation: bool,
) -> (bool, Vec<serde_json::Value>) {
    if std::env::var("SLINT_WGPU_CPU").is_ok() {
        return (true, Vec::new());
    }
    let mut desc = slint_init::default_instance_descriptor(backends);
    if force_validation {
        desc.flags |= wgpu::InstanceFlags::VALIDATION;
    }
    let instance = wgpu::Instance::new(desc);
    if backends.is_empty() {
        return (false, Vec::new());
    }
    let adapters = slint_init::block(instance.enumerate_adapters(backends));
    let seen = adapters
        .iter()
        .map(|a| {
            let i = a.get_info();
            json!({"name": i.name, "backend": format!("{:?}", i.backend),
                   "device_type": format!("{:?}", i.device_type)})
        })
        .collect();
    let any_gpu = adapters
        .iter()
        .any(|a| a.get_info().device_type != wgpu::DeviceType::Cpu);
    (any_gpu, seen)
}

/// Runs the probe on the calling thread and records it. Returns whether the probe rejected WGPU.
pub fn run(
    point: Point,
    backends: wgpu::Backends,
    wait_for_worker: bool,
    force_validation: bool,
) -> bool {
    let waited = match point.worker_stage() {
        Some(target) if wait_for_worker => {
            Some(worker::wait_for_stage(target, Duration::from_secs(30)))
        }
        _ => None,
    };
    let stage_at_probe = worker::stage();
    let start = report::now_ms();
    let (any_gpu, adapters) = report::phase("probe", "main", || {
        any_adapters_with_gpu(backends, force_validation)
    });
    report::set(
        "probe",
        json!({
            "point": point.name(),
            "forced_validation": force_validation,
            "waited_for_stage": waited.map(|s| s.name()),
            "worker_stage_at_probe": stage_at_probe.name(),
            "worker_stage_after_probe": worker::stage().name(),
            "slint_wgpu_cpu": std::env::var_os("SLINT_WGPU_CPU").is_some(),
            "instance_created": std::env::var_os("SLINT_WGPU_CPU").is_none(),
            "adapters": adapters,
            "any_gpu": any_gpu,
            "rejected": !any_gpu,
            "start_ms": report::round3(start),
        }),
    );
    !any_gpu
}
