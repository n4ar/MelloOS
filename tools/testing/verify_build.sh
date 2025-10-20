#!/bin/bash

# Comprehensive build and boot verification script
# Tests all aspects of the build process

echo "=========================================="
echo "MelloOS Build & Boot Verification"
echo "=========================================="
echo ""

FAILED=0

# Test 1: Verify kernel binary exists
echo "Test 1: Checking kernel binary..."
if [ -f "kernel/target/x86_64-unknown-none/release/mellos-kernel" ]; then
    echo "✓ Kernel binary exists"
    KERNEL_SIZE=$(ls -lh kernel/target/x86_64-unknown-none/release/mellos-kernel | awk '{print $5}')
    echo "  Size: $KERNEL_SIZE"
else
    echo "✗ Kernel binary not found"
    FAILED=1
fi
echo ""

# Test 2: Verify kernel contains expected message
echo "Test 2: Checking kernel contains expected message..."
if strings kernel/target/x86_64-unknown-none/release/mellos-kernel | grep -q "Hello from MelloOS"; then
    echo "✓ Kernel contains 'Hello from MelloOS ✨' message"
    echo "  Found: $(strings kernel/target/x86_64-unknown-none/release/mellos-kernel | grep 'Hello from MelloOS')"
else
    echo "✗ Expected message not found in kernel binary"
    FAILED=1
fi
echo ""

# Test 3: Verify ISO image exists
echo "Test 3: Checking ISO image..."
if [ -f "mellos.iso" ]; then
    echo "✓ ISO image exists"
    ISO_SIZE=$(ls -lh mellos.iso | awk '{print $5}')
    echo "  Size: $ISO_SIZE"
else
    echo "✗ ISO image not found"
    FAILED=1
fi
echo ""

# Test 4: Verify ISO contains kernel
echo "Test 4: Checking ISO contents..."
if command -v xorriso &> /dev/null; then
    if xorriso -indev mellos.iso -find 2>/dev/null | grep -q "kernel.elf"; then
        echo "✓ ISO contains kernel.elf"
    else
        echo "✗ kernel.elf not found in ISO"
        FAILED=1
    fi
else
    echo "⚠ xorriso not available, skipping ISO content check"
fi
echo ""

# Test 5: Verify QEMU is available
echo "Test 5: Checking QEMU availability..."
if command -v qemu-system-x86_64 &> /dev/null; then
    echo "✓ QEMU is installed"
    QEMU_VERSION=$(qemu-system-x86_64 --version | head -n1)
    echo "  Version: $QEMU_VERSION"
else
    echo "✗ QEMU not found"
    FAILED=1
fi
echo ""

# Test 6: Verify Limine bootloader files
echo "Test 6: Checking Limine bootloader..."
if [ -d "limine" ]; then
    echo "✓ Limine directory exists"
    if [ -f "limine/limine-bios.sys" ]; then
        echo "✓ Limine BIOS bootloader found"
    else
        echo "✗ Limine BIOS bootloader missing"
        FAILED=1
    fi
    if [ -f "limine/BOOTX64.EFI" ]; then
        echo "✓ Limine UEFI bootloader found"
    else
        echo "✗ Limine UEFI bootloader missing"
        FAILED=1
    fi
else
    echo "✗ Limine directory not found"
    FAILED=1
fi
echo ""

# Test 7: Verify configuration files
echo "Test 7: Checking configuration files..."
if [ -f "boot/limine.conf" ] || [ -f "boot/limine.cfg" ]; then
    echo "✓ Limine configuration exists"
    CONFIG_FILE=""
    if [ -f "boot/limine.conf" ]; then
        CONFIG_FILE="boot/limine.conf"
    else
        CONFIG_FILE="boot/limine.cfg"
    fi
    if grep -q "MelloOS" "$CONFIG_FILE"; then
        echo "✓ Configuration contains MelloOS entry"
    else
        echo "✗ MelloOS entry not found in configuration"
        FAILED=1
    fi
else
    echo "✗ Limine configuration not found"
    FAILED=1
fi
echo ""

# Summary
echo "=========================================="
if [ $FAILED -eq 0 ]; then
    echo "✓ ALL TESTS PASSED"
    echo ""
    echo "Build verification successful!"
    echo ""
    echo "To test the kernel visually, run:"
    echo "  make run"
    echo ""
    echo "This will launch QEMU with a graphical window where you"
    echo "should see 'Hello from my kernel ✨' displayed on screen."
    exit 0
else
    echo "✗ SOME TESTS FAILED"
    echo ""
    echo "Please fix the issues above before proceeding."
    exit 1
fi
