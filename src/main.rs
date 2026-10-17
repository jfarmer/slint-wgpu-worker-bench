//! Behavioral harness for preparing a WGPU device on a worker thread before the Winit
//! event loop starts, compared with Slint's ordinary surface-first initialization.
//!
//! Run `slint-wgpu-worker-bench --help` for options. Each run writes one JSON record.

mod diag;
mod logcap;
mod probe;
mod render;
mod report;
mod slint_init;
mod worker;

use report::Failure;
use serde_json::{json, Value};
use std::sync::{mpsc, Arc};
use std::time::Duration;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use worker::{Gate, GatePoint, Job, Pending, Stage};

const EXIT_USAGE: i32 = 2;
const EXIT_WATCHDOG: i32 = 3;
const EXIT_PANIC: i32 = 5;
/// The `early-exit` scenario's intended exit code.
const EXIT_EARLY: i32 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Strategy {
    Ordinary,
    WorkerDevice,
    WorkerInstance,
    WorkerAfterSelect,
}

impl Strategy {
    const ALL: [Strategy; 4] = [
        Self::Ordinary,
        Self::WorkerDevice,
        Self::WorkerInstance,
        Self::WorkerAfterSelect,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Ordinary => "ordinary",
            Self::WorkerDevice => "worker-device",
            Self::WorkerInstance => "worker-instance",
            Self::WorkerAfterSelect => "worker-after-select",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.name() == s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    Single,
    MultiWindow,
    Abandon,
    Failure,
    Repeated,
    JoinBlock,
    EarlyExit,
}

impl Scenario {
    const ALL: [Scenario; 7] = [
        Self::Single,
        Self::MultiWindow,
        Self::Abandon,
        Self::Failure,
        Self::Repeated,
        Self::JoinBlock,
        Self::EarlyExit,
    ];
    fn name(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::MultiWindow => "multi-window",
            Self::Abandon => "abandon",
            Self::Failure => "failure",
            Self::Repeated => "repeated",
            Self::JoinBlock => "join-block",
            Self::EarlyExit => "early-exit",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.name() == s)
    }
}

/// Where `join-block` blocks the main thread in the join.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JoinAt {
    BeforeEventLoop,
    AfterWindow,
}

impl JoinAt {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "before-event-loop" => Self::BeforeEventLoop,
            "after-window" => Self::AfterWindow,
            _ => return None,
        })
    }
    fn name(self) -> &'static str {
        match self {
            Self::BeforeEventLoop => "before-event-loop",
            Self::AfterWindow => "after-window",
        }
    }
}

fn parse_exit_at(s: &str) -> Option<Stage> {
    Some(match s {
        "in-instance" => Stage::InInstance,
        "in-adapter" => Stage::InAdapter,
        "in-device" => Stage::InDevice,
        _ => return None,
    })
}

struct Args {
    strategy: Strategy,
    scenario: Scenario,
    out: Option<String>,
    windows: usize,
    repeat: usize,
    abandon_at: GatePoint,
    failure: Failure,
    timeout: Duration,
    probe: probe::Point,
    on_reject: probe::OnReject,
    join_at: JoinAt,
    release_after: Duration,
    exit_at: Stage,
    linger: Duration,
    probe_validation: bool,
}

const USAGE: &str = "\
usage: slint-wgpu-worker-bench --strategy S --scenario C [options]

  --strategy     ordinary | worker-device | worker-instance | worker-after-select
  --scenario     single | multi-window | abandon | failure | repeated
  --out PATH     write the JSON record to PATH instead of stdout
  --windows N    windows for multi-window (default 2)
  --repeat K     iterations for repeated (default 3)
  --abandon-at P before-adapter | before-device | after-device (default before-device)
  --failure F    limits | empty-mask | unsupported-mask (failure scenario, default limits)
  --timeout SECS watchdog timeout (default 60); exit code 3 when it fires
  --probe P      none | concurrent | after-instance | after-adapter | after-device |
                 after-first-device: run Slint's main-thread GPU probe there (default none)
  --on-probe-reject R
                 continue | keep | drop: after a probe rejection, render anyway, or skip
                 WGPU and keep the prepared result until exit, or drop it at once
                 (default continue)
  --join-at J    before-event-loop | after-window: where join-block blocks in the join
                 (default before-event-loop); the gate is --abandon-at
  --release-after-ms MS
                 join-block releases the gated worker after MS (default 300)
  --exit-at E    in-instance | in-adapter | in-device: early-exit exits the process when
                 the worker enters this call (default in-device)
  --linger-ms MS how long a run that skipped WGPU keeps its window (default 300)
  --probe-validation on
                 add VALIDATION to the probe's instance only, unlike Slint, as a control
                 that turns the D3D12 debug layer on after a device exists (default off)
";

fn usage_exit(msg: &str) -> ! {
    eprintln!("{msg}\n\n{USAGE}");
    std::process::exit(EXIT_USAGE)
}

fn bad<T>(flag: &str, value: &str) -> T {
    usage_exit(&format!("bad value for {flag}: {value}"))
}

