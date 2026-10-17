//! Opens Slint windows with the default renderer and reports, per window, the renderer,
//! the WGPU adapter, and a hash of `Window::take_snapshot`.
//!
//! Usage: `slint-dx12-probe <scenario>`, where the scenario is one of `single`, `second-window`,
//! `quit-early`, `exit-immediately`, `exit-after-backend`, `process-exit-after-backend` or
//! `exit-after-prepared`.
//! Every output line is `event=<name>` followed by `key=value` pairs.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::ComponentHandle;

slint::slint! {
    export component ProbeWindow inherits Window {
        in property <string> label;
        width: 320px;
        height: 200px;
        background: #172331;
        Rectangle {
            x: 20px;
            y: 20px;
            width: 120px;
            height: 80px;
            border-radius: 12px;
            background: @linear-gradient(135deg, #398bc8 0%, #84dbc0 100%);
        }
        Rectangle {
            x: 180px;
            y: 30px;
            width: 110px;
            height: 110px;
            border-width: 4px;
            border-color: #f1a340;
            border-radius: 55px;
        }
        Text {
            x: 20px;
            y: 140px;
            text: root.label;
            color: #f1f6fc;
            font-size: 18px;
        }
    }
}

type Error = Box<dyn std::error::Error>;

thread_local! {
    static START: Instant = Instant::now();
}

fn elapsed_ms() -> f64 {
    START.with(|start| start.elapsed().as_secs_f64() * 1000.0)
}

fn fnv1a(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3))
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;

fn report_snapshot(index: usize, window: &slint::Window) {
    match window.take_snapshot() {
        Ok(buffer) => {
            let mut hash = fnv1a(FNV_OFFSET, &buffer.width().to_le_bytes());
            hash = fnv1a(hash, &buffer.height().to_le_bytes());
            hash = fnv1a(hash, buffer.as_bytes());
            println!(
                "event=snapshot window={index} width={} height={} fnv1a={hash:016x} t_ms={:.1}",
                buffer.width(),
                buffer.height(),
                elapsed_ms()
            );
        }
        Err(e) => println!("event=snapshot-error window={index} message={:?}", e.to_string()),
    }
}

/// Opens window `index`, snapshots it after its first frame, then opens the next or quits.
fn open_window(index: usize, total: usize, windows: Rc<RefCell<Vec<ProbeWindow>>>) {
    let ui = match ProbeWindow::new() {
        Ok(ui) => ui,
        Err(e) => {
            println!("event=error window={index} phase=create message={:?}", e.to_string());
            let _ = slint::quit_event_loop();
            return;
        }
    };
    ui.set_label(format!("window {index}").into());
    let rendered = Rc::new(std::cell::Cell::new(false));
    let weak = ui.as_weak();
    let next_windows = windows.clone();
    let after_first_frame = move || {
        let Some(ui) = weak.upgrade() else { return };
        report_snapshot(index, ui.window());
        if index < total {
            open_window(index + 1, total, next_windows.clone());
        } else {
            let _ = slint::quit_event_loop();
        }
    };
    let notifier = {
        let rendered = rendered.clone();
        let after_first_frame = after_first_frame.clone();
        ui.window().set_rendering_notifier(move |state, api| match state {
            slint::RenderingState::RenderingSetup => match api {
                slint::GraphicsAPI::WGPU30 { device, .. } => {
                    let info = device.adapter_info();
                    let limits = fnv1a(
                        FNV_OFFSET,
                        format!("{:?} {:?}", device.limits(), device.features()).as_bytes(),
                    );
                    println!(
                        "event=setup window={index} renderer=femtovg-wgpu adapter={:?} backend={:?} device_type={:?} features_limits_fnv1a={limits:016x} t_ms={:.1}",
                        info.name,
                        info.backend,
                        info.device_type,
                        elapsed_ms()
                    );
                }
                other => println!("event=setup window={index} graphics_api={other:?}"),
            },
            slint::RenderingState::AfterRendering => {
                if !rendered.replace(true) {
                    println!("event=first-frame window={index} t_ms={:.1}", elapsed_ms());
                    slint::Timer::single_shot(Duration::ZERO, after_first_frame.clone());
                }
            }
            _ => {}
        })
    };
    if let Err(e) = notifier {
        // The software renderer has no rendering notifier; snapshot it after the window is shown.
        println!("event=setup window={index} renderer=non-wgpu notifier_error={:?}", e.to_string());
        slint::Timer::single_shot(Duration::from_millis(500), after_first_frame);
    }
    if let Err(e) = ui.show() {
        println!("event=error window={index} phase=show message={:?}", e.to_string());
        let _ = slint::quit_event_loop();
    }
    windows.borrow_mut().push(ui);
}

