@echo off
rem Runs one probe build N times with WGPU_BACKEND=dx12 SLINT_WGPU_CPU=1 and records exit codes.
rem Arguments: variant scenario count set-name. Output: slint-integration\stress\<set-name>\.
setlocal EnableDelayedExpansion
set "ROOT=%~dp0.."
set BIN=%ROOT%\target-%1\release\slint-dx12-probe.exe
set OUT=%ROOT%\stress\%4
if not exist "%OUT%" mkdir "%OUT%"
set WGPU_BACKEND=dx12
set SLINT_WGPU_CPU=1
set WGPU_VALIDATION=
echo binary=%BIN% scenario=%2 count=%3 env=WGPU_BACKEND=dx12 SLINT_WGPU_CPU=1 > "%OUT%\command.txt"
type nul > "%OUT%\runs.log"
type nul > "%OUT%\exit-codes.txt"
for /L %%I in (1,1,%3) do (
  echo === run %%I >> "%OUT%\runs.log"
  "%BIN%" %2 >> "%OUT%\runs.log" 2>&1
  echo exit_code=!ERRORLEVEL! >> "%OUT%\runs.log"
  echo !ERRORLEVEL!>> "%OUT%\exit-codes.txt"
)
endlocal
