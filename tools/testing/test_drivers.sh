#!/bin/bash

# Driver Subsystem Integration Test Script
# Automated QEMU tests for device driver functionality

echo "=========================================="
echo "Driver Subsystem Integration Test Suite"
echo "=========================================="

# Configuration
TEST_TIMEOUT=30  # Seconds to wait for tests
SMP_CPUS=2      # Default CPU count for SMP IRQ testing
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
    echo "  -smp N          Number of CPU cores for SMP IRQ testing (1-4, default: 2)"
    echo "  -timeout N      Test timeout in seconds (default: 30)"
    echo "  -h, --help      Show this help message"
    echo ""
    echo "Test Categories:"
    echo "  1. Driver manager initialization"
    echo "  2. Device discovery and enumeration"
    echo "  3. Keyboard driver (PS/2)"
    echo "  4. Serial driver (UART16550)"
    echo "  5. Block device driver (virtio-blk)"
    echo "  6. IRQ handling and distribution"
    echo "  7. SMP interrupt safety"
}

# Parse command line arguments
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

# Check if userland tools exist
MISSING_TOOLS=0
for tool in kbd_test serial_test disk_bench dmesg lsdev diskinfo irq_test; do
    if [ ! -f "iso_root/bin/$tool" ]; then
        echo -e "${YELLOW}Warning: $tool not found in iso_root/bin/${NC}"
        MISSING_TOOLS=$((MISSING_TOOLS + 1))
    fi
done

if [ $MISSING_TOOLS -gt 0 ]; then
    echo -e "${YELLOW}Some userland tools are missing. Run 'make userland' to build them.${NC}"
    echo ""
fi

