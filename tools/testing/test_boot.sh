#!/bin/bash

# Test script to verify kernel boots and displays the expected message
# This runs QEMU with a timeout and captures output
# Supports both single-core and SMP testing

echo "Testing MelloOS kernel boot..."
echo "================================"

# Parse command line arguments
SMP_CPUS=1  # Default to single CPU for basic boot test
TEST_TIMEOUT=5  # Seconds to wait for boot

show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -smp N          Number of CPU cores for SMP testing (1-4, default: 1)"
    echo "  -timeout N      Boot timeout in seconds (default: 5)"
    echo "  -h, --help      Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                  # Basic single-CPU boot test"
    echo "  $0 -smp 2           # Test SMP boot with 2 CPUs"
    echo "  $0 -smp 4 -timeout 10  # Test 4-CPU SMP with longer timeout"
}

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
    echo "Error: mellos.iso not found. Run 'make iso' first."
    exit 1
fi

# Create a temporary file for output
OUTPUT_FILE=$(mktemp)

echo "Configuration:"
echo "  CPUs: $SMP_CPUS"
echo "  Timeout: ${TEST_TIMEOUT}s"
echo ""

if [ $SMP_CPUS -eq 1 ]; then
    echo "Starting single-CPU boot test..."
else
    echo "Starting SMP boot test with $SMP_CPUS CPUs..."
fi

echo "Note: Limine supports both BIOS and UEFI boot modes."
echo ""

# Run QEMU in BIOS mode with proper headless settings
qemu-system-x86_64 \
    -M q35 \
    -m 2G \
    -smp $SMP_CPUS \
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

echo "QEMU output captured. Analyzing..."
echo ""
echo "--- Boot Output ---"
cat "$OUTPUT_FILE"
echo "--- End Output ---"
echo ""

# Check for success indicators
SUCCESS=0

if grep -q "Hello from my kernel" "$OUTPUT_FILE"; then
    echo "✓ Basic kernel boot successful"
    SUCCESS=1
fi

if [ $SMP_CPUS -gt 1 ]; then
    # Additional SMP-specific checks
    if grep -q "SMP.*CPUs detected" "$OUTPUT_FILE"; then
        echo "✓ SMP CPU detection working"
    else
        echo "⚠ SMP CPU detection not found"
        SUCCESS=0
    fi
    
    if grep -q "BSP online" "$OUTPUT_FILE"; then
        echo "✓ BSP (Bootstrap Processor) initialized"
    else
        echo "⚠ BSP initialization not confirmed"
    fi
    
    if grep -q "AP.*online" "$OUTPUT_FILE"; then
        echo "✓ Application Processors brought online"
    else
        echo "⚠ AP initialization not found"
        SUCCESS=0
    fi
    
    if grep -q "SCHED.*core[1-9]" "$OUTPUT_FILE"; then
        echo "✓ Multi-core task scheduling detected"
    else
        echo "⚠ Multi-core scheduling not confirmed"
    fi
fi

rm "$OUTPUT_FILE"

if [ $SUCCESS -eq 1 ]; then
    if [ $SMP_CPUS -eq 1 ]; then
        echo ""
        echo "✓ SUCCESS: Single-CPU kernel boot test passed!"
    else
        echo ""
        echo "✓ SUCCESS: SMP kernel boot test with $SMP_CPUS CPUs passed!"
    fi
    exit 0
else
    echo ""
    echo "✗ FAILED: Boot test did not meet all criteria"
    echo ""
    echo "Troubleshooting:"
    echo "  - Run './tools/qemu.sh -smp $SMP_CPUS' for interactive testing"
    echo "  - Check kernel logs for detailed error information"
    echo "  - Verify SMP implementation is complete"
    exit 1
fi
