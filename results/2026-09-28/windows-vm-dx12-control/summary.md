# Summary: windows-vm-dx12-control

Overall: FAIL

## Comparisons Against Ordinary

| Variant | Strategy | Verdict | Differences |
| --- | --- | --- | --- |
| control-late-debug-layer | worker-device | differ | status, adapter_order, formats, present_modes, pixel_hashes, present, error_classes |
| control-late-debug-layer | worker-instance | match | - |
| control-late-debug-layer | worker-after-select | match | - |
| control-late-debug-layer-first-device | worker-device | match | - |
| control-late-debug-layer-first-device | worker-instance | match | - |
| control-late-debug-layer-first-device | worker-after-select | match | - |

## Per-Case Sanity

| Strategy | Variant | OK | Problems |
| --- | --- | --- | --- |
| ordinary | control-late-debug-layer | yes | - |
| ordinary | control-late-debug-layer-first-device | no | status is 'panic'; device-lost in w0/device_lost: device lost: Unknown: Device is lost; wgpu-validation in device/uncaptured: uncaptured wgpu error: Validation Error

Caused by:
  In Buffer::map_async
    Buffer with 'bench-readback' label is invalid
; panic in main/panic: panic: panicked at <cargo-registry>/wgpu-30.0.1\src\backend\wgpu_core.rs:4036:26:
Error in Surface::get_current_texture_view: Validation Error

Caused by:
  Surface is not configured for presentation
; no window was rendered; device removed after the probe: 0x887a0007 The GPU will not respond to more commands, most likely because some other application submitted invalid commands.
The calling application should re-create the device and continue.; exit code 5 (panic), expected 0 |
| worker-after-select | control-late-debug-layer | yes | - |
| worker-after-select | control-late-debug-layer-first-device | no | status is 'panic'; device-lost in worker/device_lost: device lost: Unknown: Device is lost; wgpu-validation in device/uncaptured: uncaptured wgpu error: Validation Error

Caused by:
  In Buffer::map_async
    Buffer with 'bench-readback' label is invalid
; panic in main/panic: panic: panicked at <cargo-registry>/wgpu-30.0.1\src\backend\wgpu_core.rs:4036:26:
Error in Surface::get_current_texture_view: Validation Error

Caused by:
  Surface is not configured for presentation
; no window was rendered; device removed after the probe: 0x887a0007 The GPU will not respond to more commands, most likely because some other application submitted invalid commands.
The calling application should re-create the device and continue.; exit code 5 (panic), expected 0 |
| worker-device | control-late-debug-layer | no | status is 'panic'; device-lost in worker/device_lost: device lost: Unknown: Device is lost; wgpu-validation in device/uncaptured: uncaptured wgpu error: Validation Error

Caused by:
  In Buffer::map_async
    Buffer with 'bench-readback' label is invalid
; panic in main/panic: panic: panicked at <cargo-registry>/wgpu-30.0.1\src\backend\wgpu_core.rs:4036:26:
Error in Surface::get_current_texture_view: Validation Error

Caused by:
  Surface is not configured for presentation
; no window was rendered; device removed after the probe: 0x887a0007 The GPU will not respond to more commands, most likely because some other application submitted invalid commands.
The calling application should re-create the device and continue.; exit code 5 (panic), expected 0 |
| worker-device | control-late-debug-layer-first-device | no | status is 'panic'; device-lost in worker/device_lost: device lost: Unknown: Device is lost; wgpu-validation in device/uncaptured: uncaptured wgpu error: Validation Error

Caused by:
  In Buffer::map_async
    Buffer with 'bench-readback' label is invalid
; panic in main/panic: panic: panicked at <cargo-registry>/wgpu-30.0.1\src\backend\wgpu_core.rs:4036:26:
Error in Surface::get_current_texture_view: Validation Error

Caused by:
  Surface is not configured for presentation
; no window was rendered; device removed after the probe: 0x887a0007 The GPU will not respond to more commands, most likely because some other application submitted invalid commands.
The calling application should re-create the device and continue.; exit code 5 (panic), expected 0 |
| worker-instance | control-late-debug-layer | yes | - |
| worker-instance | control-late-debug-layer-first-device | no | status is 'panic'; device-lost in w0/device_lost: device lost: Unknown: Device is lost; wgpu-validation in device/uncaptured: uncaptured wgpu error: Validation Error

Caused by:
  In Buffer::map_async
    Buffer with 'bench-readback' label is invalid
; panic in main/panic: panic: panicked at <cargo-registry>/wgpu-30.0.1\src\backend\wgpu_core.rs:4036:26:
Error in Surface::get_current_texture_view: Validation Error

Caused by:
  Surface is not configured for presentation
; no window was rendered; device removed after the probe: 0x887a0007 The GPU will not respond to more commands, most likely because some other application submitted invalid commands.
The calling application should re-create the device and continue.; exit code 5 (panic), expected 0 |
