#!/bin/bash

# MelloOS QEMU Launch Script
# Boots the kernel in QEMU with appropriate settings

echo "Starting MelloOS in QEMU..."
echo ""

# Check if ISO exists
if [ ! -f "mellos.iso" ]; then
    echo "Error: mellos.iso not found. Run 'make iso' first."
    exit 1
fi

# Detect UEFI firmware location based on OS
# Limine supports both BIOS and UEFI boot, so we'll try UEFI first
UEFI_MODE=0

if [ -f "/usr/share/ovmf/OVMF.fd" ]; then
    UEFI_BIOS="/usr/share/ovmf/OVMF.fd"
    UEFI_MODE=1
elif [ -f "/usr/share/edk2/ovmf/OVMF_CODE.fd" ]; then
    UEFI_BIOS="/usr/share/edk2/ovmf/OVMF_CODE.fd"
    UEFI_MODE=1
elif [ -f "/usr/local/share/qemu/edk2-x86_64-code.fd" ]; then
    # macOS with Homebrew - need to use drive format for proper UEFI
    UEFI_CODE="/usr/local/share/qemu/edk2-x86_64-code.fd"
    UEFI_MODE=2
elif [ -f "/opt/homebrew/share/qemu/edk2-x86_64-code.fd" ]; then
    # macOS with Homebrew (ARM)
    UEFI_CODE="/opt/homebrew/share/qemu/edk2-x86_64-code.fd"
    UEFI_MODE=2
fi

# Launch QEMU with appropriate settings
if [ $UEFI_MODE -eq 1 ]; then
    echo "Booting in UEFI mode..."
    qemu-system-x86_64 \
        -M q35 \
        -m 2G \
        -cdrom mellos.iso \
        -boot d \
        -serial stdio \
        -bios "$UEFI_BIOS"
elif [ $UEFI_MODE -eq 2 ]; then
    echo "Booting in UEFI mode (EDK2)..."
    qemu-system-x86_64 \
        -M q35 \
        -m 2G \
        -cdrom mellos.iso \
        -boot d \
        -serial stdio \
        -drive if=pflash,format=raw,readonly=on,file="$UEFI_CODE"
else
    echo "Booting in BIOS mode (UEFI firmware not found)..."
    echo "Note: Limine supports both BIOS and UEFI boot modes."
    qemu-system-x86_64 \
        -M q35 \
        -m 2G \
        -cdrom mellos.iso \
        -boot d \
        -serial stdio
fi
