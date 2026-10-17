#!/bin/sh
# Runs one probe binary N times sequentially and records each run's output and exit code.
# Usage: run-macos-set.sh <out-dir> <binary> <scenario> <count> [VAR=value ...]
# Output: <out-dir>/runs.log (all runs, delimited), exit-codes.txt, histogram.txt,
# and crash-reports/ with any new slint-dx12-probe crash reports.
set -u
out=$1 bin=$2 scenario=$3 count=$4
shift 4
reports="$HOME/Library/Logs/DiagnosticReports"
mkdir -p "$out/crash-reports"
: > "$out/runs.log"
: > "$out/exit-codes.txt"
ls "$reports" 2>/dev/null | grep '^slint-dx12-probe' | sort > "$out/.reports-before"
echo "binary=$bin scenario=$scenario count=$count env=$*" > "$out/command.txt"
i=1
while [ "$i" -le "$count" ]; do
  echo "=== run $i" >> "$out/runs.log"
  env "$@" "$bin" "$scenario" >> "$out/runs.log" 2>&1
  code=$?
  echo "exit_code=$code" >> "$out/runs.log"
  echo "$code" >> "$out/exit-codes.txt"
  i=$((i + 1))
done
# ReportCrash writes reports asynchronously.
sleep 5
ls "$reports" 2>/dev/null | grep '^slint-dx12-probe' | sort > "$out/.reports-after"
comm -13 "$out/.reports-before" "$out/.reports-after" | while read -r f; do cp "$reports/$f" "$out/crash-reports/"; done
rm -f "$out/.reports-before" "$out/.reports-after"
sort -n "$out/exit-codes.txt" | uniq -c | awk '{print "exit " $2 ": " $1}' > "$out/histogram.txt"
echo "crash_reports=$(ls "$out/crash-reports" | wc -l | tr -d ' ')" >> "$out/histogram.txt"
cat "$out/histogram.txt"
