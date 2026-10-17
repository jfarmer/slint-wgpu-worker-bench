@echo off
rem Runs every case for each variant given as an argument, three times each.
set "ROOT=%~dp0.."
if not exist "%ROOT%\out" mkdir "%ROOT%\out"
for %%V in (%*) do (
  for %%R in (1 2 3) do (
    call "%~dp0run-case.cmd" %%V a-dx12-cpu-r%%R single "WGPU_BACKEND=dx12" "SLINT_WGPU_CPU=1"
    call "%~dp0run-case.cmd" %%V b-default-cpu-r%%R single "SLINT_WGPU_CPU=1"
    call "%~dp0run-case.cmd" %%V c-dx12-nocpu-r%%R single "WGPU_BACKEND=dx12"
    call "%~dp0run-case.cmd" %%V d-dx12-cpu-validation-r%%R single "WGPU_BACKEND=dx12" "SLINT_WGPU_CPU=1" "WGPU_VALIDATION=1"
    call "%~dp0run-case.cmd" %%V e-second-window-r%%R second-window "WGPU_BACKEND=dx12" "SLINT_WGPU_CPU=1"
    call "%~dp0run-case.cmd" %%V f-quit-early-r%%R quit-early "WGPU_BACKEND=dx12" "SLINT_WGPU_CPU=1"
    call "%~dp0run-case.cmd" %%V g-exit-immediately-r%%R exit-immediately "WGPU_BACKEND=dx12" "SLINT_WGPU_CPU=1"
  )
)