# Function to run QEMU test
run_qemu_test() {
    local cpus=$1
    local test_name=$2
    
    echo -e "${BLUE}Running $test_name with $cpus CPU(s)...${NC}"
    
    # Clear output file
    > "$OUTPUT_FILE"
    
    # Run QEMU with virtio-blk device for disk testing
    qemu-system-x86_64 \
        -M q35 \
        -m 2G \
        -smp $cpus \
        -cdrom mellos.iso \
        -boot d \
        -drive file=/dev/zero,format=raw,if=none,id=disk0 \
        -device virtio-blk-pci,drive=disk0 \
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
    
    # Driver manager initialization tests
    check_test_result "$test_name" "Initializing driver subsystem" "Driver subsystem initialization started"
    check_test_result "$test_name" "Initializing IOAPIC routing" "IOAPIC routing initialized"
    check_test_result "$test_name" "Registering built-in drivers" "Built-in drivers registered"
    check_test_result "$test_name" "Driver initialization complete" "Driver subsystem fully initialized"
    
    # Device discovery tests
    check_test_result "$test_name" "Scanning.*bus" "Bus scanning performed"
    check_test_result "$test_name" "Scanning PS/2 bus" "PS/2 bus scanned"
    check_test_result "$test_name" "Scanning virtio bus" "Virtio bus scanned"
    check_test_result "$test_name" "Registering device.*ps2-keyboard" "PS/2 keyboard device detected"
    check_test_result "$test_name" "Registering device.*virtio-blk" "Virtio block device detected"
    
    # Driver probing and initialization tests
    check_test_result "$test_name" "Driver.*matched device" "Driver probing successful"
    check_test_result "$test_name" "Driver.*initialized successfully" "Driver initialization successful"
    
    # Keyboard driver tests
    check_test_result "$test_name" "Initializing PS/2 keyboard driver" "Keyboard driver initialization"
    check_test_result "$test_name" "PS/2 keyboard initialized" "Keyboard driver ready"
    check_test_result "$test_name" "Registered IRQ 1 handler" "Keyboard IRQ handler registered"
    
    # Serial driver tests
    check_test_result "$test_name" "Initializing UART16550 serial driver" "Serial driver initialization"
    check_test_result "$test_name" "Serial port COM1 initialized" "Serial port ready"
    
    # Block device driver tests
    check_test_result "$test_name" "Initializing virtio-blk driver" "Block driver initialization"
    check_test_result "$test_name" "virtio-blk initialized.*blocks" "Block device capacity detected"
    
    # IRQ handling tests
    check_test_result "$test_name" "Registered IRQ.*handler" "IRQ handlers registered"
    check_test_result "$test_name" "Handling IRQ.*on CPU" "IRQ handling working"
    
    # Device enumeration tests (lsdev)
    check_test_result "$test_name" "lsdev.*Device List" "Device enumeration tool working"
    check_test_result "$test_name" "ps2-keyboard.*PS2" "Keyboard listed in device tree"
    check_test_result "$test_name" "virtio-blk.*Virtio" "Block device listed in device tree"
    
    # Block device info tests (diskinfo)
    check_test_result "$test_name" "diskinfo.*Block Device Information" "Disk info tool working"
    check_test_result "$test_name" "Block count.*[0-9]" "Block count reported"
    check_test_result "$test_name" "Block size.*512" "Block size reported correctly"
    
    # Kernel log tests (dmesg)
    check_test_result "$test_name" "dmesg.*Kernel Log" "Kernel log tool working"
    check_test_result "$test_name" "Driver lifecycle.*register" "Driver registration logged"
    
    # SMP-specific IRQ tests (only for multi-CPU)
    if [ $SMP_CPUS -gt 1 ]; then
        check_test_result "$test_name" "IRQ.*CPU affinity.*[0-9]" "IRQ CPU affinity configured"
        check_test_result "$test_name" "irq_test.*IRQ Statistics" "IRQ test tool working"
        check_test_result "$test_name" "Interrupt distribution.*core" "IRQ distribution across cores"
        check_test_result "$test_name" "IOAPIC routing.*verified" "IOAPIC routing verified"
    fi
    
    # Error handling tests
    check_test_result "$test_name" "Driver.*probe.*failed.*continue" "Probe failure handled gracefully"
    check_test_result "$test_name" "No driver found.*warn" "Missing driver logged as warning"
    
    # System stability tests
    check_test_result "$test_name" "Init process monitoring" "System remained stable with drivers"
    
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
echo ""

echo "=========================================="
echo "Driver Subsystem Integration Tests"
echo "=========================================="

run_qemu_test $SMP_CPUS "Driver-Subsystem"
analyze_test_output "Driver-Subsystem"

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
    echo "Driver subsystem is working correctly:"
    echo "  - Driver manager and device tree"
    echo "  - Device discovery (PS/2, virtio)"
    echo "  - Keyboard driver (PS/2)"
    echo "  - Serial driver (UART16550)"
    echo "  - Block device driver (virtio-blk)"
    echo "  - IRQ handling and registration"
    if [ $SMP_CPUS -gt 1 ]; then
        echo "  - SMP interrupt safety and distribution"
    fi
    echo "  - Error handling and stability"
    
    EXIT_CODE=0
elif [ $TOTAL_TESTS -eq 0 ]; then
    echo -e "${YELLOW}⚠ NO TESTS COMPLETED${NC}"
    echo ""
    echo "Possible issues:"
    echo "  - Kernel failed to boot"
    echo "  - Driver subsystem not initialized"
    echo "  - Userland tools not available"
    echo ""
    echo "Check the detailed output above for more information."
    EXIT_CODE=2
else
    echo -e "${RED}✗ SOME TESTS FAILED${NC}"
    echo ""
    echo "Failed tests indicate issues with driver subsystem."
    echo "Check the detailed output above for specific failures."
    echo ""
    echo "Common issues:"
    echo "  - Driver registration problems"
    echo "  - Device detection failures"
    echo "  - IRQ handler issues"
    echo "  - IOAPIC routing problems"
    echo "  - Driver initialization errors"
    EXIT_CODE=1
fi

echo ""
echo "Detailed results saved to: $RESULTS_FILE"
echo "Full QEMU output saved to: $OUTPUT_FILE"

# Cleanup
# Note: Keep temp files for debugging, user can clean them manually

exit $EXIT_CODE
