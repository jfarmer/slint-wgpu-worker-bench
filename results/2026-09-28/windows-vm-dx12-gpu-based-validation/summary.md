# Summary: windows-vm-dx12-gpu-based-validation

Overall: PASS

## Comparisons Against Ordinary

| Variant | Strategy | Verdict | Differences |
| --- | --- | --- | --- |
| failure-limits | worker-device | match | - |
| failure-limits | worker-instance | match | - |
| failure-limits | worker-after-select | match | - |
| multi-window | worker-device | match | - |
| multi-window | worker-instance | match | - |
| multi-window | worker-after-select | match | - |
| single | worker-device | match | - |
| single | worker-instance | match | - |
| single | worker-after-select | match | - |

## Per-Case Sanity

| Strategy | Variant | OK | Problems |
| --- | --- | --- | --- |
| ordinary | failure-limits | yes | - |
| ordinary | multi-window | yes | - |
| ordinary | single | yes | - |
| worker-after-select | failure-limits | yes | - |
| worker-after-select | multi-window | yes | - |
| worker-after-select | single | yes | - |
| worker-device | failure-limits | yes | - |
| worker-device | multi-window | yes | - |
| worker-device | single | yes | - |
| worker-instance | failure-limits | yes | - |
| worker-instance | multi-window | yes | - |
| worker-instance | single | yes | - |
