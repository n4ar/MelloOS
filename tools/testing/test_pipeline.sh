#!/bin/bash

# Pipeline Integration Test Script
# Tests shell pipeline functionality and process group management

echo "=========================================="
echo "Pipeline Integration Test Suite"
echo "=========================================="

# Configuration
TEST_TIMEOUT=30
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
    echo "  -timeout N      Test timeout in seconds (default: 30)"
    echo "  -h, --help      Show this help message"
    echo ""
    echo "Test Categories:"
    echo "  1. Simple pipeline execution"
    echo "  2. Multi-stage pipelines"
    echo "  3. Process group management"
    echo "  4. Exit status propagation"
    echo "  5. Pipe buffer handling"
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
    
    # Pipeline parsing tests
    check_test_result "$test_name" "pipeline.*parsed" "Pipeline command parsed correctly"
    check_test_result "$test_name" "pipe.*created" "Pipe created successfully"
    
    # Simple pipeline tests
    check_test_result "$test_name" "echo.*pipe.*cat" "Simple two-stage pipeline executed"
    check_test_result "$test_name" "pipeline.*output.*correct" "Pipeline output correct"
    
    # Multi-stage pipeline tests
    check_test_result "$test_name" "three.*stage.*pipeline" "Three-stage pipeline executed"
    check_test_result "$test_name" "echo.*grep.*wc" "echo | grep | wc pipeline worked"
    
    # Process group tests
    check_test_result "$test_name" "all.*processes.*same.*PGID" "All pipeline processes in same PGID"
    check_test_result "$test_name" "pipeline.*PGID.*[0-9]+" "Pipeline PGID assigned"
    check_test_result "$test_name" "setpgid.*pipeline" "setpgid called for pipeline"
    
    # Exit status tests
    check_test_result "$test_name" "exit.*status.*propagated" "Exit status propagated"
    check_test_result "$test_name" "last.*command.*status" "Last command status returned"
    check_test_result "$test_name" "pipeline.*exit.*0" "Successful pipeline exit status"
    
    # Pipe buffer tests
    check_test_result "$test_name" "pipe.*read.*success" "Pipe read successful"
    check_test_result "$test_name" "pipe.*write.*success" "Pipe write successful"
    check_test_result "$test_name" "pipe.*closed.*correctly" "Pipe ends closed correctly"
    
    # File descriptor management
    check_test_result "$test_name" "dup2.*stdin" "stdin redirected with dup2"
    check_test_result "$test_name" "dup2.*stdout" "stdout redirected with dup2"
    check_test_result "$test_name" "unused.*pipe.*closed" "Unused pipe ends closed"
    
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
echo "Pipeline Integration Tests"
echo "=========================================="

run_qemu_test "Pipeline"
analyze_test_output "Pipeline"

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
    echo "Pipeline functionality is working correctly:"
    echo "  - Simple two-stage pipelines (cmd1 | cmd2)"
    echo "  - Multi-stage pipelines (cmd1 | cmd2 | cmd3)"
    echo "  - Process group management (all in same PGID)"
    echo "  - Exit status propagation (last command)"
    echo "  - Pipe buffer handling and FD management"
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
