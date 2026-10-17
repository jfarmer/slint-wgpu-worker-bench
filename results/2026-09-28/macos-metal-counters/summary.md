# Summary: macos-metal-counters

Overall: PASS

## Comparisons Against Ordinary

| Variant | Strategy | Verdict | Differences |
| --- | --- | --- | --- |
| early-exit-in-device | worker-device | match | - |
| early-exit-in-device | worker-instance | match | - |
| early-exit-in-device | worker-after-select | match | - |
| join-block-after-device-after-window | worker-device | match | - |
| join-block-after-device-after-window | worker-instance | match | - |
| join-block-after-device-after-window | worker-after-select | match | - |
| probe-after-device | worker-device | match | - |
| probe-after-device | worker-instance | match | - |
| probe-after-device | worker-after-select | match | - |
| single | worker-device | match | - |
| single | worker-instance | match | - |
| single | worker-after-select | match | - |

## Per-Case Sanity

| Strategy | Variant | OK | Problems |
| --- | --- | --- | --- |
| ordinary | early-exit-in-device | yes | - |
| ordinary | join-block-after-device-after-window | yes | - |
| ordinary | probe-after-device | yes | - |
| ordinary | single | yes | - |
| worker-after-select | early-exit-in-device | yes | - |
| worker-after-select | join-block-after-device-after-window | yes | - |
| worker-after-select | probe-after-device | yes | - |
| worker-after-select | single | yes | - |
| worker-device | early-exit-in-device | yes | - |
| worker-device | join-block-after-device-after-window | yes | - |
| worker-device | probe-after-device | yes | - |
| worker-device | single | yes | - |
| worker-instance | early-exit-in-device | yes | - |
| worker-instance | join-block-after-device-after-window | yes | - |
| worker-instance | probe-after-device | yes | - |
| worker-instance | single | yes | - |
