#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BASELINE_FILE="${SCRIPT_DIR}/perf_baseline.json"
CRITERION_DIR="${REPO_ROOT}/target/criterion"

MAX_ALLOWED_REGRESSION_PERCENT=10.0

echo "=== Duet CI Performance Gate (T-3.3.5) ==="

if [[ "${1:-}" == "--save-baseline" ]]; then
    echo "Running benchmark suite to generate baseline..."
    cargo bench --bench vfs_benchmarks
    
    python3 -c "
import json, os, glob

criterion_dir = '${CRITERION_DIR}'
baseline = {}

for estimates_path in glob.glob(os.path.join(criterion_dir, '**', 'new', 'estimates.json'), recursive=True):
    rel_path = os.path.relpath(estimates_path, criterion_dir)
    parts = rel_path.split(os.sep)
    bench_name = '/'.join(parts[:-2])
    with open(estimates_path) as f:
        data = json.load(f)
        mean_ns = data.get('mean', {}).get('point_estimate', 0)
        baseline[bench_name] = mean_ns

with open('${BASELINE_FILE}', 'w') as f:
    json.dump(baseline, f, indent=2)

print(f'Saved baseline with {len(baseline)} metrics to ${BASELINE_FILE}')
"
    exit 0
fi

echo "Running Criterion benchmarks for regression check..."
cargo bench --bench vfs_benchmarks

if [[ ! -f "${BASELINE_FILE}" ]]; then
    echo "Warning: No baseline file found at ${BASELINE_FILE}."
    echo "Generating initial baseline file..."
    python3 -c "
import json, os, glob

criterion_dir = '${CRITERION_DIR}'
baseline = {}

for estimates_path in glob.glob(os.path.join(criterion_dir, '**', 'new', 'estimates.json'), recursive=True):
    rel_path = os.path.relpath(estimates_path, criterion_dir)
    parts = rel_path.split(os.sep)
    bench_name = '/'.join(parts[:-2])
    with open(estimates_path) as f:
        data = json.load(f)
        mean_ns = data.get('mean', {}).get('point_estimate', 0)
        baseline[bench_name] = mean_ns

with open('${BASELINE_FILE}', 'w') as f:
    json.dump(baseline, f, indent=2)

print(f'Initial baseline saved with {len(baseline)} benchmarks.')
"
    exit 0
fi

python3 -c "
import json, os, glob, sys

criterion_dir = '${CRITERION_DIR}'
baseline_file = '${BASELINE_FILE}'
max_regression = ${MAX_ALLOWED_REGRESSION_PERCENT}

with open(baseline_file) as f:
    baseline = json.load(f)

failed = False
compared_count = 0

print(f'Comparing performance against baseline ({len(baseline)} benchmarks)...')
print('-' * 80)
print(f'{\"Benchmark\":<50} | {\"Baseline (ns)\":<15} | {\"Current (ns)\":<15} | {\"Change\":<10}')
print('-' * 80)

for estimates_path in glob.glob(os.path.join(criterion_dir, '**', 'new', 'estimates.json'), recursive=True):
    rel_path = os.path.relpath(estimates_path, criterion_dir)
    parts = rel_path.split(os.sep)
    bench_name = '/'.join(parts[:-2])
    
    with open(estimates_path) as f:
        data = json.load(f)
        current_ns = data.get('mean', {}).get('point_estimate', 0)
    
    if bench_name in baseline:
        base_ns = baseline[bench_name]
        compared_count += 1
        if base_ns > 0:
            diff_percent = ((current_ns - base_ns) / base_ns) * 100.0
        else:
            diff_percent = 0.0

        status = f'{diff_percent:+.2f}%'
        if diff_percent > max_regression:
            print(f'{bench_name:<50} | {base_ns:<15.2f} | {current_ns:<15.2f} | {status:<10} [FAIL >{max_regression}%]')
            failed = True
        else:
            print(f'{bench_name:<50} | {base_ns:<15.2f} | {current_ns:<15.2f} | {status:<10} [PASS]')

if compared_count == 0:
    print('Warning: No matching benchmarks were compared.')

print('-' * 80)
if failed:
    print('FAILURE: Performance regression exceeded 10.0% threshold!')
    sys.exit(1)
else:
    print('SUCCESS: All performance metrics within allowed <= 10.0% regression threshold.')
    sys.exit(0)
"
