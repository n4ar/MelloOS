#!/bin/bash

# Stability Integration Test Script
# Long-running tests for system stability, memory leaks, and zombie processes

echo "=========================================="
echo "Stability Integration Test Suite"
echo "=========================================="

# Configuration
TEST_TIMEOUT=120  # 2 minutes for stability testing
OUTPUT_FILE=$(mktemp)
RESULTS_FILE=$(mktemp)

# Test result tracking
TESTS_PASSED=0
TESTS_FAILED=0
TOTAL_TESTS=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -timeout N      Test timeout in seconds (default: 120)"
    echo "  -short          Run short stability test (30s)"
    echo "  -long           Run extended stability test (300s)"
    echo "  -h, --help      Show this help message"
    echo ""
    echo "Test Categories:"
    echo "  1. Extended runtime stability"
    echo "  2. Repeated command execution"
    echo "  3. Zombie process detection"
    echo "  4. Memory leak detection"
    echo "  5. Kernel panic detection"
    echo "  6. Resource exhaustion handling"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -timeout)
            if [[ "$2" =~ ^[0-9]+$ ]] && [ "$2" -gt 0 ]; then
                TEST_TIMEOUT="$2"
                shift 2
            else
                echo "Error: Timeout must be a positive number"
                exit 1
            fi
            ;;
        -short)
            TEST_TIMEOUT=30
            shift
            ;;
        -long)
            TEST_TIMEOUT=300
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Run '$0 --help' for usage information."
            exit 1
            ;;
    esac
done

# Check if ISO exists
if [ ! -f "mellos.iso" ]; then
    echo -e "${RED}Error: mellos.iso not found. Run 'make iso' first.${NC}"
    exit 1
fi

# Function to run QEMU test
run_qemu_test() {
    local test_name=$1
    
    echo -e "${BLUE}Running $test_name for ${TEST_TIMEOUT}s...${NC}"
    
    # Clear output file
    > "$OUTPUT_FILE"
    
    # Run QEMU
    qemu-system-x86_64 \
        -M q35 \
        -m 2G \
        -smp 4 \
        -cdrom mellos.iso \
        -boot d \
        -serial file:"$OUTPUT_FILE" \
        -display none \
        -no-reboot \
        -no-shutdown &
    
    QEMU_PID=$!
    
    # Monitor progress
    local elapsed=0
    local interval=10
    while [ $elapsed -lt $TEST_TIMEOUT ]; do
        sleep $interval
        elapsed=$((elapsed + interval))
        echo "  ... ${elapsed}s elapsed (${TEST_TIMEOUT}s total)"
    done
    
    kill -9 $QEMU_PID 2>/dev/null || true
    wait $QEMU_PID 2>/dev/null || true
    
    echo "Test completed. Analyzing results..."
}

# Function to check test result
check_test_result() {
    local test_name=$1
    local pattern=$2
    local description=$3
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if grep -q "$pattern" "$OUTPUT_FILE"; then
        echo -e "${GREEN}✓ PASS${NC}: $description"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo "PASS: $test_name - $description" >> "$RESULTS_FILE"
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}: $description"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo "FAIL: $test_name - $description" >> "$RESULTS_FILE"
        return 1
    fi
}

# Function to check absence of pattern (for negative tests)
check_no_pattern() {
    local test_name=$1
    local pattern=$2
    local description=$3
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if ! grep -q "$pattern" "$OUTPUT_FILE"; then
        echo -e "${GREEN}✓ PASS${NC}: $description"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo "PASS: $test_name - $description" >> "$RESULTS_FILE"
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}: $description"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo "FAIL: $test_name - $description" >> "$RESULTS_FILE"
        return 1
    fi
}

# Function to count occurrences
count_pattern() {
    local pattern=$1
    grep -c "$pattern" "$OUTPUT_FILE" 2>/dev/null || echo "0"
}

