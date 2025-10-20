#!/bin/bash

# MelloOS QEMU Launch Script for SMP testing with 2 CPUs
# This is a legacy script - use tools/qemu-test-smp2.sh for enhanced testing

echo "Note: This is a legacy script. For enhanced SMP testing, use:"
echo "  ./tools/qemu-test-smp2.sh"
echo ""
echo "Starting MelloOS in QEMU with 2 CPUs..."

# Redirect to the main script with 2 CPU preset
exec ./tools/qemu.sh -preset smp2
