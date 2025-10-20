#!/bin/bash

# MelloOS SMP Test Script - 2 CPUs
# Quick test configuration for 2-CPU SMP functionality

echo "MelloOS SMP Test: 2 CPUs with KVM"
echo "=================================="
echo ""

# Check if ISO exists
if [ ! -f "mellos.iso" ]; then
    echo "Error: mellos.iso not found. Run 'make iso' first."
    exit 1
fi

# Check if KVM is available
KVM_AVAILABLE=0
if [ -r /dev/kvm ]; then
    KVM_AVAILABLE=1
    echo "✓ KVM acceleration available"
else
    echo "⚠ KVM not available - tests will run slower"
fi

echo "Configuration:"
echo "  CPUs: 2"
echo "  KVM: ${KVM_AVAILABLE:+enabled}"
echo "  Memory: 2GB"
echo ""
echo "Expected SMP boot sequence:"
echo "  1. BSP (Core 0) initializes"
echo "  2. AP (Core 1) brought online"
echo "  3. Tasks distributed across cores"
echo ""

# Launch with optimal settings for 2-CPU testing
if [ $KVM_AVAILABLE -eq 1 ]; then
    exec ./tools/qemu.sh -smp 2 -enable-kvm
else
    exec ./tools/qemu.sh -smp 2
fi