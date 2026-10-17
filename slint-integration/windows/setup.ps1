# Prepares baseline and candidate probe builds against a supplied Slint checkout.
# The checkout must contain the experimental revisions documented in docs/PR.md.
# Builds and copied app manifests are written under slint-integration/.
param(
    [Parameter(Mandatory = $true)][string]$SlintPath,
    [string]$BaselinePath = "$SlintPath-baseline",
    [string[]]$Variants = @("baseline", "candidate")
)
$ErrorActionPreference = "Stop"
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Program Files\Git\cmd;$env:PATH"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$SlintPath = (Resolve-Path $SlintPath).Path
$BaselinePath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($BaselinePath)
$trees = @{ baseline = $BaselinePath; candidate = $SlintPath; fixed = $SlintPath }
$patches = @{ candidate = "trace.patch"; fixed = "trace-fixed.patch" }

if (-not (Test-Path $trees.baseline)) {
    git -C $trees.candidate worktree add --detach $trees.baseline 97066a9e8
}
foreach ($variant in $Variants) {
    $tree = $trees[$variant]
    if ($patches.ContainsKey($variant)) {
        git -C $tree checkout -- internal/backends/winit/wgpu_device.rs
        git -C $tree apply -p1 "$root\$($patches[$variant])"
    }
    $app = "$root\app-$variant"
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $app
    Copy-Item -Recurse "$root\app" $app
    $slintPath = $tree.Replace("\", "/")
    (Get-Content "$app\Cargo.toml") -replace "SLINT_PATH", $slintPath | Set-Content -Encoding ASCII "$app\Cargo.toml"
    Copy-Item "$tree\Cargo.lock" "$app\Cargo.lock"
    $log = "$root\build-$variant.log"
    "tree=$tree head=$(git -C $tree rev-parse HEAD)" | Set-Content -Encoding ASCII $log
    git -C $tree status --short | Add-Content -Encoding ASCII $log
    Push-Location $app
    cmd /c "cargo build --release --target-dir `"$root\target-$variant`" >> `"$log`" 2>&1"
    $code = $LASTEXITCODE
    Pop-Location
    "build_exit=$code" | Add-Content -Encoding ASCII $log
}
