#!/bin/bash

# User-Mode Integration Test Script
# Automated QEMU tests for user-mode support functionality

echo "=========================================="
echo "User-Mode Integration Test Suite"
echo "=========================================="

# Configuration
TEST_TIMEOUT=60  # Seconds to wait for tests
SMP_CPUS=2      # Default CPU count for SMP testing
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
NC='\033[0m' # No Color

show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -smp N          Number of CPU cores for SMP testing (1-4, default: 2)"
    echo "  -timeout N      Test timeout in seconds (default: 15)"
    echo "  -single         Run single-CPU tests only"
    echo "  -smp-only       Run SMP tests only"
    echo "  -h, --help      Show this help message"
    echo ""
    echo "Test Categories:"
    echo "  1. Basic user-mode transition"
    echo "  2. Privilege level validation"
    echo "  3. Syscall functionality"
    echo "  4. Fork chain stress test"
    echo "  5. Memory protection"
    echo "  6. SMP safety (multi-CPU only)"
}

# Parse command line arguments
RUN_SINGLE=true
RUN_SMP=true

while [[ $# -gt 0 ]]; do
    case $1 in
        -smp)
            if [[ "$2" =~ ^[1-4]$ ]]; then
                SMP_CPUS="$2"
                shift 2
            else
                echo "Error: SMP CPU count must be between 1 and 4"
                exit 1
            fi
            ;;
        -timeout)
            if [[ "$2" =~ ^[0-9]+$ ]] && [ "$2" -gt 0 ]; then
                TEST_TIMEOUT="$2"
                shift 2
            else
                echo "Error: Timeout must be a positive number"
                exit 1
            fi
            ;;
        -single)
            RUN_SINGLE=true
            RUN_SMP=false
            shift
            ;;
        -smp-only)
            RUN_SINGLE=false
            RUN_SMP=true
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
    local cpus=$1
    local test_name=$2
    
    echo -e "${BLUE}Running $test_name with $cpus CPU(s)...${NC}"
    
    # Clear output file
    > "$OUTPUT_FILE"
    
    # Run QEMU
    qemu-system-x86_64 \
        -M q35 \
        -m 2G \
        -smp $cpus \
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
    
    # Core user-mode functionality tests
    check_test_result "$test_name" "Hello from userland!" "Init process started and printed required message"
    check_test_result "$test_name" "Running at privilege level 3" "Process running in user mode (ring 3)"
    check_test_result "$test_name" "sys_getpid returned valid PID" "sys_getpid syscall working"
    check_test_result "$test_name" "sys_write working correctly" "sys_write syscall working"
    check_test_result "$test_name" "sys_yield completed successfully" "sys_yield syscall working"
    
    # Fork functionality tests
    check_test_result "$test_name" "Fork chain test completed successfully" "Fork chain stress test passed"
    check_test_result "$test_name" "Child process created in fork chain" "Fork creating child processes"
    
    # Memory protection tests
    check_test_result "$test_name" "Valid user memory access succeeded" "Valid memory access working"
    check_test_result "$test_name" "Invalid kernel memory access correctly rejected" "Kernel memory protection working"
    check_test_result "$test_name" "Null pointer access correctly rejected" "Null pointer protection working"
    
    # Integration test framework tests
    check_test_result "$test_name" "USER-TEST.*Privilege Level Validation" "Kernel integration tests running"
    check_test_result "$test_name" "USER-TEST.*Basic Syscall Functionality" "Syscall integration tests running"
    
    # SMP-specific tests (only for multi-CPU)
    if [[ "$test_name" == *"SMP"* ]]; then
        check_test_result "$test_name" "SMP-A.*CPU.*iteration" "SMP safety test A running"
        check_test_result "$test_name" "SMP-B.*CPU.*iteration" "SMP safety test B running"
        check_test_result "$test_name" "SMP Safety Test.*completed" "SMP safety tests completed"
        check_test_result "$test_name" "Performance test starting" "Performance monitoring test running"
    fi
    
    # System stability tests
    check_test_result "$test_name" "Init process monitoring system" "System remained stable"
    
    echo "--- End Analysis ---"
    echo ""
}

# Function to show detailed output on failure
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
echo "  SMP CPUs: $SMP_CPUS"
echo "  Run single-CPU: $RUN_SINGLE"
echo "  Run SMP: $RUN_SMP"
echo ""

# Test 1: Single-CPU user-mode tests
if [ "$RUN_SINGLE" = true ]; then
    echo "=========================================="
    echo "Test 1: Single-CPU User-Mode Tests"
    echo "=========================================="
    
    run_qemu_test 1 "Single-CPU"
    analyze_test_output "Single-CPU"
    
    # Show output if any tests failed
    if [ $TESTS_FAILED -gt 0 ]; then
        show_detailed_output
    fi
fi

# Test 2: SMP user-mode tests
if [ "$RUN_SMP" = true ] && [ $SMP_CPUS -gt 1 ]; then
    echo "=========================================="
    echo "Test 2: SMP User-Mode Tests ($SMP_CPUS CPUs)"
    echo "=========================================="
    
    run_qemu_test $SMP_CPUS "SMP"
    analyze_test_output "SMP"
    
    # Show output if any tests failed
    if [ $TESTS_FAILED -gt 0 ]; then
        show_detailed_output
    fi
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
    echo "User-mode support is working correctly:"
    echo "  - Privilege level transitions (ring 0 ↔ ring 3)"
    echo "  - Syscall interface (syscall/sysret)"
    echo "  - Process management (fork/exit/wait)"
    echo "  - Memory protection (user/kernel separation)"
    if [ "$RUN_SMP" = true ] && [ $SMP_CPUS -gt 1 ]; then
        echo "  - SMP safety (multi-core process execution)"
    fi
    
    EXIT_CODE=0
elif [ $TOTAL_TESTS -eq 0 ]; then
    echo -e "${YELLOW}⚠ NO TESTS COMPLETED${NC}"
    echo ""
    echo "Possible issues:"
    echo "  - Kernel failed to boot"
    echo "  - User-mode transition failed"
    echo "  - Init process crashed"
    echo ""
    echo "Check the detailed output above for more information."
    EXIT_CODE=2
else
    echo -e "${RED}✗ SOME TESTS FAILED${NC}"
    echo ""
    echo "Failed tests indicate issues with user-mode support."
    echo "Check the detailed output above for specific failures."
    echo ""
    echo "Common issues:"
    echo "  - GDT/TSS configuration problems"
    echo "  - Syscall handler issues"
    echo "  - Memory protection not working"
    echo "  - Process management bugs"
    EXIT_CODE=1
fi

echo ""
echo "Detailed results saved to: $RESULTS_FILE"
echo "Full QEMU output saved to: $OUTPUT_FILE"

# Cleanup
# Note: Keep temp files for debugging, user can clean them manually

exit $EXIT_CODE