//! The owned preparation thread, modeled on `PendingWgpuDevice` in
//! `internal/backends/winit/wgpu_device.rs`.

use crate::report::{self, Failure};
use crate::slint_init;
use std::sync::{mpsc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub enum Job {
    /// `worker-device`: instance, surface-free adapter, and device.
    Device { backends: wgpu::Backends },
    /// `worker-instance`: the instance only.
    Instance { backends: wgpu::Backends },
    /// `worker-after-select`: the device for an adapter the main thread chose.
    DeviceFor { adapter: wgpu::Adapter },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatePoint {
    BeforeAdapter,
    BeforeDevice,
    AfterDevice,
}

impl GatePoint {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "before-adapter" => Self::BeforeAdapter,
            "before-device" => Self::BeforeDevice,
            "after-device" => Self::AfterDevice,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::BeforeAdapter => "before-adapter",
            Self::BeforeDevice => "before-device",
            Self::AfterDevice => "after-device",
        }
    }
}

/// A point where the worker reports that it arrived, then waits for release.
pub struct Gate {
    pub point: GatePoint,
    pub reached: mpsc::Sender<GatePoint>,
    pub release: mpsc::Receiver<()>,
}

impl Gate {
    fn pass(&self, here: GatePoint) {
        if self.point == here {
            let _ = self.reached.send(here);
            // Released by a message or by the main thread dropping its sender.
            let _ = self.release.recv();
        }
    }
}

/// How far the current worker got, in order.
/// The main thread waits on it to place the probe and the early exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Idle,
    Spawned,
    InInstance,
    InstanceDone,
    InAdapter,
    AdapterDone,
    InDevice,
    DeviceDone,
    Finished,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Spawned => "spawned",
            Self::InInstance => "in-instance",
            Self::InstanceDone => "instance-done",
            Self::InAdapter => "in-adapter",
            Self::AdapterDone => "adapter-done",
            Self::InDevice => "in-device",
            Self::DeviceDone => "device-done",
            Self::Finished => "finished",
        }
    }
}

static STAGE: Mutex<Stage> = Mutex::new(Stage::Idle);
static STAGE_CHANGED: Condvar = Condvar::new();

fn advance(stage: Stage) {
    *STAGE.lock().unwrap_or_else(|e| e.into_inner()) = stage;
    STAGE_CHANGED.notify_all();
}