# Function to analyze test output
analyze_test_output() {
    local test_name=$1
    
    echo ""
    echo "--- Analyzing $test_name Results ---"
    
    # System stability tests
    check_no_pattern "$test_name" "KERNEL PANIC" "No kernel panics occurred"
    check_no_pattern "$test_name" "triple.*fault" "No triple faults occurred"
    check_no_pattern "$test_name" "page.*fault.*kernel" "No kernel page faults"
    check_no_pattern "$test_name" "deadlock.*detected" "No deadlocks detected"
    
    # Repeated command execution
    check_test_result "$test_name" "stress.*test.*running" "Stress test executed"
    check_test_result "$test_name" "command.*iteration.*[0-9]+" "Commands executed repeatedly"
    local iterations=$(count_pattern "iteration")
    echo "  Commands executed: $iterations iterations"
    
    # Zombie process detection
    check_no_pattern "$test_name" "zombie.*accumulating" "No zombie accumulation"
    check_no_pattern "$test_name" "zombie.*count.*increasing" "Zombie count not increasing"
    check_test_result "$test_name" "zombie.*reaping.*working" "Zombie reaping working"
    local zombie_count=$(count_pattern "zombie.*process")
    echo "  Zombie processes detected: $zombie_count"
    
    # Memory leak detection
    check_test_result "$test_name" "memory.*stable" "Memory usage stable"
    check_no_pattern "$test_name" "memory.*leak.*detected" "No memory leaks detected"
    check_no_pattern "$test_name" "out.*of.*memory" "No OOM conditions"
    check_test_result "$test_name" "heap.*allocations.*freed" "Heap allocations properly freed"
    
    # Resource management
    check_test_result "$test_name" "file.*descriptors.*managed" "File descriptors managed correctly"
    check_no_pattern "$test_name" "fd.*leak" "No file descriptor leaks"
    check_no_pattern "$test_name" "too.*many.*open.*files" "No FD exhaustion"
    
    # Process management
    check_test_result "$test_name" "process.*creation.*working" "Process creation working"
    check_test_result "$test_name" "process.*cleanup.*working" "Process cleanup working"
    check_test_result "$test_name" "fork.*exec.*cycle" "Fork/exec cycle stable"
    
    # Lock contention and performance
    check_no_pattern "$test_name" "lock.*timeout" "No lock timeouts"
    check_no_pattern "$test_name" "priority.*inversion" "No priority inversion"
    check_test_result "$test_name" "lock.*ordering.*correct" "Lock ordering correct"
    
    # Signal handling stability
    check_test_result "$test_name" "signal.*delivery.*stable" "Signal delivery stable"
    check_no_pattern "$test_name" "signal.*lost" "No signals lost"
    check_no_pattern "$test_name" "signal.*race" "No signal races"
    
    # SMP safety
    check_test_result "$test_name" "SMP.*stable" "SMP operation stable"
    check_test_result "$test_name" "CPU.*[0-3].*active" "All CPUs active"
    check_no_pattern "$test_name" "CPU.*hung" "No CPU hangs"
    
    # System uptime
    check_test_result "$test_name" "uptime.*[0-9]+.*seconds" "System uptime tracked"
    check_test_result "$test_name" "system.*running.*continuously" "System ran continuously"
    
    echo "--- End Analysis ---"
    echo ""
}

# Function to show summary statistics
show_statistics() {
    echo ""
    echo "--- System Statistics ---"
    
    local total_commands=$(count_pattern "command.*executed")
    local total_forks=$(count_pattern "fork.*success")
    local total_signals=$(count_pattern "signal.*delivered")
    local context_switches=$(count_pattern "context.*switch")
    
    echo "Total commands executed: $total_commands"
    echo "Total fork operations: $total_forks"
    echo "Total signals delivered: $total_signals"
    echo "Context switches: $context_switches"
    
    echo "--- End Statistics ---"
    echo ""
}

# Function to show detailed output
show_detailed_output() {
    echo ""
    echo "--- Detailed QEMU Output (last 100 lines) ---"
    tail -100 "$OUTPUT_FILE"
    echo "--- End Detailed Output ---"
    echo ""
}

# Main test execution
echo "Configuration:"
echo "  Timeout: ${TEST_TIMEOUT}s"
echo "  CPUs: 4"
echo ""

echo "=========================================="
echo "Stability Integration Tests"
echo "=========================================="

run_qemu_test "Stability"
analyze_test_output "Stability"
show_statistics

# Show output if any tests failed
if [ $TESTS_FAILED -gt 0 ]; then
    show_detailed_output
fi

# Final results
echo "=========================================="
echo "Test Results Summary"
echo "=========================================="
echo "Total tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Failed: ${RED}$TESTS_FAILED${NC}"
echo "Runtime: ${TEST_TIMEOUT}s"

if [ $TESTS_FAILED -eq 0 ] && [ $TOTAL_TESTS -gt 0 ]; then
    echo -e "${GREEN}✓ ALL TESTS PASSED!${NC}"
    echo ""
    echo "System stability verified:"
    echo "  - No kernel panics during ${TEST_TIMEOUT}s runtime"
    echo "  - No zombie process accumulation"
    echo "  - No memory leaks detected"
    echo "  - No deadlocks or lock timeouts"
    echo "  - SMP operation stable on 4 CPUs"
    echo "  - Resource management working correctly"
    EXIT_CODE=0
elif [ $TOTAL_TESTS -eq 0 ]; then
    echo -e "${YELLOW}⚠ NO TESTS COMPLETED${NC}"
    EXIT_CODE=2
else
    echo -e "${RED}✗ SOME TESTS FAILED${NC}"
    echo ""
    echo "Stability issues detected. Review the detailed output above."
    EXIT_CODE=1
fi

echo ""
echo "Detailed results saved to: $RESULTS_FILE"
echo "Full QEMU output saved to: $OUTPUT_FILE"

exit $EXIT_CODE
