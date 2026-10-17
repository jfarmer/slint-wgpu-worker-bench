# Exit Stress for the Detached Device Worker

These runs check what happens at process exit when a Slint backend exists but no window ever resumed.
Since the thread-local fix, `PendingWgpuDevice::drop` releases the worker's claim without joining.
The worker then keeps running while the process exits.
The runs look for aborts and crash reports from that teardown on macOS and Windows.

Run date: 2026-09-28.
This is a behavioral check, not a timing campaign.
The public archive keeps exit codes, histograms, classifications and the reproduction scripts.
Raw run logs, command transcripts, crash reports and Windows event exports are excluded.
Sets ran one after another with no other workload on the Mac, and the Windows sets ran after the macOS sets finished.

Claim labels follow `docs/slint-windows-integration.md`: **Experiment**, **Source-established**, **Documented**, **Hypothesis**.

## Revisions

Slint worktree `<slint-worktree>`.

| Name here | Commit | Content |
| --- | --- | --- |
| `fixes` | `b1dd5ac4f`, tip of `wgpu-worker-preparation-fixes` | `97066a9e8` plus Metal-only versions of the probe release, the thread-local fix and the non-joining discard |
| `followup` | `11fe95b24`, tip of `wgpu-worker-dx12-followup` | `fixes` plus the DX12 gate, same tree as `wgpu-worker-dx12` (`ab27d194a`) |
| `control` (macOS) | `97066a9e8`, `wgpu-worker-preparation` | the Metal worker that joins in `Drop` |
| `control` (Windows) | `adde532c9` plus `slint-integration/trace.patch` | the existing `target-candidate` build from `slint-windows-run1`, DX12 gate, joins in `Drop` |
| `traced` | `fixes` plus `trace-metal.patch` | trace lines, and an optional random delay before the worker prepares |

`trace-metal.patch` prints `slint-wgpu-trace:` lines when the worker spawns, starts preparing, finishes, and destroys an unclaimed result, and when `discard` releases the claim.
With `SLINT_TRACE_WORKER_DELAY_MAX_MS` set, the worker sleeps a pseudo-random delay between `SLINT_TRACE_WORKER_DELAY_MIN_MS` (default 0) and the maximum before preparing.
The minimum was added after the first delay set ran.
With the minimum unset the code computes the same delay as the build that ran that set.

## Probe App Scenarios

The app is `slint-integration/app`.
This change adds three scenarios to it, next to the existing `exit-immediately`.

| Scenario | What `main` does |
| --- | --- |
| `exit-immediately` | creates a `ProbeWindow` component and returns, as case g in `docs/slint-windows-integration.md` |
| `exit-after-backend` | calls `slint::BackendSelector::new().select()` and returns, so no component is compiled |
| `process-exit-after-backend` | the same, then calls `std::process::exit(0)` |
| `exit-after-prepared` | creates a component, sleeps one second so the worker has finished, and returns |

`exit-after-backend` was meant as the case where exit lands while the worker still prepares.
On this Mac the trace shows it doesn't: the Metal device is ready before `select()` returns (see Findings).
The `traced` delay sets are the ones that exit during preparation.

## Commands

macOS, Apple Silicon, default `WGPU_BACKEND` (Metal).
Builds live in the session scratchpad, outside this repository.
Each app copy has `SLINT_PATH` replaced by a worktree at the listed commit and that worktree's `Cargo.lock`.

```sh
# in each app copy: app-fixes, app-control, app-traced
cargo build --release --target-dir <build-dir>/target-<name>
# debug builds, fixes and control only
cargo build --target-dir <build-dir>/target-<name>
./run-macos-1.sh <build-dir>   # release sets
./run-macos-2.sh <build-dir>   # focused delay set and debug sets
./classify-trace.sh <set-dir>  # for the traced sets
```

`run-macos-set.sh` runs one binary 200 times and writes, per set, `command.txt`, `runs.log`, `exit-codes.txt`, `histogram.txt`, and `crash-reports/` with any new `slint-dx12-probe` report from `~/Library/Logs/DiagnosticReports`.

Windows 11 ARM64 VM, WARP only, `WGPU_BACKEND=dx12 SLINT_WGPU_CPU=1`, in the interactive session.
Copy the scripts from `windows-scripts/` into `slint-integration/windows/` before using the commands below.
Supply the Slint checkout containing the experimental follow-up as `-SlintPath`.

```bat
powershell -NoProfile -ExecutionPolicy Bypass -File .\slint-integration\windows\build-followup.ps1 -SlintPath C:\src\slint
powershell -NoProfile -ExecutionPolicy Bypass -File .\slint-integration\windows\stress-interactive.ps1
```

`stress-interactive.ps1` runs `stress-all.cmd` through a one-shot `schtasks /IT` task, as `slint-integration/windows/run-interactive.ps1` does.
Afterwards it exports every Application log event of level Critical or Error since the start to `windows/application-errors.txt`.
The follow-up app was first compiled at `278f1b15d`.
That commit has the same tree as `11fe95b24` and was replaced by it when a commit message was reworded.
`windows/build-followup.log` is the no-op rebuild at `11fe95b24`.

## Results

### macOS