pub fn stage() -> Stage {
    *STAGE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Waits until the worker reaches `target` or finishes, and returns the stage it saw.
pub fn wait_for_stage(target: Stage, timeout: Duration) -> Stage {
    let guard = STAGE.lock().unwrap_or_else(|e| e.into_inner());
    let (guard, _) = STAGE_CHANGED
        .wait_timeout_while(guard, timeout, |s| *s < target && *s != Stage::Finished)
        .unwrap_or_else(|e| e.into_inner());
    *guard
}

/// Runs `f` between the two stages.
fn staged<R>(inside: Stage, done: Stage, f: impl FnOnce() -> R) -> R {
    advance(inside);
    let out = f();
    advance(done);
    out
}

#[derive(Default)]
pub struct Output {
    pub instance: Option<wgpu::Instance>,
    pub adapter: Option<wgpu::Adapter>,
    pub device: Option<(wgpu::Device, wgpu::Queue)>,
    pub error: Option<(&'static str, String)>,
}

pub struct Pending {
    handle: Option<JoinHandle<Output>>,
}

const CONTEXT: &str = "worker";

fn run(job: Job, failure: Failure, gate: Option<Gate>) -> Output {
    let pass = |p| {
        if let Some(g) = &gate {
            g.pass(p)
        }
    };
    let mut out = Output::default();
    match job {
        Job::Instance { backends } => {
            out.instance = Some(staged(Stage::InInstance, Stage::InstanceDone, || {
                report::phase("instance", CONTEXT, || {
                    slint_init::create_instance(backends)
                })
            }));
            // This job has one stopping point, so every requested gate maps to it.
            if let Some(g) = &gate {
                g.pass(g.point);
            }
        }
        Job::Device { backends } => {
            let instance = staged(Stage::InInstance, Stage::InstanceDone, || {
                report::phase("instance", CONTEXT, || {
                    slint_init::create_instance(backends)
                })
            });
            pass(GatePoint::BeforeAdapter);
            let adapter = staged(Stage::InAdapter, Stage::AdapterDone, || {
                report::phase("adapter", CONTEXT, || {
                    slint_init::select_adapter(&instance, None)
                })
            });
            out.instance = Some(instance);
            let adapter = match adapter {
                Ok(a) => a,
                Err(e) => {
                    out.error = Some(("adapter", e));
                    return out;
                }
            };
            pass(GatePoint::BeforeDevice);
            match staged(Stage::InDevice, Stage::DeviceDone, || {
                report::phase("device", CONTEXT, || {
                    slint_init::request_device(&adapter, failure)
                })
            }) {
                Ok(d) => {
                    crate::diag::record_new_device(&d.0, CONTEXT);
                    out.device = Some(d)
                }
                Err(e) => out.error = Some(("device", e)),
            }
            out.adapter = Some(adapter);
            pass(GatePoint::AfterDevice);
        }
        Job::DeviceFor { adapter } => {
            // The adapter already exists, so `before-adapter` stops where `before-device` does.
            pass(GatePoint::BeforeAdapter);
            pass(GatePoint::BeforeDevice);
            match staged(Stage::InDevice, Stage::DeviceDone, || {
                report::phase("device", CONTEXT, || {
                    slint_init::request_device(&adapter, failure)
                })
            }) {
                Ok(d) => {
                    crate::diag::record_new_device(&d.0, CONTEXT);
                    out.device = Some(d)
                }
                Err(e) => out.error = Some(("device", e)),
            }
            out.adapter = Some(adapter);
            pass(GatePoint::AfterDevice);
        }
    }
    out
}

/// Runs `f` inside an autorelease pool on Apple platforms, as Slint's worker and `Drop` do.
pub fn autoreleasepool<R>(f: impl FnOnce() -> R) -> R {
    #[cfg(target_vendor = "apple")]
    {
        objc2::rc::autoreleasepool(|_| f())
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        f()
    }
}

impl Pending {
    pub fn start(job: Job, failure: Failure, gate: Option<Gate>) -> Self {
        report::event("worker_spawn", serde_json::json!({}));
        advance(Stage::Spawned);
        let handle = std::thread::Builder::new()
            .name("slint-wgpu-device".into())
            .spawn(move || {
                let out = autoreleasepool(|| run(job, failure, gate));
                advance(Stage::Finished);
                out
            });
        match handle {
            Ok(h) => Self { handle: Some(h) },
            Err(e) => {
                report::error("spawn", CONTEXT, format!("thread spawn failed: {e}"));
                Self { handle: None }
            }
        }
    }

    /// Joins the worker. A worker panic becomes an error, not a propagated panic.
    pub fn take(&mut self) -> Option<Output> {
        let handle = self.handle.take()?;
        let joined = report::phase("worker_join", "main", || handle.join());
        Some(joined.unwrap_or_else(|panic| {
            let msg = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "non-string payload".into());
            Output {
                error: Some(("worker", format!("worker panic: {msg}"))),
                ..Default::default()
            }
        }))
    }

    /// Abandons the worker: waits up to `timeout` for it to finish, then drops its result
    /// inside an autorelease pool. Returns whether it finished in time.
    pub fn abandon(mut self, timeout: Duration) -> (bool, Option<bool>) {
        let Some(handle) = self.handle.take() else {
            return (true, None);
        };
        let deadline = Instant::now() + timeout;
        while !handle.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        if !handle.is_finished() {
            // Leak the thread; the watchdog reports the hang if the process can't exit.
            return (false, None);
        }
        let ok = autoreleasepool(|| {
            report::phase("abandon_join_and_drop", "main", || match handle.join() {
                Ok(out) => {
                    let ok = out.error.is_none();
                    drop(out);
                    Some(ok)
                }
                Err(_) => None,
            })
        });
        (true, ok)
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            autoreleasepool(|| {
                let _ = handle.join();
            });
        }
    }
}