fn run_windows(total: usize) -> Result<(), Error> {
    let windows = Rc::new(RefCell::new(Vec::new()));
    open_window(1, total, windows.clone());
    slint::run_event_loop_until_quit()?;
    let count = windows.borrow().len();
    windows.borrow_mut().clear();
    println!("event=windows-dropped count={count} t_ms={:.1}", elapsed_ms());
    Ok(())
}

fn quit_early() -> Result<(), Error> {
    let ui = ProbeWindow::new()?;
    ui.show()?;
    ui.hide()?;
    slint::Timer::single_shot(Duration::ZERO, || {
        let _ = slint::quit_event_loop();
    });
    slint::run_event_loop_until_quit()?;
    drop(ui);
    println!("event=quit t_ms={:.1}", elapsed_ms());
    Ok(())
}

fn exit_immediately() -> Result<(), Error> {
    let _ui = ProbeWindow::new()?;
    println!("event=created t_ms={:.1}", elapsed_ms());
    Ok(())
}

/// Creates the backend, which starts device preparation, and returns while the worker likely runs.
fn exit_after_backend() -> Result<(), Error> {
    slint::BackendSelector::new().select()?;
    println!("event=backend-created t_ms={:.1}", elapsed_ms());
    Ok(())
}

/// Like `exit_after_backend`, but ends with `std::process::exit` from inside `main`.
fn process_exit_after_backend() -> Result<(), Error> {
    exit_after_backend()?;
    println!("event=process-exit code=0 t_ms={:.1}", elapsed_ms());
    std::process::exit(0);
}

/// Creates a component and waits long enough for the worker to finish before returning.
fn exit_after_prepared() -> Result<(), Error> {
    let _ui = ProbeWindow::new()?;
    println!("event=created t_ms={:.1}", elapsed_ms());
    std::thread::sleep(Duration::from_secs(1));
    println!("event=waited t_ms={:.1}", elapsed_ms());
    Ok(())
}

fn main() {
    elapsed_ms();
    let scenario = std::env::args().nth(1).unwrap_or_else(|| "single".into());
    let env = |name| std::env::var(name).ok();
    println!(
        "event=start scenario={scenario} WGPU_BACKEND={:?} SLINT_WGPU_CPU={:?} WGPU_VALIDATION={:?}",
        env("WGPU_BACKEND"),
        env("SLINT_WGPU_CPU"),
        env("WGPU_VALIDATION")
    );
    let result = match scenario.as_str() {
        "single" => run_windows(1),
        "second-window" => run_windows(2),
        "quit-early" => quit_early(),
        "exit-immediately" => exit_immediately(),
        "exit-after-backend" => exit_after_backend(),
        "process-exit-after-backend" => process_exit_after_backend(),
        "exit-after-prepared" => exit_after_prepared(),
        other => Err(format!("unknown scenario {other}").into()),
    };
    match result {
        Ok(()) => println!("event=exit code=0 t_ms={:.1}", elapsed_ms()),
        Err(e) => {
            println!("event=error phase=main message={:?}", e.to_string());
            std::process::exit(2);
        }
    }
}
