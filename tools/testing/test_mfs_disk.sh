#!/bin/bash
# Test script for MFS Disk filesystem mounting

set -e

echo "=== MFS Disk Filesystem Test ==="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Step 1: Building kernel and userspace...${NC}"
make clean
make build
make iso

echo ""
echo -e "${YELLOW}Step 2: Creating test disk image...${NC}"
# Create a 64MB disk image
dd if=/dev/zero of=test_disk.img bs=1M count=64 2>/dev/null
echo "Created test_disk.img (64MB)"

echo ""
echo -e "${YELLOW}Step 3: Starting QEMU with test disk...${NC}"
echo "Note: This will start QEMU. You can test mfs_disk manually."
echo ""
echo "Commands to try in MelloOS:"
echo "  1. Check block devices: lsdev"
echo "  2. Format disk: mkfs.mfs /dev/vda"
echo "  3. Create mount point: mkdir /data"
echo "  4. Mount filesystem: mount -t mfs_disk /dev/vda /data"
echo "  5. Test operations: cd /data && echo test > file.txt && cat file.txt"
echo ""

# Start QEMU with additional disk
qemu-system-x86_64 \
    -cdrom mellos.iso \
    -m 512M \
    -smp 2 \
    -serial stdio \
    -drive file=test_disk.img,if=none,id=disk0,format=raw \
    -device virtio-blk-pci,drive=disk0 \
    -no-reboot \
    -no-shutdown

echo ""
echo -e "${GREEN}Test complete!${NC}"
