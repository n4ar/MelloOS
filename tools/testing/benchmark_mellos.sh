#!/bin/bash
# MelloOS-Specific Performance Benchmark Script
# This script runs benchmarks inside MelloOS via QEMU

set -e

QEMU_TIMEOUT=120
RESULTS_FILE="mellos_benchmark_results.txt"

echo "=========================================="
echo "MelloOS Performance Benchmark (In-System)"
echo "=========================================="
echo ""

# Check if MelloOS is built
if [ ! -f "kernel/target/x86_64-unknown-none/release/mellos_kernel" ]; then
    echo "Error: MelloOS kernel not built. Run 'make build' first."
    exit 1
fi

echo "Starting MelloOS in QEMU for benchmarking..."
echo ""

# Create benchmark commands file
cat > /tmp/mellos_bench_commands.txt << 'EOF'
# Benchmark 1: Shell startup time
echo "=== Benchmark 1: Shell Startup ==="
time mello-sh -c 'exit'

# Benchmark 2: Process spawn
echo "=== Benchmark 2: Process Spawn (100x /bin/true) ==="
time mello-sh -c 'for i in 1 2 3 4 5 6 7 8 9 10; do /bin/true; /bin/true; /bin/true; /bin/true; /bin/true; /bin/true; /bin/true; /bin/true; /bin/true; /bin/true; done'

# Benchmark 3: Pipe throughput
echo "=== Benchmark 3: Pipe Throughput ==="
# Note: This requires dd and proper /dev/zero support
# For now, test with smaller data
echo "Testing pipe with echo and cat..."
time sh -c 'for i in 1 2 3 4 5; do echo "test data line $i"; done | cat | cat | cat'

# Benchmark 4: Directory listing
echo "=== Benchmark 4: Directory Listing ==="
# Create test directory with files
mkdir -p /tmp/bench_test
for i in 1 2 3 4 5 6 7 8 9 10; do
    touch /tmp/bench_test/file_$i.txt
done
time ls -la /tmp/bench_test
rm -rf /tmp/bench_test

# Benchmark 5: Syscall latency
echo "=== Benchmark 5: Syscall Latency ==="
# Test with repeated small reads
echo "test" > /tmp/bench_file
time sh -c 'for i in 1 2 3 4 5 6 7 8 9 10; do cat /tmp/bench_file > /dev/null; done'
rm /tmp/bench_file

echo "=== Benchmarks Complete ==="
EOF

echo "Benchmark commands prepared."
echo "Note: Actual benchmarking requires MelloOS to be fully booted with shell support."
echo ""
echo "To run benchmarks manually:"
echo "1. Boot MelloOS: make run"
echo "2. In the shell, run the benchmark commands"
echo "3. Record the timing results"
echo ""

# Cleanup
rm -f /tmp/mellos_bench_commands.txt

echo "Benchmark script prepared. Manual execution required in MelloOS environment."
