#!/bin/bash

# MelloOS QEMU Launch Script
# Boots the kernel in QEMU with appropriate settings for SMP testing

echo "Starting MelloOS in QEMU..."
echo ""

# Check if ISO exists
if [ ! -f "mellos.iso" ]; then
    echo "Error: mellos.iso not found. Run 'make iso' first."
    exit 1
fi

# Parse command line arguments
SMP_CPUS=4  # Default to 4 CPUs for SMP testing
ENABLE_KVM=""
PRESET=""

show_help() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -smp N          Number of CPU cores (1-8, default: 4)"
    echo "  -enable-kvm     Enable KVM acceleration (recommended for faster testing)"
    echo "  -preset NAME    Use predefined configuration:"
    echo "                    smp2    - 2 CPUs with KVM"
    echo "                    smp4    - 4 CPUs with KVM"
    echo "                    debug   - 2 CPUs with debugging enabled"
    echo "                    single  - 1 CPU (disable SMP)"
    echo "  -h, --help      Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                      # Default: 4 CPUs"
    echo "  $0 -smp 2 -enable-kvm   # 2 CPUs with KVM"
    echo "  $0 -preset smp2         # Quick 2-CPU test"
    echo "  $0 -preset debug        # Debug mode with 2 CPUs"
}

while [[ $# -gt 0 ]]; do
    case $1 in
        -smp)
            if [[ "$2" =~ ^[1-8]$ ]]; then
                SMP_CPUS="$2"
                shift 2
            else
                echo "Error: SMP CPU count must be between 1 and 8"
                exit 1
            fi
            ;;
        -enable-kvm)
            ENABLE_KVM="-enable-kvm"
            shift
            ;;
        -preset)
            PRESET="$2"
            shift 2
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

# Apply preset configurations
case $PRESET in
    smp2)
        SMP_CPUS=2
        ENABLE_KVM="-enable-kvm"
        echo "Using preset: 2 CPUs with KVM acceleration"
        ;;
    smp4)
        SMP_CPUS=4
        ENABLE_KVM="-enable-kvm"
        echo "Using preset: 4 CPUs with KVM acceleration"
        ;;
    debug)
        SMP_CPUS=2
        echo "Using preset: Debug mode with 2 CPUs"
        ;;
    single)
        SMP_CPUS=1
        echo "Using preset: Single CPU (SMP disabled)"
        ;;
    "")
        # No preset, use command line options
        ;;
    *)
        echo "Error: Unknown preset '$PRESET'"
        echo "Available presets: smp2, smp4, debug, single"
        exit 1
        ;;
esac

echo "Configuration:"
echo "  CPUs: $SMP_CPUS"
echo "  KVM: ${ENABLE_KVM:-disabled}"
echo ""

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
        -smp $SMP_CPUS \
        -cdrom mellos.iso \
        -boot d \
        -serial stdio \
        -bios "$UEFI_BIOS" \
        $ENABLE_KVM
elif [ $UEFI_MODE -eq 2 ]; then
    echo "Booting in UEFI mode (EDK2)..."
    qemu-system-x86_64 \
        -M q35 \
        -m 2G \
        -smp $SMP_CPUS \
        -cdrom mellos.iso \
        -boot d \
        -serial stdio \
        -drive if=pflash,format=raw,readonly=on,file="$UEFI_CODE" \
        $ENABLE_KVM
else
    echo "Booting in BIOS mode (UEFI firmware not found)..."
    echo "Note: Limine supports both BIOS and UEFI boot modes."
    qemu-system-x86_64 \
        -M q35 \
        -m 2G \
        -smp $SMP_CPUS \
        -cdrom mellos.iso \
        -boot d \
        -serial stdio \
        $ENABLE_KVM
fi
