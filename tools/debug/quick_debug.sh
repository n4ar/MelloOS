#!/bin/bash
# Quick debug script for MelloOS
# Usage: ./tools/debug/quick_debug.sh [breakpoint]

set -e

BREAKPOINT=${1:-kernel_main}

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}=== MelloOS Quick Debug ===${NC}"
echo ""

# Check if QEMU is already running
if lsof -Pi :1234 -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo -e "${YELLOW}QEMU is already running on port 1234${NC}"
    echo "Connecting to existing QEMU instance..."
else
    echo -e "${BLUE}Starting QEMU in debug mode...${NC}"
    
    # Build first
    echo "Building kernel..."
    make clean > /dev/null 2>&1
    make build > /dev/null 2>&1
    make iso > /dev/null 2>&1
    
    # Start QEMU in background
    qemu-system-x86_64 \
        -cdrom melloos.iso \
        -m 512M \
        -smp 4 \
        -serial stdio \
        -no-reboot \
        -no-shutdown \
        -s \
        -S \
        -d int,cpu_reset \
        -D qemu.log \
        > /dev/null 2>&1 &
    
    QEMU_PID=$!
    echo "QEMU started (PID: $QEMU_PID)"
    
    # Wait for QEMU to be ready
    sleep 2
fi

# Create temporary GDB script
GDB_SCRIPT=$(mktemp)
cat > "$GDB_SCRIPT" << EOF
target remote localhost:1234
break $BREAKPOINT
continue
EOF

echo -e "${GREEN}Connecting GDB with breakpoint at: $BREAKPOINT${NC}"
echo ""

# Start GDB
gdb -x "$GDB_SCRIPT" kernel/target/x86_64-unknown-none/debug/kernel

# Cleanup
rm -f "$GDB_SCRIPT"

echo ""
echo -e "${GREEN}Debug session ended${NC}"
