# Runs scripts/run-matrix.py inside the logged-on user's interactive desktop session.
#
# An SSH or service session (session 0) can create windows, but DXGI rejects their swap chains,
# so Surface::configure fails. A scheduled task registered with /IT runs on the user's desktop.
#
# Usage (from any session, while the user is logged on):
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\windows\run-interactive.ps1 `
#       -Name win-dx12 -Extra "--env WGPU_BACKEND=dx12"
#
# Output: runs\<Name>\ (the matrix), runs\<Name>.console.log, runs\<Name>.exitcode.

param(
    [Parameter(Mandatory = $true)][string]$Name,
    [string]$Extra = "",
    [int]$TimeoutSeconds = 1800
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$runs = Join-Path $root "runs"
New-Item -ItemType Directory -Force -Path $runs | Out-Null
$cmdFile = Join-Path $runs "$Name.task.cmd"
$log = Join-Path $runs "$Name.console.log"
$exitFile = Join-Path $runs "$Name.exitcode"
Remove-Item -Force -ErrorAction SilentlyContinue $log, $exitFile

@"
@echo off
set PATH=%USERPROFILE%\.cargo\bin;C:\Program Files\Git\cmd;%PATH%
cd /d "$root"
py -3.12 scripts\run-matrix.py --out runs\$Name $Extra > "$log" 2>&1
> "$exitFile" echo %ERRORLEVEL%
"@ | Set-Content -Encoding ASCII $cmdFile

$task = "slint-wgpu-worker-bench-$Name"
schtasks /Create /TN $task /TR "cmd /c `"$cmdFile`"" /SC ONCE /ST 23:59 /IT /RU $env:USERNAME /F | Out-Null
schtasks /Run /TN $task | Out-Null

$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
while (-not (Test-Path $exitFile) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 2 }
schtasks /Delete /TN $task /F | Out-Null

if (-not (Test-Path $exitFile)) {
    Write-Output "timed out waiting for $exitFile"
    exit 124
}
# The task may still be writing the file when it first appears.
$text = $null
for ($i = 0; $i -lt 20 -and -not $text; $i++) {
    Start-Sleep -Milliseconds 250
    $text = Get-Content -Raw -ErrorAction SilentlyContinue $exitFile
}
Get-Content $log -Tail 5
$code = [int]("$text".Trim())
Write-Output "run-matrix exit code: $code"
exit $code
