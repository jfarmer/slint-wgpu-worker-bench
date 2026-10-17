# Summary: final-macos-metal-default

Overall: PASS

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
| ordinary | abandon-after-device | yes | - |
| ordinary | abandon-before-adapter | yes | - |
| ordinary | abandon-before-device | yes | - |
| ordinary | failure-empty-mask | yes | - |
| ordinary | failure-limits | yes | - |
| ordinary | failure-unsupported-mask | yes | - |
| ordinary | multi-window | yes | - |
| ordinary | repeated | yes | - |
| ordinary | single | yes | - |
| worker-after-select | abandon-after-device | yes | - |
| worker-after-select | abandon-before-adapter | yes | - |
| worker-after-select | abandon-before-device | yes | - |
| worker-after-select | failure-empty-mask | yes | - |
| worker-after-select | failure-limits | yes | - |
| worker-after-select | failure-unsupported-mask | yes | - |
| worker-after-select | multi-window | yes | - |
| worker-after-select | repeated | yes | - |
| worker-after-select | single | yes | - |
| worker-device | abandon-after-device | yes | - |
| worker-device | abandon-before-adapter | yes | - |
| worker-device | abandon-before-device | yes | - |
| worker-device | failure-empty-mask | yes | - |
| worker-device | failure-limits | yes | - |
| worker-device | failure-unsupported-mask | yes | - |
| worker-device | multi-window | yes | - |
| worker-device | repeated | yes | - |
| worker-device | single | yes | - |
| worker-instance | abandon-after-device | yes | - |
| worker-instance | abandon-before-adapter | yes | - |
| worker-instance | abandon-before-device | yes | - |
| worker-instance | failure-empty-mask | yes | - |
| worker-instance | failure-limits | yes | - |
| worker-instance | failure-unsupported-mask | yes | - |
| worker-instance | multi-window | yes | - |
| worker-instance | repeated | yes | - |
| worker-instance | single | yes | - |
