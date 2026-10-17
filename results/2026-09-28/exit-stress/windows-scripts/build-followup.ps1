# Builds the probe app against a supplied Slint checkout, without trace patches.
# Output: slint-integration\build-followup.log and target-followup\release\slint-dx12-probe.exe.
param([Parameter(Mandatory = $true)][string]$SlintPath)
$ErrorActionPreference = "Stop"
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Program Files\Git\cmd;$env:PATH"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$tree = (Resolve-Path $SlintPath).Path
$app = "$root\app-followup"
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $app
Copy-Item -Recurse "$root\app" $app
(Get-Content "$app\Cargo.toml") -replace "SLINT_PATH", $tree.Replace("\", "/") | Set-Content -Encoding ASCII "$app\Cargo.toml"
Copy-Item "$tree\Cargo.lock" "$app\Cargo.lock"
$log = "$root\build-followup.log"
"tree=$tree head=$(git -C $tree rev-parse HEAD)" | Set-Content -Encoding ASCII $log
git -C $tree status --short | Add-Content -Encoding ASCII $log
Push-Location $app
cmd /c "cargo build --release --target-dir `"$root\target-followup`" >> `"$log`" 2>&1"
$code = $LASTEXITCODE
Pop-Location
"build_exit=$code" | Add-Content -Encoding ASCII $log
