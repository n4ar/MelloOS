#!/bin/bash

# Advanced Userland & Shell Integration Test Suite
# Master test runner for all Phase 6.6 integration tests

echo "=========================================="
echo "Advanced Userland & Shell Test Suite"
echo "Phase 6.6 Integration Tests"
echo "=========================================="

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR=$(mktemp -d)
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Test result tracking
SUITES_PASSED=0
SUITES_FAILED=0
TOTAL_SUITES=0

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -all            Run all test suites (default)"
    echo "  -pty            Run PTY integration tests only"
    echo "  -job            Run job control tests only"
    echo "  -pipeline       Run pipeline tests only"
    echo "  -io             Run I/O redirection tests only"
    echo "  -interactive    Run interactive session tests only"
    echo "  -stability      Run stability tests only"
    echo "  -quick          Run quick tests (skip stability)"
    echo "  -timeout N      Set timeout for each test suite"
    echo "  -h, --help      Show this help message"
    echo ""
    echo "Test Suites:"
    echo "  1. PTY Integration (test_pty_integration.sh)"
    echo "  2. Job Control (test_job_control.sh)"
    echo "  3. Pipeline (test_pipeline.sh)"
    echo "  4. I/O Redirection (test_io_redirection.sh)"
    echo "  5. Interactive Session (test_interactive_session.sh)"
    echo "  6. Stability (test_stability.sh)"
}

# Parse command line arguments
RUN_PTY=false
RUN_JOB=false
RUN_PIPELINE=false
RUN_IO=false
RUN_INTERACTIVE=false
RUN_STABILITY=false
RUN_ALL=true
TIMEOUT_ARG=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -all)
            RUN_ALL=true
            shift
            ;;
        -pty)
            RUN_PTY=true
            RUN_ALL=false
            shift
            ;;
        -job)
            RUN_JOB=true
            RUN_ALL=false
            shift
            ;;
        -pipeline)
            RUN_PIPELINE=true
            RUN_ALL=false
            shift
            ;;
        -io)
            RUN_IO=true
            RUN_ALL=false
            shift
            ;;
        -interactive)
            RUN_INTERACTIVE=true
            RUN_ALL=false
            shift
            ;;
        -stability)
            RUN_STABILITY=true
            RUN_ALL=false
            shift
            ;;
        -quick)
            RUN_ALL=true
            RUN_STABILITY=false
            shift
            ;;
        -timeout)
            TIMEOUT_ARG="-timeout $2"
            shift 2
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

# Set all flags if running all tests
if [ "$RUN_ALL" = true ]; then
    RUN_PTY=true
    RUN_JOB=true
    RUN_PIPELINE=true
    RUN_IO=true
    RUN_INTERACTIVE=true
    if [ "$RUN_STABILITY" != false ]; then
        RUN_STABILITY=true
    fi
fi

# Check if ISO exists
if [ ! -f "mellos.iso" ]; then
    echo -e "${RED}Error: mellos.iso not found. Run 'make iso' first.${NC}"
    exit 1
fi

# Function to run a test suite
run_test_suite() {
    local suite_name=$1
    local script_name=$2
    
    TOTAL_SUITES=$((TOTAL_SUITES + 1))
    
    echo ""
    echo -e "${CYAN}=========================================="
    echo "Running: $suite_name"
    echo -e "==========================================${NC}"
    
    local result_file="$RESULTS_DIR/${suite_name// /_}.result"
    
    if [ -f "$SCRIPT_DIR/$script_name" ]; then
        if bash "$SCRIPT_DIR/$script_name" $TIMEOUT_ARG > "$result_file" 2>&1; then
            echo -e "${GREEN}✓ $suite_name PASSED${NC}"
            SUITES_PASSED=$((SUITES_PASSED + 1))
            return 0
        else
            echo -e "${RED}✗ $suite_name FAILED${NC}"
            SUITES_FAILED=$((SUITES_FAILED + 1))
            echo ""
            echo "--- Test Output ---"
            cat "$result_file"
            echo "--- End Output ---"
            return 1
        fi
    else
        echo -e "${YELLOW}⚠ Test script not found: $script_name${NC}"
        SUITES_FAILED=$((SUITES_FAILED + 1))
        return 1
    fi
}

# Print test plan
echo ""
echo "Test Plan:"
[ "$RUN_PTY" = true ] && echo "  ✓ PTY Integration"
[ "$RUN_JOB" = true ] && echo "  ✓ Job Control"
[ "$RUN_PIPELINE" = true ] && echo "  ✓ Pipeline"
[ "$RUN_IO" = true ] && echo "  ✓ I/O Redirection"
[ "$RUN_INTERACTIVE" = true ] && echo "  ✓ Interactive Session"
[ "$RUN_STABILITY" = true ] && echo "  ✓ Stability"
echo ""
echo "Results will be saved to: $RESULTS_DIR"
echo ""

# Run test suites
START_TIME=$(date +%s)

[ "$RUN_PTY" = true ] && run_test_suite "PTY Integration" "test_pty_integration.sh"
[ "$RUN_JOB" = true ] && run_test_suite "Job Control" "test_job_control.sh"
[ "$RUN_PIPELINE" = true ] && run_test_suite "Pipeline" "test_pipeline.sh"
[ "$RUN_IO" = true ] && run_test_suite "I/O Redirection" "test_io_redirection.sh"
[ "$RUN_INTERACTIVE" = true ] && run_test_suite "Interactive Session" "test_interactive_session.sh"
[ "$RUN_STABILITY" = true ] && run_test_suite "Stability" "test_stability.sh"

END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

# Final results
echo ""
echo "=========================================="
echo "Final Test Results"
echo "=========================================="
echo "Total test suites: $TOTAL_SUITES"
echo -e "Passed: ${GREEN}$SUITES_PASSED${NC}"
echo -e "Failed: ${RED}$SUITES_FAILED${NC}"
echo "Total time: ${ELAPSED}s"
echo ""

if [ $SUITES_FAILED -eq 0 ] && [ $TOTAL_SUITES -gt 0 ]; then
    echo -e "${GREEN}✓✓✓ ALL TEST SUITES PASSED! ✓✓✓${NC}"
    echo ""
    echo "Phase 6.6 - Advanced Userland & Shell Environment"
    echo "All integration tests completed successfully!"
    echo ""
    echo "Verified functionality:"
    echo "  ✓ PTY subsystem (allocation, termios, ANSI, SIGWINCH)"
    echo "  ✓ Job control (fg, bg, signals, process groups)"
    echo "  ✓ Pipelines (multi-stage, PGID management)"
    echo "  ✓ I/O redirection (>, >>, <, combined with pipes)"
    echo "  ✓ Interactive session (boot, prompt, commands)"
    if [ "$RUN_STABILITY" = true ]; then
        echo "  ✓ System stability (no panics, no leaks, no zombies)"
    fi
    EXIT_CODE=0
elif [ $TOTAL_SUITES -eq 0 ]; then
    echo -e "${YELLOW}⚠ NO TEST SUITES RAN${NC}"
    EXIT_CODE=2
else
    echo -e "${RED}✗✗✗ SOME TEST SUITES FAILED ✗✗✗${NC}"
    echo ""
    echo "Failed suites: $SUITES_FAILED / $TOTAL_SUITES"
    echo "Check the output above for details."
    EXIT_CODE=1
fi

echo ""
echo "Detailed results saved to: $RESULTS_DIR"
echo ""

exit $EXIT_CODE
