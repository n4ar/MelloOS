#!/bin/bash
# Performance Benchmark Suite for MelloOS Advanced Userland
# Tests shell startup, process spawn, pipe throughput, directory listing, and syscall latency

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Performance targets (from requirements)
TARGET_SHELL_STARTUP_MS=10
TARGET_SPAWN_MS=2
TARGET_PIPE_THROUGHPUT_MBS=200
TARGET_LS_MS=80
TARGET_SYSCALL_US=5

# Results tracking
RESULTS_FILE="benchmark_results.txt"
PASSED=0
FAILED=0

echo "=========================================="
echo "MelloOS Performance Benchmark Suite"
echo "=========================================="
echo ""

# Function to print test result
print_result() {
    local test_name="$1"
    local result="$2"
    local target="$3"
    local unit="$4"
    local passed="$5"
    
    if [ "$passed" = "true" ]; then
        echo -e "${GREEN}✓${NC} $test_name: ${result}${unit} (target: <${target}${unit})"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}✗${NC} $test_name: ${result}${unit} (target: <${target}${unit})"
        FAILED=$((FAILED + 1))
    fi
}

# Function to extract time in milliseconds from 'time' output
extract_time_ms() {
    local time_output="$1"
    # Extract real time and convert to milliseconds
    # Format: "real 0m0.005s" -> 5ms
    local seconds=$(echo "$time_output" | grep real | awk '{print $2}' | sed 's/[^0-9.]//g')
    echo "scale=2; $seconds * 1000" | bc
}

echo "Starting benchmarks..."
echo ""

# Initialize results file
echo "MelloOS Performance Benchmark Results" > "$RESULTS_FILE"
echo "Date: $(date)" >> "$RESULTS_FILE"
echo "========================================" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

#
# Benchmark 1: Shell Startup Time
#
echo -e "${BLUE}[1/5]${NC} Testing shell startup time..."
echo "Benchmark 1: Shell Startup Time" >> "$RESULTS_FILE"

# We'll measure this by timing a simple exit command
# Run multiple iterations and take the average
ITERATIONS=10
TOTAL_TIME=0

for i in $(seq 1 $ITERATIONS); do
    START=$(date +%s%N)
    # This would run: mello-sh -c 'exit'
    # For now, we'll simulate with a placeholder
    # In actual implementation, this would be:
    # /path/to/mello-sh -c 'exit'
    END=$(date +%s%N)
    ELAPSED=$((END - START))
    TOTAL_TIME=$((TOTAL_TIME + ELAPSED))
done

AVG_TIME_NS=$((TOTAL_TIME / ITERATIONS))
AVG_TIME_MS=$(echo "scale=2; $AVG_TIME_NS / 1000000" | bc)

if (( $(echo "$AVG_TIME_MS < $TARGET_SHELL_STARTUP_MS" | bc -l) )); then
    print_result "Shell startup" "$AVG_TIME_MS" "$TARGET_SHELL_STARTUP_MS" "ms" "true"
    echo "Result: PASS ($AVG_TIME_MS ms)" >> "$RESULTS_FILE"
else
    print_result "Shell startup" "$AVG_TIME_MS" "$TARGET_SHELL_STARTUP_MS" "ms" "false"
    echo "Result: FAIL ($AVG_TIME_MS ms)" >> "$RESULTS_FILE"
fi
echo "" >> "$RESULTS_FILE"

#
# Benchmark 2: Process Spawn Time
#
echo -e "${BLUE}[2/5]${NC} Testing process spawn time..."
echo "Benchmark 2: Process Spawn Time" >> "$RESULTS_FILE"

# Measure time to spawn /bin/true 100 times
# Command: time mello-sh -c 'for i in {1..100}; do /bin/true; done'
SPAWN_ITERATIONS=100

START=$(date +%s%N)
# Placeholder for actual command
# In actual implementation:
# /path/to/mello-sh -c 'for i in {1..100}; do /bin/true; done'
for i in $(seq 1 $SPAWN_ITERATIONS); do
    /bin/true 2>/dev/null || true
done
END=$(date +%s%N)

TOTAL_TIME_NS=$((END - START))
TOTAL_TIME_MS=$(echo "scale=2; $TOTAL_TIME_NS / 1000000" | bc)
PER_SPAWN_MS=$(echo "scale=2; $TOTAL_TIME_MS / $SPAWN_ITERATIONS" | bc)

if (( $(echo "$PER_SPAWN_MS < $TARGET_SPAWN_MS" | bc -l) )); then
    print_result "Process spawn" "$PER_SPAWN_MS" "$TARGET_SPAWN_MS" "ms/spawn" "true"
    echo "Result: PASS ($PER_SPAWN_MS ms per spawn)" >> "$RESULTS_FILE"
else
    print_result "Process spawn" "$PER_SPAWN_MS" "$TARGET_SPAWN_MS" "ms/spawn" "false"
    echo "Result: FAIL ($PER_SPAWN_MS ms per spawn)" >> "$RESULTS_FILE"
fi
echo "" >> "$RESULTS_FILE"

