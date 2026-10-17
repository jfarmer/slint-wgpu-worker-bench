# Summary: win-ssh-session0

Overall: FAIL

## Comparisons Against Ordinary

| Variant | Strategy | Verdict | Differences |
| --- | --- | --- | --- |
| abandon-after-device | worker-device | match | - |
| abandon-after-device | worker-instance | match | - |
| abandon-after-device | worker-after-select | match | - |
| abandon-before-adapter | worker-device | match | - |
| abandon-before-adapter | worker-instance | match | - |
| abandon-before-adapter | worker-after-select | match | - |
| abandon-before-device | worker-device | match | - |
| abandon-before-device | worker-instance | match | - |
| abandon-before-device | worker-after-select | match | - |
| failure-empty-mask | worker-device | match | - |
| failure-empty-mask | worker-instance | match | - |
| failure-empty-mask | worker-after-select | match | - |
| failure-limits | worker-device | match | - |
| failure-limits | worker-instance | match | - |
| failure-limits | worker-after-select | match | - |
| failure-unsupported-mask | worker-device | known-difference | - (known: error_classes) |
| failure-unsupported-mask | worker-instance | match | - |
| failure-unsupported-mask | worker-after-select | match | - |
| multi-window | worker-device | match | - |
| multi-window | worker-instance | match | - |
| multi-window | worker-after-select | match | - |
| repeated | worker-device | match | - |
| repeated | worker-instance | match | - |
| repeated | worker-after-select | match | - |
| single | worker-device | match | - |
| single | worker-instance | match | - |
| single | worker-after-select | match | - |

## Per-Case Sanity

| Strategy | Variant | OK | Problems |
| --- | --- | --- | --- |
| ordinary | abandon-after-device | no | post-abandon: surface configure was rejected, nothing was presented |
| ordinary | abandon-before-adapter | no | post-abandon: surface configure was rejected, nothing was presented |
| ordinary | abandon-before-device | no | post-abandon: surface configure was rejected, nothing was presented |
| ordinary | failure-empty-mask | no | status is 'watchdog-timeout'; timeout in /watchdog: timed out after 60s during main/event_loop on main |
| ordinary | failure-limits | yes | - |
| ordinary | failure-unsupported-mask | yes | - |
| ordinary | multi-window | no | seq0: surface configure was rejected, nothing was presented; seq1: surface configure was rejected, nothing was presented; sim0: surface configure was rejected, nothing was presented; sim1: surface configure was rejected, nothing was presented |
| ordinary | repeated | no | r0: surface configure was rejected, nothing was presented; r1: surface configure was rejected, nothing was presented; r2: surface configure was rejected, nothing was presented |
| ordinary | single | no | w0: surface configure was rejected, nothing was presented |
| worker-after-select | abandon-after-device | no | post-abandon: surface configure was rejected, nothing was presented |
| worker-after-select | abandon-before-adapter | no | post-abandon: surface configure was rejected, nothing was presented |
| worker-after-select | abandon-before-device | no | post-abandon: surface configure was rejected, nothing was presented |
| worker-after-select | failure-empty-mask | no | status is 'watchdog-timeout'; timeout in /watchdog: timed out after 60s during main/event_loop on main |
| worker-after-select | failure-limits | yes | - |
| worker-after-select | failure-unsupported-mask | yes | - |
| worker-after-select | multi-window | no | seq0: surface configure was rejected, nothing was presented; seq1: surface configure was rejected, nothing was presented; sim0: surface configure was rejected, nothing was presented; sim1: surface configure was rejected, nothing was presented |
| worker-after-select | repeated | no | r0: surface configure was rejected, nothing was presented; r1: surface configure was rejected, nothing was presented; r2: surface configure was rejected, nothing was presented |
| worker-after-select | single | no | w0: surface configure was rejected, nothing was presented |
| worker-device | abandon-after-device | no | post-abandon: surface configure was rejected, nothing was presented |
| worker-device | abandon-before-adapter | no | post-abandon: surface configure was rejected, nothing was presented |
| worker-device | abandon-before-device | no | post-abandon: surface configure was rejected, nothing was presented |
| worker-device | failure-empty-mask | no | status is 'watchdog-timeout'; timeout in /watchdog: timed out after 60s during main/event_loop on main |
| worker-device | failure-limits | yes | - |
| worker-device | failure-unsupported-mask | yes | - |
| worker-device | multi-window | no | seq0: surface configure was rejected, nothing was presented; seq1: surface configure was rejected, nothing was presented; sim0: surface configure was rejected, nothing was presented; sim1: surface configure was rejected, nothing was presented |
| worker-device | repeated | no | r0: surface configure was rejected, nothing was presented; r1: surface configure was rejected, nothing was presented; r2: surface configure was rejected, nothing was presented |
| worker-device | single | no | w0: surface configure was rejected, nothing was presented |
| worker-instance | abandon-after-device | no | post-abandon: surface configure was rejected, nothing was presented |
| worker-instance | abandon-before-adapter | no | post-abandon: surface configure was rejected, nothing was presented |
| worker-instance | abandon-before-device | no | post-abandon: surface configure was rejected, nothing was presented |
| worker-instance | failure-empty-mask | no | status is 'watchdog-timeout'; timeout in /watchdog: timed out after 60s during main/event_loop on main |
| worker-instance | failure-limits | yes | - |
| worker-instance | failure-unsupported-mask | yes | - |
| worker-instance | multi-window | no | seq0: surface configure was rejected, nothing was presented; seq1: surface configure was rejected, nothing was presented; sim0: surface configure was rejected, nothing was presented; sim1: surface configure was rejected, nothing was presented |
| worker-instance | repeated | no | r0: surface configure was rejected, nothing was presented; r1: surface configure was rejected, nothing was presented; r2: surface configure was rejected, nothing was presented |
| worker-instance | single | no | w0: surface configure was rejected, nothing was presented |
