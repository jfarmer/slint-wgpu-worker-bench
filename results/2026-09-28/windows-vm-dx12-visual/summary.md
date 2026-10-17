# Summary: windows-vm-dx12-visual

Overall: PASS

## Comparisons Against Ordinary

| Variant | Strategy | Verdict | Differences |
| --- | --- | --- | --- |
| multi-window | worker-device | match | - |
| multi-window | worker-instance | match | - |
| multi-window | worker-after-select | match | - |
| single | worker-device | match | - |
| single | worker-instance | match | - |
| single | worker-after-select | match | - |

## Per-Case Sanity

| Strategy | Variant | OK | Problems |
| --- | --- | --- | --- |
| ordinary | multi-window | yes | - |
| ordinary | single | yes | - |
| worker-after-select | multi-window | yes | - |
| worker-after-select | single | yes | - |
| worker-device | multi-window | yes | - |
| worker-device | single | yes | - |
| worker-instance | multi-window | yes | - |
| worker-instance | single | yes | - |