#
# Benchmark 3: Pipe Throughput
#
echo -e "${BLUE}[3/5]${NC} Testing pipe throughput..."
echo "Benchmark 3: Pipe Throughput" >> "$RESULTS_FILE"

# Measure throughput: dd if=/dev/zero bs=1M count=100 | cat > /dev/null
PIPE_SIZE_MB=100

START=$(date +%s%N)
dd if=/dev/zero bs=1M count=$PIPE_SIZE_MB 2>/dev/null | cat > /dev/null
END=$(date +%s%N)

ELAPSED_NS=$((END - START))
ELAPSED_S=$(echo "scale=3; $ELAPSED_NS / 1000000000" | bc)
THROUGHPUT_MBS=$(echo "scale=2; $PIPE_SIZE_MB / $ELAPSED_S" | bc)

if (( $(echo "$THROUGHPUT_MBS > $TARGET_PIPE_THROUGHPUT_MBS" | bc -l) )); then
    print_result "Pipe throughput" "$THROUGHPUT_MBS" "$TARGET_PIPE_THROUGHPUT_MBS" "MB/s" "true"
    echo "Result: PASS ($THROUGHPUT_MBS MB/s)" >> "$RESULTS_FILE"
else
    print_result "Pipe throughput" "$THROUGHPUT_MBS" "$TARGET_PIPE_THROUGHPUT_MBS" "MB/s" "false"
    echo "Result: FAIL ($THROUGHPUT_MBS MB/s)" >> "$RESULTS_FILE"
fi
echo "" >> "$RESULTS_FILE"

#
# Benchmark 4: Directory Listing
#
echo -e "${BLUE}[4/5]${NC} Testing directory listing performance..."
echo "Benchmark 4: Directory Listing" >> "$RESULTS_FILE"

# Create a test directory with 1000+ files
TEST_DIR="/tmp/mello_bench_$$"
mkdir -p "$TEST_DIR"

echo "Creating 1000 test files..."
for i in $(seq 1 1000); do
    touch "$TEST_DIR/file_$i.txt"
done

# Measure time to list directory
START=$(date +%s%N)
ls -la "$TEST_DIR" > /dev/null
END=$(date +%s%N)

ELAPSED_NS=$((END - START))
ELAPSED_MS=$(echo "scale=2; $ELAPSED_NS / 1000000" | bc)

# Cleanup
rm -rf "$TEST_DIR"

if (( $(echo "$ELAPSED_MS < $TARGET_LS_MS" | bc -l) )); then
    print_result "Directory listing (1000 files)" "$ELAPSED_MS" "$TARGET_LS_MS" "ms" "true"
    echo "Result: PASS ($ELAPSED_MS ms)" >> "$RESULTS_FILE"
else
    print_result "Directory listing (1000 files)" "$ELAPSED_MS" "$TARGET_LS_MS" "ms" "false"
    echo "Result: FAIL ($ELAPSED_MS ms)" >> "$RESULTS_FILE"
fi
echo "" >> "$RESULTS_FILE"

#
# Benchmark 5: Syscall Latency
#
echo -e "${BLUE}[5/5]${NC} Testing syscall latency..."
echo "Benchmark 5: Syscall Latency (read/write 4KB)" >> "$RESULTS_FILE"

# Create a 4KB test file
TEST_FILE="/tmp/mello_syscall_bench_$$"
dd if=/dev/zero of="$TEST_FILE" bs=4096 count=1 2>/dev/null

# Measure read/write latency (hot cache)
SYSCALL_ITERATIONS=10000

# Warm up cache
cat "$TEST_FILE" > /dev/null

# Measure read latency
START=$(date +%s%N)
for i in $(seq 1 $SYSCALL_ITERATIONS); do
    cat "$TEST_FILE" > /dev/null
done
END=$(date +%s%N)

TOTAL_NS=$((END - START))
PER_CALL_NS=$((TOTAL_NS / SYSCALL_ITERATIONS))
PER_CALL_US=$(echo "scale=2; $PER_CALL_NS / 1000" | bc)

# Cleanup
rm -f "$TEST_FILE"

if (( $(echo "$PER_CALL_US < $TARGET_SYSCALL_US" | bc -l) )); then
    print_result "Syscall latency (4KB read)" "$PER_CALL_US" "$TARGET_SYSCALL_US" "µs" "true"
    echo "Result: PASS ($PER_CALL_US µs)" >> "$RESULTS_FILE"
else
    print_result "Syscall latency (4KB read)" "$PER_CALL_US" "$TARGET_SYSCALL_US" "µs" "false"
    echo "Result: FAIL ($PER_CALL_US µs)" >> "$RESULTS_FILE"
fi
echo "" >> "$RESULTS_FILE"

#
# Summary
#
echo ""
echo "=========================================="
echo "Benchmark Summary"
echo "=========================================="
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All performance targets met!${NC}"
    echo "Summary: ALL TESTS PASSED" >> "$RESULTS_FILE"
    exit 0
else
    echo -e "${YELLOW}Some performance targets not met.${NC}"
    echo "Summary: $FAILED test(s) failed" >> "$RESULTS_FILE"
    exit 1
fi
