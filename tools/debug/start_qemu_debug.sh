#!/bin/bash
# Start QEMU with GDB server for debugging
# Usage: ./tools/debug/start_qemu_debug.sh

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting QEMU with GDB server...${NC}"
echo -e "${YELLOW}GDB will listen on localhost:1234${NC}"
echo ""
echo "To connect from GDB:"
echo "  gdb kernel/target/x86_64-unknown-none/debug/kernel"
echo "  (gdb) target remote localhost:1234"
echo ""
echo "Or use VS Code debugger (F5)"
echo ""

# Build first
echo -e "${GREEN}Building kernel...${NC}"
make clean
make build
make iso

# Start QEMU with GDB server
# -s: shorthand for -gdb tcp::1234
# -S: freeze CPU at startup (wait for GDB to connect)
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
    -D qemu.log

echo -e "${GREEN}QEMU stopped${NC}"
