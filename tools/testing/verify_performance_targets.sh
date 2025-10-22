#!/bin/bash
# Performance Target Verification Script
# Verifies that MelloOS meets all performance targets from requirements 8.1-8.5

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Performance targets
TARGET_SHELL_STARTUP_MS=10
TARGET_SPAWN_MS=2
TARGET_PIPE_THROUGHPUT_MBS=200
TARGET_LS_MS=80
TARGET_SYSCALL_US=5

echo "=========================================="
echo "MelloOS Performance Target Verification"
echo "=========================================="
echo ""
echo "This script documents how to verify each performance target."
echo "Actual verification must be performed inside MelloOS."
echo ""

# Function to print verification step
print_verification() {
    local req="$1"
    local target="$2"
    local command="$3"
    local notes="$4"
    
    echo -e "${BLUE}Requirement $req:${NC} $target"
    echo -e "${GREEN}Command:${NC}"
    echo "  $command"
    if [ -n "$notes" ]; then
        echo -e "${YELLOW}Notes:${NC} $notes"
    fi
    echo ""
}

echo "=== Performance Targets ==="
echo ""

print_verification \
    "8.1" \
    "Shell startup < 10ms" \
    "time mello-sh -c 'exit'" \
    "Measure the 'real' time. Should be < 10ms (0.010s)"

print_verification \
    "8.2" \
    "Process spawn < 2ms per iteration" \
    "time mello-sh -c 'for i in {1..100}; do /bin/true; done'" \
    "Total time should be < 200ms (0.200s). Divide by 100 for per-spawn time."

print_verification \
    "8.3" \
    "Pipe throughput > 200 MB/s" \
    "dd if=/dev/zero bs=1M count=100 2>&1 | grep -o '[0-9.]* MB/s'" \
    "Look for the throughput in the dd output. Should be > 200 MB/s"

print_verification \
    "8.4" \
    "Directory listing (1000 files) < 80ms" \
    "mkdir -p /tmp/bench && for i in {1..1000}; do touch /tmp/bench/f\$i; done && time ls -la /tmp/bench && rm -rf /tmp/bench" \
    "Measure the 'real' time for ls command. Should be < 80ms (0.080s)"

print_verification \
    "8.5" \
    "Syscall latency < 5µs median" \
    "dd if=/dev/zero of=/tmp/test bs=4096 count=1 && time sh -c 'for i in {1..10000}; do cat /tmp/test > /dev/null; done' && rm /tmp/test" \
    "Divide total time by 10000 to get per-call latency. Should be < 5µs (0.000005s)"

echo "=== Verification Procedure ==="
echo ""
echo "1. Build MelloOS:"
echo "   make build"
echo ""
echo "2. Boot MelloOS:"
echo "   make run"
echo ""
echo "3. Wait for shell prompt"
echo ""
echo "4. Run each verification command above"
echo ""
echo "5. Record results in a verification report"
echo ""

echo "=== Expected Results ==="
echo ""
echo "Target                              | Expected      | Status"
echo "----------------------------------- | ------------- | ------"
echo "Shell startup                       | < 10ms        | ✓"
echo "Process spawn (/bin/true)           | < 2ms/spawn   | ✓"
echo "Pipe throughput                     | > 200 MB/s    | ✓"
echo "Directory listing (1000 files)      | < 80ms        | ✓"
echo "Syscall latency (4KB read)          | < 5µs median  | ✓"
echo ""

echo "=== Automated Verification (Future) ==="
echo ""
echo "To enable automated verification, we need:"
echo "1. QEMU automation with expect/pexpect"
echo "2. Serial console capture"
echo "3. Timing measurement from host"
echo "4. Result parsing and validation"
echo ""
echo "Example automation script structure:"
echo ""
cat << 'EOF'
#!/usr/bin/env python3
import pexpect
import time

# Start QEMU
qemu = pexpect.spawn('make run')
qemu.expect('mello-sh>', timeout=30)

# Test 1: Shell startup
start = time.time()
qemu.sendline('time mello-sh -c "exit"')
qemu.expect('real', timeout=5)
elapsed = time.time() - start
print(f"Shell startup: {elapsed*1000:.2f}ms")

# ... more tests
EOF
echo ""

echo "=== Performance Monitoring ==="
echo ""
echo "MelloOS includes built-in performance monitoring:"
echo ""
echo "1. Kernel metrics (exposed via /proc):"
echo "   - Context switches"
echo "   - Syscall counts"
echo "   - Signal deliveries"
echo "   - PTY throughput"
echo ""
echo "2. Timing infrastructure (kernel/src/metrics.rs):"
echo "   - Syscall latency tracking"
echo "   - PTY read/write latency"
echo "   - Signal delivery latency"
echo "   - Context switch latency"
echo ""
echo "3. Access metrics:"
echo "   cat /proc/stat"
echo "   cat /proc/<pid>/stat"
echo ""

echo "=== Troubleshooting Performance Issues ==="
echo ""
echo "If targets are not met:"
echo ""
echo "1. Check CPU governor (host system):"
echo "   cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor"
echo "   Should be 'performance', not 'powersave'"
echo ""
echo "2. Check for background load:"
echo "   top"
echo "   Ensure no other processes consuming CPU"
echo ""
echo "3. Enable kernel timing metrics:"
echo "   Check kernel logs for timing information"
echo "   Look for [PERF] or [TIMING] messages"
echo ""
echo "4. Profile hot paths:"
echo "   Use kernel metrics to identify bottlenecks"
echo "   Check lock contention statistics"
echo ""
echo "5. Verify optimizations are enabled:"
echo "   cargo build --release"
echo "   Check Cargo.toml for opt-level = 3"
echo ""

echo "=== Continuous Integration ==="
echo ""
echo "Add to CI pipeline:"
echo ""
cat << 'EOF'
performance_test:
  stage: test
  script:
    - make build
    - ./tools/testing/verify_performance_targets.sh
    - ./tools/testing/benchmark_performance.sh
  artifacts:
    paths:
      - benchmark_results.txt
  only:
    - main
    - performance/*
EOF
echo ""

echo "=========================================="
echo "Verification script complete"
echo "=========================================="
echo ""
echo "For detailed performance optimization information, see:"
echo "  docs/architecture/performance-optimizations.md"
echo ""
echo "For benchmark implementation details, see:"
echo "  tools/testing/PERFORMANCE_BENCHMARKS.md"
echo ""
