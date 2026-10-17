# Summary: windows-vm-default-probe

Overall: PASS

## Comparisons Against Ordinary

| Variant | Strategy | Verdict | Differences |
| --- | --- | --- | --- |
| probe-after-adapter | worker-device | match | - |
| probe-after-adapter | worker-instance | match | - |
| probe-after-adapter | worker-after-select | match | - |
| probe-after-device | worker-device | match | - |
| probe-after-device | worker-instance | match | - |
| probe-after-device | worker-after-select | match | - |
| probe-after-first-device | worker-device | match | - |
| probe-after-first-device | worker-instance | match | - |
| probe-after-first-device | worker-after-select | match | - |
| probe-after-instance | worker-device | match | - |
| probe-after-instance | worker-instance | match | - |
| probe-after-instance | worker-after-select | match | - |
| probe-concurrent | worker-device | match | - |
| probe-concurrent | worker-instance | match | - |
| probe-concurrent | worker-after-select | match | - |
| probe-reject-drop | worker-device | match | - |
| probe-reject-drop | worker-instance | match | - |
| probe-reject-drop | worker-after-select | match | - |
| probe-reject-keep | worker-device | match | - |
| probe-reject-keep | worker-instance | match | - |
| probe-reject-keep | worker-after-select | match | - |

## Per-Case Sanity

| Strategy | Variant | OK | Problems |
| --- | --- | --- | --- |
| ordinary | probe-after-adapter | yes | - |
| ordinary | probe-after-device | yes | - |
| ordinary | probe-after-first-device | yes | - |
| ordinary | probe-after-instance | yes | - |
| ordinary | probe-concurrent | yes | - |
| ordinary | probe-reject-drop | yes | - |
| ordinary | probe-reject-keep | yes | - |
| worker-after-select | probe-after-adapter | yes | - |
| worker-after-select | probe-after-device | yes | - |
| worker-after-select | probe-after-first-device | yes | - |
| worker-after-select | probe-after-instance | yes | - |
| worker-after-select | probe-concurrent | yes | - |
| worker-after-select | probe-reject-drop | yes | - |
| worker-after-select | probe-reject-keep | yes | - |
| worker-device | probe-after-adapter | yes | - |
| worker-device | probe-after-device | yes | - |
| worker-device | probe-after-first-device | yes | - |
| worker-device | probe-after-instance | yes | - |
| worker-device | probe-concurrent | yes | - |
| worker-device | probe-reject-drop | yes | - |
| worker-device | probe-reject-keep | yes | - |
| worker-instance | probe-after-adapter | yes | - |
| worker-instance | probe-after-device | yes | - |
| worker-instance | probe-after-first-device | yes | - |
| worker-instance | probe-after-instance | yes | - |
| worker-instance | probe-concurrent | yes | - |
| worker-instance | probe-reject-drop | yes | - |
| worker-instance | probe-reject-keep | yes | - |
