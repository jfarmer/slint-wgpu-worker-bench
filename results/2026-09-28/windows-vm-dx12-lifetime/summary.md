# Summary: windows-vm-dx12-lifetime

Overall: PASS

## Comparisons Against Ordinary

| Variant | Strategy | Verdict | Differences |
| --- | --- | --- | --- |
| early-exit-in-adapter | worker-device | match | - |
| early-exit-in-adapter | worker-instance | match | - |
| early-exit-in-adapter | worker-after-select | match | - |
| early-exit-in-device | worker-device | match | - |
| early-exit-in-device | worker-instance | match | - |
| early-exit-in-device | worker-after-select | match | - |
| early-exit-in-instance | worker-device | match | - |
| early-exit-in-instance | worker-instance | match | - |
| early-exit-in-instance | worker-after-select | match | - |
| join-block-after-device-after-window | worker-device | match | - |
| join-block-after-device-after-window | worker-instance | match | - |
| join-block-after-device-after-window | worker-after-select | match | - |
| join-block-after-device-before-loop | worker-device | match | - |
| join-block-after-device-before-loop | worker-instance | match | - |
| join-block-after-device-before-loop | worker-after-select | match | - |
| join-block-before-adapter-after-window | worker-device | match | - |
| join-block-before-adapter-after-window | worker-instance | match | - |
| join-block-before-adapter-after-window | worker-after-select | match | - |
| join-block-before-adapter-before-loop | worker-device | match | - |
| join-block-before-adapter-before-loop | worker-instance | match | - |
| join-block-before-adapter-before-loop | worker-after-select | match | - |
| join-block-before-device-after-window | worker-device | match | - |
| join-block-before-device-after-window | worker-instance | match | - |
| join-block-before-device-after-window | worker-after-select | match | - |
| join-block-before-device-before-loop | worker-device | match | - |
| join-block-before-device-before-loop | worker-instance | match | - |
| join-block-before-device-before-loop | worker-after-select | match | - |

## Per-Case Sanity

| Strategy | Variant | OK | Problems |
| --- | --- | --- | --- |
| ordinary | early-exit-in-adapter | yes | - |
| ordinary | early-exit-in-device | yes | - |
| ordinary | early-exit-in-instance | yes | - |
| ordinary | join-block-after-device-after-window | yes | - |
| ordinary | join-block-after-device-before-loop | yes | - |
| ordinary | join-block-before-adapter-after-window | yes | - |
| ordinary | join-block-before-adapter-before-loop | yes | - |
| ordinary | join-block-before-device-after-window | yes | - |
| ordinary | join-block-before-device-before-loop | yes | - |
| worker-after-select | early-exit-in-adapter | yes | - |
| worker-after-select | early-exit-in-device | yes | - |
| worker-after-select | early-exit-in-instance | yes | - |
| worker-after-select | join-block-after-device-after-window | yes | - |
| worker-after-select | join-block-after-device-before-loop | yes | - |
| worker-after-select | join-block-before-adapter-after-window | yes | - |
| worker-after-select | join-block-before-adapter-before-loop | yes | - |
| worker-after-select | join-block-before-device-after-window | yes | - |
| worker-after-select | join-block-before-device-before-loop | yes | - |
| worker-device | early-exit-in-adapter | yes | - |
| worker-device | early-exit-in-device | yes | - |
| worker-device | early-exit-in-instance | yes | - |
| worker-device | join-block-after-device-after-window | yes | - |
| worker-device | join-block-after-device-before-loop | yes | - |
| worker-device | join-block-before-adapter-after-window | yes | - |
| worker-device | join-block-before-adapter-before-loop | yes | - |
| worker-device | join-block-before-device-after-window | yes | - |
| worker-device | join-block-before-device-before-loop | yes | - |
| worker-instance | early-exit-in-adapter | yes | - |
| worker-instance | early-exit-in-device | yes | - |
| worker-instance | early-exit-in-instance | yes | - |
| worker-instance | join-block-after-device-after-window | yes | - |
| worker-instance | join-block-after-device-before-loop | yes | - |
| worker-instance | join-block-before-adapter-after-window | yes | - |
| worker-instance | join-block-before-adapter-before-loop | yes | - |
| worker-instance | join-block-before-device-after-window | yes | - |
| worker-instance | join-block-before-device-before-loop | yes | - |
