# Cross-Run Comparison

Overall: PASS

| Run | Environment | Variant | Strategy | Adapter | Order | DX12 gate | Compiler |
| --- | --- | --- | --- | --- | --- | --- | --- |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | failure-limits | ordinary | - | - | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | failure-limits | worker-after-select | - | - | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | failure-limits | worker-device | - | - | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | failure-limits | worker-instance | - | - | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | multi-window | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | multi-window | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | multi-window | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | multi-window | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | single | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | single | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | single | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-env-baseline | WGPU_BACKEND=dx12 | single | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | failure-limits | ordinary | - | - | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | failure-limits | worker-after-select | - | - | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | failure-limits | worker-device | - | - | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | failure-limits | worker-instance | - | - | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | multi-window | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | multi-window | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | multi-window | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | multi-window | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | single | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | single | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | single | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-low | WGPU_BACKEND=dx12 WGPU_POWER_PREF=low | single | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | failure-limits | ordinary | - | - | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | failure-limits | worker-after-select | - | - | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | failure-limits | worker-device | - | - | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | failure-limits | worker-instance | - | - | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | multi-window | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | multi-window | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | multi-window | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | multi-window | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | single | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | single | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | single | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-power-high | WGPU_BACKEND=dx12 WGPU_POWER_PREF=high | single | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | failure-limits | ordinary | - | - | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | failure-limits | worker-after-select | - | - | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | failure-limits | worker-device | - | - | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | failure-limits | worker-instance | - | - | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | multi-window | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | multi-window | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | multi-window | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | multi-window | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | single | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | single | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | single | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-wgpu-debug | WGPU_BACKEND=dx12 WGPU_DEBUG=1 | single | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | failure-limits | ordinary | - | - | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | failure-limits | worker-after-select | - | - | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | failure-limits | worker-device | - | - | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | failure-limits | worker-instance | - | - | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | multi-window | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | multi-window | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | multi-window | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | multi-window | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | single | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | single | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | single | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-gpu-based-validation | WGPU_BACKEND=dx12 WGPU_GPU_BASED_VALIDATION=1 | single | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | failure-limits | ordinary | - | - | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | failure-limits | worker-after-select | - | - | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | failure-limits | worker-device | - | - | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | failure-limits | worker-instance | - | - | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | multi-window | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | multi-window | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | multi-window | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | multi-window | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | single | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | single | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | single | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | False | fxc |
| windows-vm-dx12-adapter-name | WGPU_ADAPTER_NAME=basic WGPU_BACKEND=dx12 | single | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | False | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | failure-limits | ordinary | - | - | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | failure-limits | worker-after-select | - | - | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | failure-limits | worker-device | - | - | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | failure-limits | worker-instance | - | - | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | multi-window | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | multi-window | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | multi-window | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | multi-window | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | single | ordinary | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | single | worker-after-select | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | single | worker-device | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
| windows-vm-dx12-compiler-fxc | WGPU_BACKEND=dx12 WGPU_DX12_COMPILER=fxc | single | worker-instance | Microsoft Basic Render Driver (Dx12) | Microsoft Basic Render Driver (Dx12) | True | fxc |
