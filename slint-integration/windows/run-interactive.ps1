# Runs cases.cmd for the given variants in the logged-on user's interactive session.
# Follows scripts\windows\run-interactive.ps1: session 0 can't configure DXGI swap chains.
param([Parameter(Mandatory = $true)][string]$Name, [string]$Variants = "baseline candidate", [int]$TimeoutSeconds = 1800)
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$done = "$root\$Name.done"
Remove-Item -Force -ErrorAction SilentlyContinue $done
$cmdFile = "$root\$Name.task.cmd"
"@echo off`r`ncall `"$PSScriptRoot\cases.cmd`" $Variants > `"$root\$Name.console.log`" 2>&1`r`necho done> `"$done`"" | Set-Content -Encoding ASCII $cmdFile
$task = "slint-dx12-integration-$Name"
schtasks /Create /TN $task /TR "cmd /c `"$cmdFile`"" /SC ONCE /ST 23:59 /IT /RU $env:USERNAME /F | Out-Null
schtasks /Run /TN $task | Out-Null
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
while (-not (Test-Path $done) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 2 }
schtasks /Delete /TN $task /F | Out-Null
if (-not (Test-Path $done)) { Write-Output "timed out"; exit 124 }
Write-Output "done"
