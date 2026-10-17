//! The JSON run record and the helpers every phase writes through.

use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    None,
    Limits,
    EmptyMask,
    UnsupportedMask,
}

impl Failure {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "none" => Self::None,
            "limits" => Self::Limits,
            "empty-mask" => Self::EmptyMask,
            "unsupported-mask" => Self::UnsupportedMask,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Limits => "limits",
            Self::EmptyMask => "empty-mask",
            Self::UnsupportedMask => "unsupported-mask",
        }
    }
}

static START: OnceLock<Instant> = OnceLock::new();
static PENDING_POLLS: AtomicU64 = AtomicU64::new(0);
static REPORT: Mutex<Option<Map<String, Value>>> = Mutex::new(None);
static CURRENT_PHASE: Mutex<String> = Mutex::new(String::new());

pub fn start_clock() {
    START.get_or_init(Instant::now);
}

pub fn now_ms() -> f64 {
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

pub fn note_pending_poll() {
    PENDING_POLLS.fetch_add(1, Ordering::Relaxed);
}

pub fn pending_polls() -> u64 {
    PENDING_POLLS.load(Ordering::Relaxed)
}

pub fn init(root: Map<String, Value>) {
    *REPORT.lock().unwrap() = Some(root);
}

pub fn with<R>(f: impl FnOnce(&mut Map<String, Value>) -> R) -> R {
    let mut guard = REPORT.lock().unwrap_or_else(|e| e.into_inner());
    f(guard.as_mut().expect("report not initialized"))
}

pub fn set(key: &str, value: Value) {
    with(|r| {
        r.insert(key.to_string(), value);
    });
}

pub fn push(key: &str, value: Value) {
    with(|r| {
        let entry = r
            .entry(key.to_string())
            .or_insert_with(|| Value::Array(vec![]));
        entry.as_array_mut().unwrap().push(value);
    });
}

pub fn update_window(context: &str, key: &str, value: Value) {
    with(|r| {
        if let Some(Value::Array(windows)) = r.get_mut("windows") {
            if let Some(w) = windows.iter_mut().rev().find(|w| w["context"] == context) {
                w[key] = value;
            }
        }
    });
}

pub fn current_phase() -> String {
    CURRENT_PHASE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn thread_name() -> String {
    std::thread::current()
        .name()
        .unwrap_or("unnamed")
        .to_string()
}

/// Records one phase's wall-clock interval, measured from process start.
pub fn phase<R>(name: &str, context: &str, f: impl FnOnce() -> R) -> R {
    let label = if context.is_empty() {
        name.to_string()
    } else {
        format!("{context}/{name}")
    };
    *CURRENT_PHASE.lock().unwrap_or_else(|e| e.into_inner()) =
        format!("{label} on {}", thread_name());
    let start = now_ms();
    let out = f();
    let end = now_ms();
    push(
        "phases",
        json!({"name": name, "context": context, "thread": thread_name(),
               "start_ms": round3(start), "end_ms": round3(end), "duration_ms": round3(end - start)}),
    );
    out
}

pub fn error(phase: &str, context: &str, message: impl Into<String>) {
    let message = message.into();
    eprintln!("error [{context}/{phase}]: {message}");
    push(
        "errors",
        json!({"phase": phase, "context": context, "class": classify(&message), "message": message,
               "at_ms": round3(now_ms())}),
    );
}

pub fn event(name: &str, detail: Value) {
    push(
        "events",
        json!({"name": name, "at_ms": round3(now_ms()), "detail": detail}),
    );
}

pub fn push_window(value: Value) {
    push("windows", value);
}

pub fn push_log(value: Value) {
    push("log", value);
}

/// A stable error class, so runs compare by kind rather than by driver wording.
pub fn classify(message: &str) -> &'static str {
    let m = message.to_lowercase();
    if m.contains("device lost") {
        "device-lost"
    } else if m.contains("not supported under wgpu") {
        "no-backends"
    } else if m.contains("appropriate adapter") {
        "adapter-not-found"
    } else if m.contains("create device") {
        "device-request-failed"
    } else if m.contains("not supported by the adapter") || m.contains("surface-unsupported") {
        "surface-unsupported"
    } else if m.contains("window surface") {
        "surface-creation-failed"
    } else if m.contains("window") {
        "window-creation-failed"
    } else if m.contains("panic") {
        "panic"
    } else if m.contains("timed out") || m.contains("timeout") {
        "timeout"
    } else if m.contains("surface configure") {
        "surface-configure-failed"
    } else if m.contains("validation") || m.contains("uncaptured") {
        "wgpu-validation"
    } else {
        "other"
    }
}

pub fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

pub fn snapshot() -> Value {
    with(|r| Value::Object(r.clone()))
}

pub fn write(path: Option<&str>, status: &str) {
    set("status", json!(status));
    set("pending_polls", json!(pending_polls()));
    set("log_dropped", json!(crate::logcap::dropped()));
    set("finished_ms", json!(round3(now_ms())));
    let text = serde_json::to_string_pretty(&snapshot()).unwrap();
    match path {
        Some(p) => {
            if let Err(e) = std::fs::write(p, text + "\n") {
                eprintln!("failed to write {p}: {e}");
            }
        }
        None => println!("{text}"),
    }
}
