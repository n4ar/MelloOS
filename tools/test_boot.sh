#!/bin/bash

# Test script to verify kernel boots and displays the expected message
# This runs QEMU with a timeout and captures output

echo "Testing MelloOS kernel boot..."
echo "================================"

# Check if ISO exists
if [ ! -f "mellos.iso" ]; then
    echo "Error: mellos.iso not found. Run 'make iso' first."
    exit 1
fi

# Create a temporary file for output
OUTPUT_FILE=$(mktemp)

echo "Starting QEMU in BIOS mode (will run for 3 seconds to capture boot output)..."
echo "Note: Limine supports both BIOS and UEFI boot modes."
echo ""

# Run QEMU in BIOS mode with proper headless settings
# Using background process with sleep for timeout
qemu-system-x86_64 \
    -M q35 \
    -m 2G \
    -cdrom mellos.iso \
    -boot d \
    -serial file:"$OUTPUT_FILE" \
    -display none \
    -no-reboot \
    -no-shutdown &

QEMU_PID=$!
sleep 3
kill -9 $QEMU_PID 2>/dev/null || true
wait $QEMU_PID 2>/dev/null || true

echo "QEMU output captured. Analyzing..."
echo ""
echo "--- Boot Output ---"
cat "$OUTPUT_FILE"
echo "--- End Output ---"
echo ""

# Check for success indicators
if grep -q "Hello from my kernel" "$OUTPUT_FILE"; then
    echo "✓ SUCCESS: Kernel booted and displayed the expected message!"
    rm "$OUTPUT_FILE"
    exit 0
else
    echo "✗ FAILED: Expected message not found in output"
    echo ""
    echo "Note: The kernel might still boot correctly in graphical mode."
    echo "Run 'make run' or './tools/qemu.sh' to see the graphical output."
    rm "$OUTPUT_FILE"
    exit 1
fi
