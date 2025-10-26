#!/bin/bash
# Test script for Task 15: Verify Shell Integration
# Tests all requirements: R9.1-R9.7

set -e

echo "========================================="
echo "Task 15: Shell Integration Verification"
echo "========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

# Function to print test result
print_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ PASS${NC}: $2"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}: $2"
        ((TESTS_FAILED++))
    fi
}

echo "Building MelloOS..."
make clean > /dev/null 2>&1
make build > /dev/null 2>&1
make iso > /dev/null 2>&1
echo -e "${GREEN}✓${NC} Build complete"
echo ""

echo "========================================="
echo "Manual Verification Required"
echo "========================================="
echo ""
echo "This test requires manual verification of shell integration."
echo "The system will boot in QEMU. Please verify the following:"
echo ""
echo "R9.1: Shell displays prompt on startup"
echo "  - Look for 'mello-sh>' or similar prompt"
echo ""
echo "R9.2: Shell reads from stdin (FD 0)"
echo "  - Type commands and verify they are echoed"
echo ""
echo "R9.3: Shell writes to stdout (FD 1)"
echo "  - Verify command output appears on screen"
echo ""
echo "R9.4: Test basic commands"
echo "  - Try: echo hello"
echo "  - Try: ls"
echo "  - Try: ps"
echo ""
echo "R9.5: Shell can fork and exec other programs"
echo "  - Run any command from mellobox (echo, ls, ps, cat)"
echo "  - Verify it executes and returns to shell"
echo ""
echo "R9.6: Test Ctrl+C signal handling"
echo "  - Press Ctrl+C and verify shell handles it gracefully"
echo ""
echo "R9.7: Test Ctrl+D EOF handling"
echo "  - Press Ctrl+D and verify shell handles EOF"
echo ""
echo "========================================="
echo ""
echo "Press Enter to start QEMU..."
read

# Start QEMU
./tools/qemu.sh

echo ""
echo "========================================="
echo "Verification Checklist"
echo "========================================="
echo ""
echo "Did you verify the following?"
echo ""

# R9.1: Prompt display
echo -n "R9.1 - Shell displays prompt on startup? (y/n): "
read response
if [ "$response" = "y" ] || [ "$response" = "Y" ]; then
    print_result 0 "R9.1: Shell displays prompt"
else
    print_result 1 "R9.1: Shell displays prompt"
fi

# R9.2: stdin reading
echo -n "R9.2 - Shell reads from stdin (FD 0)? (y/n): "
read response
if [ "$response" = "y" ] || [ "$response" = "Y" ]; then
    print_result 0 "R9.2: Shell reads from stdin"
else
    print_result 1 "R9.2: Shell reads from stdin"
fi

# R9.3: stdout writing
echo -n "R9.3 - Shell writes to stdout (FD 1)? (y/n): "
read response
if [ "$response" = "y" ] || [ "$response" = "Y" ]; then
    print_result 0 "R9.3: Shell writes to stdout"
else
    print_result 1 "R9.3: Shell writes to stdout"
fi

# R9.4: Basic commands
echo -n "R9.4 - Basic commands work (echo, ls, ps)? (y/n): "
read response
if [ "$response" = "y" ] || [ "$response" = "Y" ]; then
    print_result 0 "R9.4: Basic commands work"
else
    print_result 1 "R9.4: Basic commands work"
fi

# R9.5: Fork and exec
echo -n "R9.5 - Shell can fork and exec programs? (y/n): "
read response
if [ "$response" = "y" ] || [ "$response" = "Y" ]; then
    print_result 0 "R9.5: Fork and exec work"
else
    print_result 1 "R9.5: Fork and exec work"
fi

# R9.6: Ctrl+C handling
echo -n "R9.6 - Ctrl+C signal handling works? (y/n): "
read response
if [ "$response" = "y" ] || [ "$response" = "Y" ]; then
    print_result 0 "R9.6: Ctrl+C handling"
else
    print_result 1 "R9.6: Ctrl+C handling"
fi

# R9.7: Ctrl+D EOF handling
echo -n "R9.7 - Ctrl+D EOF handling works? (y/n): "
read response
if [ "$response" = "y" ] || [ "$response" = "Y" ]; then
    print_result 0 "R9.7: Ctrl+D EOF handling"
else
    print_result 1 "R9.7: Ctrl+D EOF handling"
fi

echo ""
echo "========================================="
echo "Test Summary"
echo "========================================="
echo -e "Tests Passed: ${GREEN}${TESTS_PASSED}${NC}"
echo -e "Tests Failed: ${RED}${TESTS_FAILED}${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All shell integration tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed. Please review the failures above.${NC}"
    exit 1
fi