fn parse_args() -> Args {
    let mut args = Args {
        strategy: Strategy::Ordinary,
        scenario: Scenario::Single,
        out: None,
        windows: 2,
        repeat: 3,
        abandon_at: GatePoint::BeforeDevice,
        failure: Failure::None,
        timeout: Duration::from_secs(60),
        probe: probe::Point::None,
        on_reject: probe::OnReject::Continue,
        join_at: JoinAt::BeforeEventLoop,
        release_after: Duration::from_millis(300),
        exit_at: Stage::InDevice,
        linger: Duration::from_millis(300),
        probe_validation: false,
    };
    let mut failure_set = false;
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        if flag == "--help" || flag == "-h" {
            print!("{USAGE}");
            std::process::exit(0);
        }
        let value = it
            .next()
            .unwrap_or_else(|| usage_exit(&format!("{flag} needs a value")));
        match flag.as_str() {
            "--strategy" => {
                args.strategy = Strategy::parse(&value).unwrap_or_else(|| bad(&flag, &value))
            }
            "--scenario" => {
                args.scenario = Scenario::parse(&value).unwrap_or_else(|| bad(&flag, &value))
            }
            "--out" => args.out = Some(value.clone()),
            "--windows" => {
                args.windows = value
                    .parse()
                    .ok()
                    .filter(|n| *n > 0)
                    .unwrap_or_else(|| bad(&flag, &value))
            }
            "--repeat" => {
                args.repeat = value
                    .parse()
                    .ok()
                    .filter(|n| *n > 0)
                    .unwrap_or_else(|| bad(&flag, &value))
            }
            "--abandon-at" => {
                args.abandon_at = GatePoint::parse(&value).unwrap_or_else(|| bad(&flag, &value))
            }
            "--failure" => {
                args.failure = Failure::parse(&value).unwrap_or_else(|| bad(&flag, &value));
                failure_set = true;
            }
            "--timeout" => {
                args.timeout =
                    Duration::from_secs(value.parse().unwrap_or_else(|_| bad(&flag, &value)))
            }
            "--probe" => {
                args.probe = probe::Point::parse(&value).unwrap_or_else(|| bad(&flag, &value))
            }
            "--on-probe-reject" => {
                args.on_reject =
                    probe::OnReject::parse(&value).unwrap_or_else(|| bad(&flag, &value))
            }
            "--join-at" => {
                args.join_at = JoinAt::parse(&value).unwrap_or_else(|| bad(&flag, &value))
            }
            "--release-after-ms" => {
                args.release_after =
                    Duration::from_millis(value.parse().unwrap_or_else(|_| bad(&flag, &value)))
            }
            "--exit-at" => {
                args.exit_at = parse_exit_at(&value).unwrap_or_else(|| bad(&flag, &value))
            }
            "--probe-validation" => {
                args.probe_validation = match value.as_str() {
                    "on" => true,
                    "off" => false,
                    _ => bad(&flag, &value),
                }
            }
            "--linger-ms" => {
                args.linger =
                    Duration::from_millis(value.parse().unwrap_or_else(|_| bad(&flag, &value)))
            }
            _ => usage_exit(&format!("unknown flag {flag}")),
        }
    }
    if args.scenario == Scenario::Failure && !failure_set {
        args.failure = Failure::Limits;
    }
    if args.scenario != Scenario::Failure {
        args.failure = Failure::None;
    }
    args
}

const ENV_KEYS: &[&str] = &[
    "WGPU_BACKEND",
    "WGPU_ADAPTER_NAME",
    "WGPU_POWER_PREF",
    "WGPU_DEBUG",
    "WGPU_VALIDATION",
    "WGPU_GPU_BASED_VALIDATION",
    "WGPU_DX12_COMPILER",
    "WGPU_DX12_PRESENTATION_SYSTEM",
    "WGPU_VALIDATION_INDIRECT_CALL",
    "SLINT_WGPU_CPU",
    "VK_LOADER_DEVICE_SELECT",
    "VK_LOADER_DISABLE_SELECT",
    "WGPU_GLES_MINOR_VERSION",
    "WGPU_FORCE_FALLBACK_ADAPTER",
    "VK_ICD_FILENAMES",
    "VK_DRIVER_FILES",
    "LIBGL_ALWAYS_SOFTWARE",
    "GALLIUM_DRIVER",
    "MESA_LOADER_DRIVER_OVERRIDE",
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "WINIT_UNIX_BACKEND",
    "SESSIONNAME",
    "BENCH_TEST_HANG",
];

fn adapter_info(adapter: &wgpu::Adapter) -> Value {
    let i = adapter.get_info();
    json!({
        "name": i.name, "vendor": i.vendor, "device": i.device,
        "device_type": format!("{:?}", i.device_type), "backend": format!("{:?}", i.backend),
        "driver": i.driver, "driver_info": i.driver_info,
    })
}

fn debug_list<T: std::fmt::Debug>(items: &[T]) -> Vec<String> {
    items.iter().map(|v| format!("{v:?}")).collect()
}

