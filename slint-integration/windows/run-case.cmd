@echo off
rem Runs one probe case in the interactive session. Arguments: variant case scenario env-assignments...
rem Output: slint-integration\out\<variant>__<case>.log with the exit code appended.
setlocal
set "ROOT=%~dp0.."
set VARIANT=%1
set CASE=%2
set SCENARIO=%3
shift & shift & shift
set WGPU_BACKEND=
set SLINT_WGPU_CPU=
set WGPU_VALIDATION=
:envloop
if "%~1"=="" goto run
set "%~1"
shift
goto envloop
:run
set OUT=%ROOT%\out\%VARIANT%__%CASE%.log
echo variant=%VARIANT% case=%CASE% scenario=%SCENARIO% > "%OUT%"
"%ROOT%\target-%VARIANT%\release\slint-dx12-probe.exe" %SCENARIO% >> "%OUT%" 2>&1
echo exit_code=%ERRORLEVEL% >> "%OUT%"
endlocal
