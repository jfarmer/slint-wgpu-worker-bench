# Runs stress-all.cmd in the logged-on user's interactive session through a one-shot scheduled
# task, as run-interactive.ps1 does, then exports Application log errors since the start.
param([int]$TimeoutSeconds = 3600)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
New-Item -ItemType Directory -Force "$root\stress" | Out-Null
$done = "$root\stress\stress.done"
Remove-Item -Force -ErrorAction SilentlyContinue $done
$cmdFile = "$root\stress\stress.task.cmd"
"@echo off`r`ncall `"$PSScriptRoot\stress-all.cmd`" > `"$root\stress\console.log`" 2>&1`r`necho done> `"$done`"" | Set-Content -Encoding ASCII $cmdFile
$task = "slint-exit-stress"
schtasks /Create /TN $task /TR "cmd /c `"$cmdFile`"" /SC ONCE /ST 23:59 /IT /RU $env:USERNAME /F | Out-Null
schtasks /Run /TN $task | Out-Null
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
while (-not (Test-Path $done) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 5 }
schtasks /Delete /TN $task /F | Out-Null
$start = [datetime]::Parse((Get-Content "$root\stress\start-time.txt" -Raw).Trim())
$events = Get-WinEvent -FilterHashtable @{ LogName = "Application"; Level = 1, 2; StartTime = $start } -ErrorAction SilentlyContinue
$events | Format-List TimeCreated, ProviderName, Id, Message | Out-File -Encoding ASCII "$root\stress\application-errors.txt"
"application_errors=$(@($events).Count)" | Out-File -Encoding ASCII -Append "$root\stress\application-errors.txt"
if (-not (Test-Path $done)) { Write-Output "timed out"; exit 124 }
Write-Output "done"
