#!/bin/bash

# Job Control Integration Test Script
# Tests shell job control functionality including fg, bg, and signals

echo "=========================================="
echo "Job Control Integration Test Suite"
echo "=========================================="

# Configuration
TEST_TIMEOUT=45
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
    echo "  -timeout N      Test timeout in seconds (default: 45)"
    echo "  -h, --help      Show this help message"
    echo ""
    echo "Test Categories:"
    echo "  1. Background job execution"
    echo "  2. Job table management"
    echo "  3. SIGTSTP (Ctrl-Z) handling"
    echo "  4. Foreground/background switching (fg/bg)"
    echo "  5. SIGCONT delivery"
    echo "  6. Process group management"
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
    
    echo -e "${BLUE}Running $test_name...${NC}"
    
    # Clear output file
    > "$OUTPUT_FILE"
    
    # Run QEMU
    qemu-system-x86_64 \
        -M q35 \
        -m 2G \
        -smp 2 \
        -cdrom mellos.iso \
        -boot d \
        -serial file:"$OUTPUT_FILE" \
        -display none \
        -no-reboot \
        -no-shutdown &
    
    QEMU_PID=$!
    sleep $TEST_TIMEOUT
    kill -9 $QEMU_PID 2>/dev/null || true
    wait $QEMU_PID 2>/dev/null || true
    
    echo "Test output captured. Analyzing results..."
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

# Function to analyze test output
analyze_test_output() {
    local test_name=$1
    
    echo ""
    echo "--- Analyzing $test_name Results ---"
    
    # Background job tests
    check_test_result "$test_name" "background.*job.*started" "Background job started with &"
    check_test_result "$test_name" "\[1\].*[0-9]+" "Job ID and PID displayed"
    check_test_result "$test_name" "jobs.*list.*\[1\]" "Job appears in jobs list"
    
    # Job state management
    check_test_result "$test_name" "job.*state.*Running" "Job state tracked as Running"
    check_test_result "$test_name" "job.*state.*Stopped" "Job state changed to Stopped"
    check_test_result "$test_name" "job.*state.*Done" "Job state changed to Done"
    
    # Signal handling tests
    check_test_result "$test_name" "SIGTSTP.*delivered" "SIGTSTP (Ctrl-Z) delivered"
    check_test_result "$test_name" "SIGCONT.*delivered" "SIGCONT delivered"
    check_test_result "$test_name" "SIGCHLD.*received" "SIGCHLD received by shell"
    
    # Foreground/background control
    check_test_result "$test_name" "fg.*resumed.*foreground" "fg command resumed job in foreground"
    check_test_result "$test_name" "bg.*resumed.*background" "bg command resumed job in background"
    check_test_result "$test_name" "tcsetpgrp.*success" "Foreground process group set"
    check_test_result "$test_name" "tcgetpgrp.*success" "Foreground process group retrieved"
    
    # Process group tests
    check_test_result "$test_name" "setpgid.*success" "Process group set correctly"
    check_test_result "$test_name" "getpgrp.*success" "Process group retrieved"
    check_test_result "$test_name" "PGID.*[0-9]+" "Process group ID assigned"
    
    # Job completion
    check_test_result "$test_name" "\[1\].*Done" "Job completion message displayed"
    check_test_result "$test_name" "wait4.*reaped" "Child process reaped"
    
    echo "--- End Analysis ---"
    echo ""
}

# Function to show detailed output
show_detailed_output() {
    echo ""
    echo "--- Detailed QEMU Output ---"
    cat "$OUTPUT_FILE"
    echo "--- End Detailed Output ---"
    echo ""
}

# Main test execution
echo "Configuration:"
echo "  Timeout: ${TEST_TIMEOUT}s"
echo ""

echo "=========================================="
echo "Job Control Integration Tests"
echo "=========================================="

run_qemu_test "JobControl"
analyze_test_output "JobControl"

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

if [ $TESTS_FAILED -eq 0 ] && [ $TOTAL_TESTS -gt 0 ]; then
    echo -e "${GREEN}✓ ALL TESTS PASSED!${NC}"
    echo ""
    echo "Job control is working correctly:"
    echo "  - Background job execution (&)"
    echo "  - Job table management (jobs command)"
    echo "  - Signal handling (SIGTSTP, SIGCONT, SIGCHLD)"
    echo "  - Foreground/background switching (fg, bg)"
    echo "  - Process group management (setpgid, tcsetpgrp)"
    EXIT_CODE=0
elif [ $TOTAL_TESTS -eq 0 ]; then
    echo -e "${YELLOW}⚠ NO TESTS COMPLETED${NC}"
    EXIT_CODE=2
else
    echo -e "${RED}✗ SOME TESTS FAILED${NC}"
    EXIT_CODE=1
fi

echo ""
echo "Detailed results saved to: $RESULTS_FILE"
echo "Full QEMU output saved to: $OUTPUT_FILE"

exit $EXIT_CODE