struct Gpu {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// A window plus its surface on the shared instance.
struct Target {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
}

struct PresentJob {
    ctx: String,
    target: Target,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    format: wgpu::TextureFormat,
    attempts: u32,
    done: bool,
}

/// How long `about_to_wait` keeps retrying a present while the window is not yet visible.
const PRESENT_DEADLINE: Duration = Duration::from_secs(5);

struct App {
    live: Vec<PresentJob>,
    present_deadline: Option<std::time::Instant>,
    args: Args,
    backends: wgpu::Backends,
    pending: Option<Pending>,
    gate_release: Option<mpsc::Sender<()>>,
    gate_reached: Option<mpsc::Receiver<GatePoint>>,
    done: bool,
    listed_adapters: bool,
    /// A worker result joined before `acquire`, by `join-block`.
    prejoined: Option<worker::Output>,
    /// Set when the probe rejected WGPU and `--on-probe-reject` skips it.
    skip_gpu: bool,
    probe_done: bool,
    linger_until: Option<std::time::Instant>,
    stand_in: Option<Arc<Window>>,
}

fn gate_for(
    args: &Args,
) -> (
    Option<Gate>,
    Option<mpsc::Sender<()>>,
    Option<mpsc::Receiver<GatePoint>>,
) {
    if !matches!(args.scenario, Scenario::Abandon | Scenario::JoinBlock) {
        return (None, None, None);
    }
    let (reached_tx, reached_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    (
        Some(Gate {
            point: args.abandon_at,
            reached: reached_tx,
            release: release_rx,
        }),
        Some(release_tx),
        Some(reached_rx),
    )
}

/// The pre-event-loop preparation for strategies that start before Winit.
fn start_early(
    strategy: Strategy,
    backends: wgpu::Backends,
    failure: Failure,
    gate: Option<Gate>,
) -> Option<Pending> {
    match strategy {
        Strategy::WorkerDevice => Some(Pending::start(Job::Device { backends }, failure, gate)),
        Strategy::WorkerInstance => Some(Pending::start(Job::Instance { backends }, failure, gate)),
        Strategy::Ordinary | Strategy::WorkerAfterSelect => None,
    }
}

impl App {
    fn create_window(&self, event_loop: &ActiveEventLoop, ctx: &str) -> Option<Arc<Window>> {
        let attrs = Window::default_attributes()
            .with_title(format!("slint-wgpu-worker-bench {ctx}"))
            .with_inner_size(winit::dpi::LogicalSize::new(320.0, 240.0));
        match report::phase("window", ctx, || event_loop.create_window(attrs)) {
            Ok(w) => Some(Arc::new(w)),
            Err(e) => {
                report::error("window", ctx, format!("window creation failed: {e}"));
                None
            }
        }
    }

    fn create_surface(
        &self,
        instance: &wgpu::Instance,
        window: &Arc<Window>,
        ctx: &str,
    ) -> Option<wgpu::Surface<'static>> {
        match report::phase("surface", ctx, || instance.create_surface(window.clone())) {
            Ok(s) => Some(s),
            Err(e) => {
                report::error(
                    "surface",
                    ctx,
                    format!("Error creating wgpu window surface: {e}"),
                );
                None
            }
        }
    }

    fn device_on_main(
        &self,
        adapter: &wgpu::Adapter,
        ctx: &str,
    ) -> Option<(wgpu::Device, wgpu::Queue)> {
        match report::phase("device", ctx, || {
            slint_init::request_device(adapter, self.args.failure)
        }) {
            Ok(d) => {
                diag::record_new_device(&d.0, ctx);
                Some(d)
            }
            Err(e) => {
                report::error("device", ctx, e);
                None
            }
        }
    }

    fn adapter_on_main(
        &self,
        instance: &wgpu::Instance,
        surface: &wgpu::Surface<'_>,
        ctx: &str,
    ) -> Option<wgpu::Adapter> {
        match report::phase("adapter", ctx, || {
            slint_init::select_adapter(instance, Some(surface))
        }) {
            Ok(a) => Some(a),
            Err(e) => {
                report::error("adapter", ctx, e);
                None
            }
        }
    }

    fn take_worker(&mut self, ctx: &str) -> Option<worker::Output> {
        let out = self
            .prejoined
            .take()
            .or_else(|| self.pending.as_mut().and_then(|p| p.take()));
        self.pending = None;
        match out {
            Some(out) => {
                if let Some((phase, msg)) = &out.error {
                    report::error(phase, ctx, msg.clone());
                }
                Some(out)
            }
            None => {
                report::error("worker", ctx, "no prepared result");
                None
            }
        }
    }

    /// Produces the device the first window uses, following the chosen strategy.
    fn acquire(
        &mut self,
        window: &Arc<Window>,
        ctx: &str,
    ) -> Option<(Gpu, wgpu::Surface<'static>)> {
        let failure = self.args.failure;
        match self.args.strategy {
            Strategy::Ordinary => {
                let instance = report::phase("instance", ctx, || {
                    slint_init::create_instance(self.backends)
                });
                let surface = self.create_surface(&instance, window, ctx)?;
                let adapter = self.adapter_on_main(&instance, &surface, ctx)?;
                let (device, queue) = self.device_on_main(&adapter, ctx)?;
                Some((
                    Gpu {
                        instance,
                        adapter,
                        device,
                        queue,
                    },
                    surface,
                ))
            }
            Strategy::WorkerDevice => {
                let out = self.take_worker(ctx)?;
                let instance = out.instance?;
                let adapter = out.adapter?;
                let (device, queue) = out.device?;
                let surface = self.create_surface(&instance, window, ctx)?;
                let supported = report::phase("is_surface_supported", ctx, || {
                    adapter.is_surface_supported(&surface)
                });
                report::event(
                    "prepared_adapter_surface_check",
                    json!({"context": ctx, "supported": supported}),
                );
                if supported {
                    return Some((
                        Gpu {
                            instance,
                            adapter,
                            device,
                            queue,
                        },
                        surface,
                    ));
                }
                report::error(
                    "is_surface_supported",
                    ctx,
                    format!(
                        "surface-unsupported: prepared adapter {:?} cannot present to this surface",
                        adapter.get_info().name
                    ),
                );
                report::event("fallback_to_surface_selection", json!({"context": ctx}));
                drop((device, queue));
                let adapter = self.adapter_on_main(&instance, &surface, ctx)?;
                let (device, queue) = self.device_on_main(&adapter, ctx)?;
                Some((
                    Gpu {
                        instance,
                        adapter,
                        device,
                        queue,
                    },
                    surface,
                ))
            }
            Strategy::WorkerInstance => {
                let out = self.take_worker(ctx)?;
                let instance = out.instance?;
                let surface = self.create_surface(&instance, window, ctx)?;
                let adapter = self.adapter_on_main(&instance, &surface, ctx)?;
                let (device, queue) = self.device_on_main(&adapter, ctx)?;
                Some((
                    Gpu {
                        instance,
                        adapter,
                        device,
                        queue,
                    },
                    surface,
                ))
            }
            Strategy::WorkerAfterSelect => {
                let instance = report::phase("instance", ctx, || {
                    slint_init::create_instance(self.backends)
                });
                let surface = self.create_surface(&instance, window, ctx)?;
                let adapter = self.adapter_on_main(&instance, &surface, ctx)?;
                self.pending = Some(Pending::start(
                    Job::DeviceFor {
                        adapter: adapter.clone(),
                    },
                    failure,
                    None,
                ));
                let out = self.take_worker(ctx)?;
                let (device, queue) = out.device?;
                Some((
                    Gpu {
                        instance,
                        adapter,
                        device,
                        queue,
                    },
                    surface,
                ))
            }
        }
    }

    fn list_adapters(&mut self, gpu: &Gpu, surface: &wgpu::Surface<'_>) {
        if self.listed_adapters {
            return;
        }
        self.listed_adapters = true;
        // A fresh instance with the same mask gives the surface-free enumeration order.
        let fresh = slint_init::create_instance(self.backends);
        let order: Vec<Value> = slint_init::block(fresh.enumerate_adapters(wgpu::Backends::all()))
            .iter()
            .map(adapter_info)
            .collect();
        let on_instance: Vec<Value> =
            slint_init::block(gpu.instance.enumerate_adapters(wgpu::Backends::all()))
                .iter()
                .map(|a| {
                    let mut v = adapter_info(a);
                    v["is_surface_supported"] = json!(a.is_surface_supported(surface));
                    v
                })
                .collect();
        report::set("adapters_surface_free", json!(order));
        report::set("adapters_with_surface_support", json!(on_instance));
    }

    /// Configures, renders offscreen, and presents one frame for one window.
    fn window_work(
        &self,
        gpu: &Gpu,
        target: &Target,
        ctx: &str,
        mode: &str,
    ) -> Option<(wgpu::RenderPipeline, wgpu::TextureFormat)> {
        let mut rec = json!({"context": ctx, "mode": mode});
        rec["is_surface_supported"] = json!(gpu.adapter.is_surface_supported(&target.surface));
        let size = target.window.inner_size();
        rec["inner_size"] = json!([size.width, size.height]);
        let config = report::phase("configure", ctx, || {
            let (config, caps) =
                slint_init::surface_config(&target.surface, &gpu.adapter, size.width, size.height)?;
            let mut configured = false;
            if size.width > 0 && size.height > 0 {
                let scope = gpu.device.push_error_scope(wgpu::ErrorFilter::Validation);
                target.surface.configure(&gpu.device, &config);
                match slint_init::block(scope.pop()) {
                    None => configured = true,
                    Some(e) => report::error(
                        "configure",
                        ctx,
                        format!("surface configure validation: {e}"),
                    ),
                }
            }
            Ok::<_, String>((config, caps, configured))
        });
        let (config, caps, configured) = match config {
            Ok(v) => v,
            Err(e) => {
                report::error("configure", ctx, e);
                report::push_window(rec);
                return None;
            }
        };
        rec["capabilities"] = json!({
            "formats": debug_list(&caps.formats),
            "present_modes": debug_list(&caps.present_modes),
            "alpha_modes": debug_list(&caps.alpha_modes),
            "usages": format!("{:?}", caps.usages),
        });
        rec["configured"] = json!({
            "format": format!("{:?}", config.format),
            "present_mode": format!("{:?}", config.present_mode),
            "alpha_mode": format!("{:?}", config.alpha_mode),
            "width": config.width, "height": config.height,
            "desired_maximum_frame_latency": config.desired_maximum_frame_latency,
            "accepted": configured,
        });
        let pipeline = report::phase("first_pipeline", ctx, || {
            render::pipeline(&gpu.device, config.format)
        });
        match report::phase("first_frame_submitted", ctx, || {
            render::offscreen(&gpu.device, &gpu.queue, &pipeline, config.format)
        }) {
            Ok(v) => rec["offscreen"] = v,
            Err(e) => report::error("offscreen", ctx, e),
        }
        if !configured {
            report::push_window(rec);
            return None;
        }
        {
            match report::phase("present", ctx, || {
                render::present(&target.surface, &gpu.device, &gpu.queue, &pipeline)
            }) {
                Ok(status) => rec["present_immediate"] = json!(status),
                Err(e) => report::error("present", ctx, e),
            }
        }
        report::push_window(rec);
        Some((pipeline, config.format))
    }

    /// Keeps a window alive so `about_to_wait` can present once the window is visible.
    fn keep(
        &mut self,
        gpu: &Gpu,
        target: Target,
        ctx: &str,
        pipeline: Option<(wgpu::RenderPipeline, wgpu::TextureFormat)>,
    ) {
        if let Some((pipeline, format)) = pipeline {
            self.live.push(PresentJob {
                ctx: ctx.to_string(),
                target,
                device: gpu.device.clone(),
                queue: gpu.queue.clone(),
                format,
                pipeline,
                attempts: 0,
                done: false,
            });
        }
    }

    fn with_device_errors(&mut self, gpu: &Gpu) {
        gpu.device.on_uncaptured_error(Arc::new(|e| {
            report::error(
                "uncaptured",
                "device",
                format!("uncaptured wgpu error: {e}"),
            );
        }));
        report::set("selected_adapter", adapter_info(&gpu.adapter));
        report::set(
            "selected_device",
            json!({"features": format!("{:?}", gpu.device.features()),
                   "limits": format!("{:?}", gpu.device.limits())}),
        );
        report::set("dx12_modules", diag::dx12_modules());
        if self.args.probe == probe::Point::AfterFirstDevice && !self.probe_done {
            self.probe_done = true;
            // The device is in use, so a rejection here changes nothing.
            probe::run(
                self.args.probe,
                self.backends,
                false,
                self.args.probe_validation,
            );
        }
        if self.args.probe != probe::Point::None {
            // Any probe has run by now, so this shows whether it removed the device.
            report::set("dx12_after_probe", diag::dx12_device_state(&gpu.device));
        }
    }

    /// Mirrors Slint's `new_suspended` when the probe rejected WGPU.
    /// `drop` also releases the prepared result, as `dx12-worker-safety.md` section 6 proposes.
    fn on_probe_rejected(&mut self) {
        let mode = self.args.on_reject;
        report::event(
            "probe_rejected",
            json!({"on_reject": mode.name(), "worker_stage": worker::stage().name()}),
        );
        match mode {
            probe::OnReject::Continue => {}
            probe::OnReject::Keep => self.skip_gpu = true,
            probe::OnReject::Drop => {
                self.skip_gpu = true;
                self.release_prepared("probe-rejection");
            }
        }
        report::set("gpu_skipped", json!(self.skip_gpu));
    }

    /// Joins the pending worker and drops its result, recording whether it held a device.
    fn release_prepared(&mut self, when: &str) {
        drop(self.gate_release.take());
        let Some(mut pending) = self.pending.take() else {
            report::set(
                "prepared_release",
                json!({"when": when, "had_worker": false}),
            );
            return;
        };
        let stage = worker::stage();
        let start = report::now_ms();
        let had_device = report::phase("drop_pending", "main", || {
            worker::autoreleasepool(|| {
                let out = pending.take();
                let had = out.as_ref().map(|o| o.device.is_some());
                drop(out);
                had
            })
        });
        report::set(
            "prepared_release",
            json!({"when": when, "had_worker": true, "worker_stage_before": stage.name(),
                   "had_device": had_device, "released_at_ms": report::round3(report::now_ms()),
                   "join_and_drop_ms": report::round3(report::now_ms() - start)}),
        );
    }

    /// Blocks the main thread in a plain join while the worker waits at its gate.
    /// A helper thread releases the gate after `--release-after-ms`.
    fn join_blocked(&mut self) {
        let reached = self
            .gate_reached
            .take()
            .map(|rx| rx.recv_timeout(Duration::from_secs(30)));
        let reached_name = match &reached {
            Some(Ok(p)) => json!(p.name()),
            Some(Err(e)) => json!(format!("not reached: {e}")),
            None => Value::Null,
        };
        let release = self.gate_release.take();
        let delay = self.args.release_after;
        let released_at = Arc::new(std::sync::Mutex::new(None::<f64>));
        let released = released_at.clone();
        let _ = std::thread::Builder::new()
            .name("releaser".into())
            .spawn(move || {
                std::thread::sleep(delay);
                *released.lock().unwrap() = Some(report::now_ms());
                drop(release);
            });
        let start = report::now_ms();
        let out = self.pending.as_mut().and_then(|p| p.take());
        let end = report::now_ms();
        self.pending = None;
        report::set(
            "join_block",
            json!({"join_at": self.args.join_at.name(), "requested": self.args.abandon_at.name(),
                   "reached": reached_name, "release_after_ms": delay.as_millis() as u64,
                   "join_start_ms": report::round3(start), "join_end_ms": report::round3(end),
                   "join_ms": report::round3(end - start),
                   "released_at_ms": released_at.lock().unwrap().map(report::round3),
                   "worker_result_ok": out.as_ref().map(|o| o.error.is_none())}),
        );
        self.prejoined = out;
    }

    fn scenario_single(&mut self, el: &ActiveEventLoop) {
        let Some(window) = self.create_window(el, "w0") else {
            return;
        };
        self.single_on(window);
    }

    fn single_on(&mut self, window: Arc<Window>) {
        let Some((gpu, surface)) = self.acquire(&window, "w0") else {
            return;
        };
        self.with_device_errors(&gpu);
        self.list_adapters(&gpu, &surface);
        let target = Target { window, surface };
        let pipeline = self.window_work(&gpu, &target, "w0", "single");
        self.keep(&gpu, target, "w0", pipeline);
    }

    fn scenario_multi(&mut self, el: &ActiveEventLoop) {
        let n = self.args.windows;
        let Some(window) = self.create_window(el, "seq0") else {
            return;
        };
        let Some((gpu, surface)) = self.acquire(&window, "seq0") else {
            return;
        };
        self.with_device_errors(&gpu);
        self.list_adapters(&gpu, &surface);
        self.window_work(&gpu, &Target { window, surface }, "seq0", "sequential");
        for i in 1..n {
            let ctx = format!("seq{i}");
            let Some(window) = self.create_window(el, &ctx) else {
                return;
            };
            let Some(surface) = self.create_surface(&gpu.instance, &window, &ctx) else {
                return;
            };
            self.window_work(&gpu, &Target { window, surface }, &ctx, "sequential");
        }
        let mut targets = Vec::new();
        for i in 0..n {
            let ctx = format!("sim{i}");
            let Some(window) = self.create_window(el, &ctx) else {
                return;
            };
            let Some(surface) = self.create_surface(&gpu.instance, &window, &ctx) else {
                return;
            };
            targets.push((ctx, Target { window, surface }));
        }
        report::event(
            "simultaneous_windows_alive",
            json!({"count": targets.len()}),
        );
        for (ctx, t) in targets {
            let pipeline = self.window_work(&gpu, &t, &ctx, "simultaneous");
            self.keep(&gpu, t, &ctx, pipeline);
        }
    }

    fn scenario_abandon(&mut self, el: &ActiveEventLoop) {
        let at = self.args.abandon_at;
        if self.args.strategy == Strategy::Ordinary {
            report::event(
                "abandon_not_applicable",
                json!({"reason": "ordinary has no worker"}),
            );
        } else {
            if self.args.strategy == Strategy::WorkerAfterSelect {
                // This strategy's worker starts only after main-thread selection.
                let instance = slint_init::create_instance(self.backends);
                match slint_init::select_adapter(&instance, None) {
                    Ok(adapter) => {
                        let (gate, release, reached) = gate_for(&self.args);
                        self.gate_release = release;
                        self.gate_reached = reached;
                        self.pending = Some(Pending::start(
                            Job::DeviceFor { adapter },
                            Failure::None,
                            gate,
                        ));
                    }
                    Err(e) => report::error("adapter", "abandon", e),
                }
            }
            let reached = self
                .gate_reached
                .take()
                .map(|rx| rx.recv_timeout(Duration::from_secs(30)));
            let reached_name = match &reached {
                Some(Ok(p)) => json!(p.name()),
                Some(Err(e)) => json!(format!("not reached: {e}")),
                None => Value::Null,
            };
            report::event(
                "gate_reached",
                json!({"requested": at.name(), "reached": reached_name}),
            );
            // Dropping the sender releases the worker, as an app exit would leave it running.
            drop(self.gate_release.take());
            let pending = self.pending.take();
            let abandon_start = report::now_ms();
            let (finished, result_ok) = match pending {
                Some(p) => p.abandon(self.args.timeout / 2),
                None => (true, None),
            };
            report::set(
                "abandon",
                json!({"requested_at": at.name(), "reached": reached_name, "finished_before_timeout": finished,
                       "worker_result_ok": result_ok,
                       "join_ms": report::round3(report::now_ms() - abandon_start)}),
            );
            if !finished {
                report::error(
                    "abandon",
                    "main",
                    "worker did not finish before the abandon timeout",
                );
            }
        }
        // Confirm ordinary initialization still works after the abandoned worker.
        let strategy = self.args.strategy;
        self.args.strategy = Strategy::Ordinary;
        let Some(window) = self.create_window(el, "post-abandon") else {
            return;
        };
        if let Some((gpu, surface)) = self.acquire(&window, "post-abandon") {
            self.with_device_errors(&gpu);
            self.list_adapters(&gpu, &surface);
            let target = Target { window, surface };
            let pipeline =
                self.window_work(&gpu, &target, "post-abandon", "ordinary-after-abandon");
            self.keep(&gpu, target, "post-abandon", pipeline);
        }
        self.args.strategy = strategy;
    }

    fn scenario_failure(&mut self, el: &ActiveEventLoop) {
        if let Err(e) = slint_init::check_backends(self.backends) {
            report::error("backends", "main", e);
            return;
        }
        let Some(window) = self.create_window(el, "w0") else {
            return;
        };
        if let Some((gpu, surface)) = self.acquire(&window, "w0") {
            report::event(
                "failure_did_not_fail",
                json!({"failure": self.args.failure.name()}),
            );
            self.with_device_errors(&gpu);
            self.window_work(&gpu, &Target { window, surface }, "w0", "failure");
        }
    }

    fn scenario_join_block(&mut self, el: &ActiveEventLoop) {
        let Some(window) = self.create_window(el, "w0") else {
            return;
        };
        let gated = matches!(
            self.args.strategy,
            Strategy::WorkerDevice | Strategy::WorkerInstance
        );
        if !gated {
            report::event(
                "join_block_not_applicable",
                json!({"reason": "no worker starts before the event loop"}),
            );
        } else if self.args.join_at == JoinAt::AfterWindow {
            // The window is visible and its messages go unpumped until the join returns.
            report::event("join_with_window", json!({"visible": window.is_visible()}));
            self.join_blocked();
        }
        self.single_on(window);
    }

    fn scenario_repeated(&mut self, el: &ActiveEventLoop) {
        let Some(window) = self.create_window(el, "r0") else {
            return;
        };
        for i in 0..self.args.repeat {
            let ctx = format!("r{i}");
            if i > 0 {
                self.pending = start_early(self.args.strategy, self.backends, Failure::None, None);
            }
            let Some((gpu, surface)) = self.acquire(&window, &ctx) else {
                return;
            };
            self.with_device_errors(&gpu);
            self.list_adapters(&gpu, &surface);
            let target = Target {
                window: window.clone(),
                surface,
            };
            let pipeline = self.window_work(&gpu, &target, &ctx, "repeated");
            if i + 1 == self.args.repeat {
                self.keep(&gpu, target, &ctx, pipeline);
                return;
            }
            report::phase("teardown", &ctx, || {
                drop(target);
                gpu.device.destroy();
                drop(gpu);
            });
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.done {
            return;
        }
        self.done = true;
        report::event("resumed", json!({}));
        if self.skip_gpu {
            // Stands in for the next renderer in Slint's fallback chain, which uses no GPU.
            self.stand_in = self.create_window(event_loop, "stand-in");
            report::event("gpu_skipped", json!({}));
            self.linger_until = Some(std::time::Instant::now() + self.args.linger);
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                self.linger_until.unwrap(),
            ));
            return;
        }
        if std::env::var_os("BENCH_TEST_HANG").is_some() {
            // Exercises the watchdog exit path.
            report::phase("test_hang", "main", || std::thread::sleep(Duration::MAX));
        }
        worker::autoreleasepool(|| match self.args.scenario {
            Scenario::Single => self.scenario_single(event_loop),
            Scenario::MultiWindow => self.scenario_multi(event_loop),
            Scenario::Abandon => self.scenario_abandon(event_loop),
            Scenario::Failure => self.scenario_failure(event_loop),
            Scenario::Repeated => self.scenario_repeated(event_loop),
            Scenario::JoinBlock => self.scenario_join_block(event_loop),
            // `main` exits before the event loop.
            Scenario::EarlyExit => {}
        });
        if self.live.is_empty() {
            event_loop.exit();
        } else {
            self.present_deadline = Some(std::time::Instant::now() + PRESENT_DEADLINE);
            for job in &self.live {
                job.target.window.request_redraw();
            }
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(until) = self.linger_until {
            if std::time::Instant::now() >= until {
                self.linger_until = None;
                self.stand_in = None;
                event_loop.exit();
            } else {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(until));
            }
            return;
        }
        let Some(deadline) = self.present_deadline else {
            return;
        };
        let expired = std::time::Instant::now() >= deadline;
        for job in self.live.iter_mut().filter(|j| !j.done) {
            job.attempts += 1;
            let result =
                render::present(&job.target.surface, &job.device, &job.queue, &job.pipeline);
            let finished = match &result {
                Ok("occluded") => expired,
                _ => true,
            };
            if finished {
                job.done = true;
                let at = report::round3(report::now_ms());
                let value = match result {
                    Ok(status) => json!({"status": status, "attempts": job.attempts, "at_ms": at}),
                    Err(e) => {
                        report::error("present_deferred", &job.ctx, e.clone());
                        json!({"status": "error", "error": e, "attempts": job.attempts, "at_ms": at})
                    }
                };
                report::update_window(&job.ctx, "present_deferred", value);
            }
        }
        if self.live.iter().all(|j| j.done) {
            self.present_deadline = None;
            for job in &self.live {
                after_present(job);
            }
            worker::autoreleasepool(|| self.live.clear());
            event_loop.exit();
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                std::time::Instant::now() + Duration::from_millis(10),
            ));
        }
    }
}

