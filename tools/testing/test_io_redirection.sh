#!/bin/bash

# I/O Redirection Integration Test Script
# Tests shell I/O redirection functionality

echo "=========================================="
echo "I/O Redirection Integration Test Suite"
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
    echo "  1. Output redirection (>)"
    echo "  2. Append redirection (>>)"
    echo "  3. Input redirection (<)"
    echo "  4. Combined redirects with pipes"
    echo "  5. File descriptor management"
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
    
    # Output redirection tests
    check_test_result "$test_name" "redirect.*output.*>" "Output redirection parsed"
    check_test_result "$test_name" "file.*created.*test.txt" "Output file created"
    check_test_result "$test_name" "echo.*>.*test.txt" "echo > test.txt executed"
    check_test_result "$test_name" "write.*to.*file.*success" "Data written to file"
    
    # Append redirection tests
    check_test_result "$test_name" "redirect.*append.*>>" "Append redirection parsed"
    check_test_result "$test_name" "append.*mode.*O_APPEND" "Append mode flag set"
    
    # Input redirection tests
    check_test_result "$test_name" "redirect.*input.*<" "Input redirection parsed"
    check_test_result "$test_name" "read.*from.*file" "Read from file successful"
    
    # File content verification
    check_test_result "$test_name" "file.*content.*hello world" "File contains expected content"
    check_test_result "$test_name" "cat.*test.txt.*hello world" "cat reads file correctly"
    check_test_result "$test_name" "HELLO WORLD" "Uppercase conversion worked"
    
    # Combined redirect and pipe tests
    check_test_result "$test_name" "pipe.*redirect.*combined" "Pipe and redirect combined"
    check_test_result "$test_name" "cat.*pipe.*tr.*redirect" "cat | tr > file worked"
    check_test_result "$test_name" "output.txt.*created" "Final output file created"
    
    # File descriptor management
    check_test_result "$test_name" "dup2.*redirect" "dup2 used for redirection"
    check_test_result "$test_name" "fd.*closed.*after.*dup2" "Original FD closed after dup2"
    check_test_result "$test_name" "O_WRONLY.*O_CREAT" "File opened with correct flags"
    check_test_result "$test_name" "O_RDONLY" "Input file opened read-only"
    
    # Error handling
    check_test_result "$test_name" "redirect.*error.*handled" "Redirect errors handled"
    
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
echo "I/O Redirection Integration Tests"
echo "=========================================="

run_qemu_test "IORedirection"
analyze_test_output "IORedirection"

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
    echo "I/O redirection is working correctly:"
    echo "  - Output redirection (> file)"
    echo "  - Append redirection (>> file)"
    echo "  - Input redirection (< file)"
    echo "  - Combined pipes and redirects"
    echo "  - File descriptor management (dup2)"
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