| Set | Build | Scenario | Exit codes | Crash reports |
| --- | --- | --- | --- | --- |
| `macos-fixes-exit-immediately` | release `fixes` | `exit-immediately` | 0: 200 | 0 |
| `macos-control-97066a9e8-exit-immediately` | release `control` | `exit-immediately` | 0: 200 | 0 |
| `macos-fixes-exit-after-backend` | release `fixes` | `exit-after-backend` | 0: 200 | 0 |
| `macos-control-97066a9e8-exit-after-backend` | release `control` | `exit-after-backend` | 0: 200 | 0 |
| `macos-fixes-process-exit-after-backend` | release `fixes` | `process-exit-after-backend` | 0: 200 | 0 |
| `macos-fixes-exit-after-prepared` | release `fixes` | `exit-after-prepared` | 0: 200 | 0 |
| `macos-traced-exit-immediately` | release `traced` | `exit-immediately` | 0: 200 | 0 |
| `macos-traced-exit-after-backend-delay` | release `traced`, delay 0 to 80 ms | `exit-after-backend` | 0: 200 | 0 |
| `macos-traced-exit-after-backend-delay-50-70` | release `traced`, delay 50 to 70 ms | `exit-after-backend` | 0: 200 | 0 |
| `macos-debug-control-97066a9e8-exit-immediately` | debug `control` | `exit-immediately` | **134: 200** | 28 |
| `macos-debug-fixes-exit-immediately` | debug `fixes` | `exit-immediately` | 0: 200 | 0 |
| `macos-debug-control-97066a9e8-exit-after-backend` | debug `control` | `exit-after-backend` | 0: 200 | 0 |
| `macos-debug-fixes-exit-after-backend` | debug `fixes` | `exit-after-backend` | 0: 200 | 0 |

Worker state when `main` released the claim, from `classification.txt` in each traced set:

| Set | Finished before release | Exit during preparation | Exit during the injected delay | `destroyed unclaimed result` printed |
| --- | --- | --- | --- | --- |
| `macos-traced-exit-immediately` | 200 | 0 | 0 | 3 |
| `macos-traced-exit-after-backend-delay` | 154 | 18 | 28 | 2 |
| `macos-traced-exit-after-backend-delay-50-70` | 36 | 90 | 74 | 0 |

Every traced run printed `released`, so `Drop` ran in all 600.

### Windows

| Set | Build | Scenario | Exit codes | Application errors |
| --- | --- | --- | --- | --- |
| `windows-followup-exit-immediately` | release `followup` | `exit-immediately` | 0: 200 | 0 |
| `windows-followup-exit-after-backend` | release `followup` | `exit-after-backend` | 0: 200 | 0 |
| `windows-control-adde532c9-exit-immediately` | release `control` | `exit-immediately` | **-1073740791 (0xC0000409): 50** | 50 |

All 50 Application errors are event 1000 from `target-candidate\release\slint-dx12-probe.exe` with exception code `0xc0000409`.
Each control run printed `worker spawned backends=Backends(DX12)`, `discarded`, and the panic `threads should not terminate unexpectedly` from `std\src\thread\lifecycle.rs`.
The follow-up runs printed nothing besides the app's `event=` lines.

`windows/windows-unit-tests-11fe95b24.log`: the 16 Winit lib tests pass on the VM at `11fe95b24`.

## Findings

**Experiment.** No `fixes` or `followup` run aborted or left a crash report or Application error.
That covers 1,800 macOS runs (800 release, 600 traced, 400 debug) and 400 Windows runs.

**Experiment.** 108 traced macOS runs exited while the worker was still creating the Metal device, and 102 while it slept before preparing.
All exited with 0.
So the process exit tolerated a worker in the middle of `prepare_default_metal_device` on this machine.

**Experiment.** The released worker almost never finishes destroying its result before the process ends.
Only 5 of 600 traced runs printed `worker destroyed unclaimed result`.
In the other runs, `exit` ended the worker before or during that destruction, and the exit code was still 0.

**Experiment.** Without an injected delay the worker finished before `main` released it in all 200 traced `exit-immediately` runs.
In the untraced `exit-after-backend` runs, `select()` returns after the Metal device is ready.
So `exit-after-backend` doesn't exercise an exit during preparation on this Mac, which is why the delay sets exist.

**Experiment.** The macOS abort at `97066a9e8` happens only in debug builds.
The release control exited with 0 in all 400 runs.
The debug control aborted with 134 in 200 of 200 `exit-immediately` runs, each after two `cannot access a Thread Local Storage value during or after destruction` panics.
The debug `fixes` build exited with 0 in all 400 runs.

**Source-established.** objc2 0.5.2 and 0.6.4 record each autorelease pool in the thread-local `POOLS` only under `cfg(all(debug_assertions, not(feature = "unstable-autoreleasesafe")))` (`src/rc/autorelease.rs`).
The probe app's release profile has no debug assertions, so the join inside `autoreleasepool` doesn't touch a destroyed thread-local there.
The known-bad control therefore shows 134 only in the debug build.
The Metal branch commit message says so.

**Experiment.** The debug control didn't abort in `exit-after-backend` (0 in 200 of 200), although both scenarios keep the backend in the same `GLOBAL_CONTEXT` thread-local.

**Hypothesis.** Thread-local destructor order differs between the two scenarios.
If the main thread first uses `POOLS` after `GLOBAL_CONTEXT` is registered, `POOLS` is destroyed first and the join panics.
If it doesn't, the join lazily creates `POOLS` again.
This wasn't checked.

**Experiment.** The Windows abort at `adde532c9` reproduces in 50 of 50 release runs, with the panic that `docs/slint-windows-integration.md` describes.
The follow-up exits with 0 in 200 of 200 runs of the same scenario.

**Experiment.** macOS ReportCrash wrote 28 reports for the 200 debug-control aborts, all `EXC_CRASH` / `SIGABRT`.
The exit codes, not the report count, are the abort count.

## Limits

- The Windows VM has only WARP.
  With `SLINT_WGPU_CPU=1` the probe skips the adapter probe, and the untraced follow-up build can't show where the worker was at exit.
- The injected delay moves the start of preparation, not its duration.
  The Metal driver's own teardown ran at whatever point `exit` reached it.
- One Mac, one VM.
  An exit that races the worker on other hardware can land at points these runs didn't reach.