/// Checks the device again once the deferred present finished:
/// its native DX12 state, and a second offscreen render that must still read back the scene.
fn after_present(job: &PresentJob) {
    let offscreen = render::offscreen(&job.device, &job.queue, &job.pipeline, job.format);
    let value = json!({
        "dx12": diag::dx12_device_state(&job.device),
        "offscreen": offscreen.as_ref().ok(),
        "offscreen_error": offscreen.as_ref().err(),
    });
    if let Err(e) = &offscreen {
        report::error("after_present", &job.ctx, e.clone());
    }
    report::update_window(&job.ctx, "after_present", value);
}

fn metadata(
    args: &Args,
    requested: wgpu::Backends,
    effective: wgpu::Backends,
) -> serde_json::Map<String, Value> {
    let env: serde_json::Map<String, Value> = ENV_KEYS
        .iter()
        .map(|k| {
            (
                k.to_string(),
                std::env::var(k).map(Value::from).unwrap_or(Value::Null),
            )
        })
        .collect();
    let enabled = wgpu::Instance::enabled_backend_features();
    let v = json!({
        "harness": env!("CARGO_PKG_VERSION"),
        "wgpu": "30.0.1",
        "strategy": args.strategy.name(),
        "scenario": args.scenario.name(),
        "options": {
            "windows": args.windows, "repeat": args.repeat, "abandon_at": args.abandon_at.name(),
            "failure": args.failure.name(), "timeout_s": args.timeout.as_secs(),
            "probe": args.probe.name(), "on_probe_reject": args.on_reject.name(),
            "join_at": args.join_at.name(), "release_after_ms": args.release_after.as_millis() as u64,
            "exit_at": args.exit_at.name(), "linger_ms": args.linger.as_millis() as u64,
            "probe_validation": args.probe_validation,
        },
        "build": {
            "debug_assertions": cfg!(debug_assertions),
            "instance_flags": format!("{:?}", wgpu::InstanceFlags::from_build_config().with_env()),
            "dx12_compiler": format!("{:?}", wgpu::BackendOptions::from_env_or_default().dx12.shader_compiler),
        },
        "platform": {
            "os": std::env::consts::OS, "arch": std::env::consts::ARCH, "family": std::env::consts::FAMILY,
        },
        "env": env,
        "backends": {
            "requested": format!("{requested:?}"),
            "avoid": format!("{:?}", slint_init::default_backends_to_avoid()),
            "effective": format!("{effective:?}"),
            "enabled_features": format!("{enabled:?}"),
            "effective_and_enabled": format!("{:?}", effective & enabled),
            "slint_would_prepare_on_worker": slint_init::slint_would_prepare(effective),
            "dx12_gate_would_prepare": slint_init::would_prepare_for(effective, wgpu::Backends::DX12),
        },
        "phases": [], "errors": [], "events": [], "windows": [],
    });
    match v {
        Value::Object(m) => m,
        _ => unreachable!(),
    }
}

