#!/bin/bash

# MelloOS QEMU Debug Script for SMP Issues
# This script launches QEMU with debugging features enabled to help diagnose
# triple fault issues during AP boot

echo "Starting MelloOS in QEMU DEBUG mode..."
echo ""

# Check if ISO exists
if [ ! -f "mellos.iso" ]; then
    echo "Error: mellos.iso not found. Run 'make iso' first."
    exit 1
fi

# Debug options
SMP_CPUS=2  # Start with just 1 AP for simpler debugging
MEMORY="2G"
LOG_FILE="qemu-debug.log"

echo "Debug Configuration:"
echo "  CPUs: $SMP_CPUS (1 BSP + 1 AP)"
echo "  Memory: $MEMORY"
echo "  Log file: $LOG_FILE"
echo ""
echo "QEMU Monitor Commands:"
echo "  info registers          - Show CPU registers"
echo "  info registers -a       - Show registers for all CPUs"
echo "  info mem                - Show page table mappings"
echo "  x/10i \$rip             - Disassemble at RIP"
echo "  gva2gpa <addr>          - Translate virtual to physical address"
echo ""
echo "Press Ctrl+A then C to access QEMU monitor"
echo ""

# Detect UEFI firmware
UEFI_MODE=0
if [ -f "/usr/local/share/qemu/edk2-x86_64-code.fd" ]; then
    UEFI_CODE="/usr/local/share/qemu/edk2-x86_64-code.fd"
    UEFI_MODE=1
elif [ -f "/opt/homebrew/share/qemu/edk2-x86_64-code.fd" ]; then
    UEFI_CODE="/opt/homebrew/share/qemu/edk2-x86_64-code.fd"
    UEFI_MODE=1
fi

# Launch QEMU with extensive debugging
if [ $UEFI_MODE -eq 1 ]; then
    echo "Booting in UEFI mode with debugging enabled..."
    qemu-system-x86_64 \
        -M q35 \
        -m $MEMORY \
        -smp $SMP_CPUS \
        -cdrom mellos.iso \
        -boot d \
        -serial stdio \
        -drive if=pflash,format=raw,readonly=on,file="$UEFI_CODE" \
        -monitor telnet:127.0.0.1:55555,server,nowait \
        -d int,cpu_reset,guest_errors \
        -D $LOG_FILE \
        -no-reboot \
        -no-shutdown
else
    echo "Booting in BIOS mode with debugging enabled..."
    qemu-system-x86_64 \
        -M q35 \
        -m $MEMORY \
        -smp $SMP_CPUS \
        -cdrom mellos.iso \
        -boot d \
        -serial stdio \
        -monitor telnet:127.0.0.1:55555,server,nowait \
        -d int,cpu_reset,guest_errors \
        -D $LOG_FILE \
        -no-reboot \
        -no-shutdown
fi

echo ""
echo "QEMU exited. Check $LOG_FILE for debug output."
