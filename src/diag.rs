//! Device diagnostics the scenarios record: HAL counters, device loss, and DX12 native state.

use crate::report;
use serde_json::{json, Value};

/// The HAL objects a new device already holds, recorded right after `request_device`.
/// `compute_pipelines` counts the internal indirect-validation pipelines.
pub fn record_new_device(device: &wgpu::Device, context: &str) {
    let hal = device.get_internal_counters().hal;
    report::push(
        "device_counters",
        json!({
            "context": context,
            "thread": std::thread::current().name().unwrap_or("unnamed"),
            "compute_pipelines": hal.compute_pipelines.read(),
            "render_pipelines": hal.render_pipelines.read(),
            "shader_modules": hal.shader_modules.read(),
            "pipeline_layouts": hal.pipeline_layouts.read(),
            "bind_group_layouts": hal.bind_group_layouts.read(),
            "buffers": hal.buffers.read(),
        }),
    );
    let context = context.to_string();
    device.set_device_lost_callback(move |reason, message| {
        if reason == wgpu::DeviceLostReason::Destroyed {
            // The `repeated` scenario calls `Device::destroy`.
            report::event("device_destroyed", json!({"context": context}));
            return;
        }
        report::error(
            "device_lost",
            &context,
            format!("device lost: {reason:?}: {message}"),
        );
    });
}

/// Native DX12 state of a device: `GetDeviceRemovedReason`, whether the debug layer wraps it,
/// and the messages its info queue stored. `null` for other backends.
#[cfg(windows)]
pub fn dx12_device_state(device: &wgpu::Device) -> Value {
    use windows::core::Interface;
    use windows::Win32::Graphics::Direct3D12::{ID3D12DebugDevice, ID3D12InfoQueue, D3D12_MESSAGE};
    const MAX_MESSAGES: u64 = 50;
    // SAFETY: the raw device is only queried, and the guard outlives every call.
    let Some(hal) = (unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }) else {
        return Value::Null;
    };
    let raw = hal.raw_device();
    let removed = match unsafe { raw.GetDeviceRemovedReason() } {
        Ok(()) => "S_OK".to_string(),
        Err(e) => format!("{:#010x} {}", e.code().0, e.message()),
    };
    let debug_device = raw.cast::<ID3D12DebugDevice>().is_ok();
    let mut messages = Vec::new();
    let mut stored = Value::Null;
    if let Ok(queue) = raw.cast::<ID3D12InfoQueue>() {
        let n = unsafe { queue.GetNumStoredMessages() };
        stored = json!(n);
        for i in 0..n.min(MAX_MESSAGES) {
            let mut len = 0usize;
            if unsafe { queue.GetMessage(i, None, &mut len) }.is_err() || len == 0 {
                continue;
            }
            // D3D12_MESSAGE is followed by its description in the same allocation.
            let mut buf = vec![0u64; len.div_ceil(8)];
            let msg = buf.as_mut_ptr().cast::<D3D12_MESSAGE>();
            if unsafe { queue.GetMessage(i, Some(msg), &mut len) }.is_err() {
                continue;
            }
            let m = unsafe { &*msg };
            let text = if m.pDescription.is_null() {
                String::new()
            } else {
                // SAFETY: the runtime NUL-terminates the description inside `buf`.
                unsafe { std::ffi::CStr::from_ptr(m.pDescription.cast()) }
                    .to_string_lossy()
                    .into_owned()
            };
            messages.push(
                json!({"severity": m.Severity.0, "category": m.Category.0, "id": m.ID.0,
                                 "description": text.trim_end_matches('\0')}),
            );
        }
    }
    json!({"removed_reason": removed, "debug_device": debug_device,
           "info_queue_stored": stored, "info_queue": messages})
}

#[cfg(not(windows))]
pub fn dx12_device_state(_device: &wgpu::Device) -> Value {
    Value::Null
}

/// Whether each DX12-related DLL is loaded in this process, and where the search path finds it.
#[cfg(windows)]
pub fn dx12_modules() -> Value {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> *mut core::ffi::c_void;
        fn GetModuleFileNameW(module: *mut core::ffi::c_void, buf: *mut u16, len: u32) -> u32;
        fn SearchPathW(
            path: *const u16,
            file: *const u16,
            ext: *const u16,
            len: u32,
            buf: *mut u16,
            part: *mut *mut u16,
        ) -> u32;
    }
    let wide = |s: &str| -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let mut out = serde_json::Map::new();
    for name in [
        "dxcompiler.dll",
        "dxil.dll",
        "d3dcompiler_47.dll",
        "d3d12.dll",
        "d3d12core.dll",
        "d3d12sdklayers.dll",
        "d3d10warp.dll",
    ] {
        let w = wide(name);
        let mut buf = [0u16; 520];
        // SAFETY: plain Win32 calls with valid, NUL-terminated buffers.
        let handle = unsafe { GetModuleHandleW(w.as_ptr()) };
        let loaded = if handle.is_null() {
            Value::Null
        } else {
            let n = unsafe { GetModuleFileNameW(handle, buf.as_mut_ptr(), buf.len() as u32) };
            json!(String::from_utf16_lossy(&buf[..n as usize]))
        };
        let n = unsafe {
            SearchPathW(
                std::ptr::null(),
                w.as_ptr(),
                std::ptr::null(),
                buf.len() as u32,
                buf.as_mut_ptr(),
                std::ptr::null_mut(),
            )
        };
        let found = if n == 0 || n as usize > buf.len() {
            Value::Null
        } else {
            json!(String::from_utf16_lossy(&buf[..n as usize]))
        };
        out.insert(name.into(), json!({"loaded": loaded, "search_path": found}));
    }
    Value::Object(out)
}

#[cfg(not(windows))]
pub fn dx12_modules() -> Value {
    Value::Null
}