/// Exits the process while the worker is inside the call `target` names, without joining it.
fn early_exit(target: Stage, has_worker: bool, out: Option<&str>) -> ! {
    let decided = if has_worker {
        worker::wait_for_stage(target, Duration::from_secs(30))
    } else {
        worker::stage()
    };
    let mut detail = json!({"requested": target.name(), "has_worker": has_worker,
                            "stage_at_decision": decided.name()});
    report::set("early_exit", detail.clone());
    report::write(out, "early-exit");
    // A worker still at `decided` after one record write was inside that call for at least as long.
    detail["stage_after_first_write"] = json!(worker::stage().name());
    report::set("early_exit", detail);
    report::write(out, "early-exit");
    std::process::exit(EXIT_EARLY)
}

fn main() {
    report::start_clock();
    let args = parse_args();
    let requested = slint_init::requested_backends(args.failure);
    let backends = slint_init::mask_backends(requested, slint_init::default_backends_to_avoid());
    report::init(metadata(&args, requested, backends));
    logcap::install();

    let out = args.out.clone();
    {
        let out = out.clone();
        std::panic::set_hook(Box::new(move |info| {
            eprintln!("panic: {info}");
            report::error(
                "panic",
                std::thread::current().name().unwrap_or("unnamed"),
                format!("panic: {info}"),
            );
            if std::thread::current().name() == Some("main") {
                report::write(out.as_deref(), "panic");
                std::process::exit(EXIT_PANIC);
            }
        }));
    }
    let (done_tx, done_rx) = mpsc::channel::<()>();
    {
        let out = out.clone();
        let timeout = args.timeout;
        std::thread::Builder::new()
            .name("watchdog".into())
            .spawn(move || {
                if let Err(mpsc::RecvTimeoutError::Timeout) = done_rx.recv_timeout(timeout) {
                    report::error(
                        "watchdog",
                        "",
                        format!(
                            "timed out after {}s during {}",
                            timeout.as_secs(),
                            report::current_phase()
                        ),
                    );
                    report::write(out.as_deref(), "watchdog-timeout");
                    std::process::exit(EXIT_WATCHDOG);
                }
            })
            .unwrap();
    }

    let mut app = App {
        live: Vec::new(),
        present_deadline: None,
        pending: None,
        gate_release: None,
        gate_reached: None,
        done: false,
        listed_adapters: false,
        prejoined: None,
        skip_gpu: false,
        probe_done: false,
        linger_until: None,
        stand_in: None,
        backends,
        args,
    };
    let backends_ok = slint_init::check_backends(backends).is_ok();
    if backends_ok && app.args.strategy != Strategy::WorkerAfterSelect {
        let (gate, release, reached) = gate_for(&app.args);
        app.gate_release = release;
        app.gate_reached = reached;
        app.pending = start_early(app.args.strategy, backends, app.args.failure, gate);
    }
    if app.args.scenario == Scenario::EarlyExit {
        early_exit(app.args.exit_at, app.pending.is_some(), out.as_deref());
    }
    if app.args.scenario == Scenario::JoinBlock
        && app.args.join_at == JoinAt::BeforeEventLoop
        && app.pending.is_some()
    {
        app.join_blocked();
    }
    match report::phase("event_loop", "main", EventLoop::new) {
        Ok(event_loop) => {
            // Slint's `new_suspended` probes after the backend created the event loop.
            if backends_ok
                && app.args.probe.worker_stage().is_some()
                && probe::run(
                    app.args.probe,
                    backends,
                    app.pending.is_some(),
                    app.args.probe_validation,
                )
            {
                app.on_probe_rejected();
            }
            if let Err(e) = event_loop.run_app(&mut app) {
                report::error("event_loop_run", "main", format!("event loop failed: {e}"));
            }
        }
        Err(e) => report::error(
            "event_loop",
            "main",
            format!("event loop creation failed: {e}"),
        ),
    }
    // Any worker still pending is abandoned here, as Slint's `Drop` would.
    drop(app.gate_release.take());
    if app.pending.is_some() {
        report::event("pending_dropped_at_exit", json!({}));
        app.release_prepared("exit");
    }
    let _ = done_tx.send(());
    report::write(out.as_deref(), "completed");
}
