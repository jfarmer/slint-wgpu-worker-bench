@echo off
rem Usage: unit-tests.cmd <slint-checkout>
setlocal
if "%~1"=="" exit /b 2
set "PATH=%USERPROFILE%\.cargo\bin;C:\Program Files\Git\cmd;%PATH%"
set "OUT=%~dp0followup-unit-tests.log"
cd /d "%~1" || exit /b 2
echo head: > "%OUT%"
git rev-parse HEAD >> "%OUT%"
git status --short >> "%OUT%"
cargo test -p i-slint-backend-winit --no-default-features --features renderer-femtovg-wgpu --lib --locked >> "%OUT%" 2>&1
set "CODE=%ERRORLEVEL%"
echo cargo_exit=%CODE% >> "%OUT%"
exit /b %CODE%
