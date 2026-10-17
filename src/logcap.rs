//! Captures `log` records from WGPU into the run record.
//!
//! The D3D12 debug layer reaches the log through `wgpu-hal`'s exception handler,
//! which a DX12 device installs under `InstanceFlags::VALIDATION`.

use crate::report;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_RECORDS: usize = 400;
const MAX_MESSAGE_CHARS: usize = 600;

static KEPT: AtomicUsize = AtomicUsize::new(0);
static DROPPED: AtomicUsize = AtomicUsize::new(0);

struct Capture;

/// Debug and info records from these targets are kept, and only warnings and errors from the rest.
const VERBOSE_TARGETS: &[&str] = &["wgpu_hal", "wgpu_core::instance"];

fn keep(meta: &log::Metadata) -> bool {
    meta.level() <= log::Level::Warn
        || (meta.level() <= log::Level::Debug
            && VERBOSE_TARGETS.iter().any(|t| meta.target().starts_with(t)))
}

impl log::Log for Capture {
    fn enabled(&self, meta: &log::Metadata) -> bool {
        keep(meta)
    }

    fn log(&self, record: &log::Record) {
        if !keep(record.metadata()) {
            return;
        }
        let message = record.args().to_string();
        if let Some(compiler) = dx12_compiler(&message) {
            report::set("dx12_compiler", json!(compiler));
        }
        if record.level() <= log::Level::Warn {
            eprintln!("log {} [{}]: {message}", record.level(), record.target());
        }
        if KEPT.fetch_add(1, Ordering::Relaxed) >= MAX_RECORDS {
            DROPPED.fetch_add(1, Ordering::Relaxed);
            return;
        }
        let thread = std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_string();
        let message: String = message.chars().take(MAX_MESSAGE_CHARS).collect();
        report::push_log(json!({
            "level": record.level().as_str(), "target": record.target(), "thread": thread,
            "at_ms": report::round3(report::now_ms()), "message": message,
        }));
    }

    fn flush(&self) {}
}

/// The compiler that `wgpu-hal`'s DX12 instance reports at debug level.
fn dx12_compiler(message: &str) -> Option<&'static str> {
    match message {
        "Using dynamic DXC for shader compilation" => Some("dynamic-dxc"),
        "Using static DXC for shader compilation" => Some("static-dxc"),
        "Using FXC for shader compilation" => Some("fxc"),
        _ => None,
    }
}

pub fn install() {
    if log::set_logger(&Capture).is_ok() {
        log::set_max_level(log::LevelFilter::Debug);
    }
}

pub fn dropped() -> usize {
    DROPPED.load(Ordering::Relaxed)
}
