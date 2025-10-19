# MelloOS Makefile
# Build automation for kernel compilation and ISO creation

# Configuration variables
KERNEL_DIR := kernel
USERSPACE_DIR := $(KERNEL_DIR)/userspace/init
KERNEL_BINARY := $(KERNEL_DIR)/target/x86_64-unknown-none/release/mellos-kernel
INIT_BINARY := $(USERSPACE_DIR)/target/x86_64-unknown-none/release/init
BUILD_MODE := release
ISO_ROOT := iso_root
ISO_NAME := mellos.iso

# Limine configuration
LIMINE_DIR := limine
LIMINE_REPO := https://github.com/limine-bootloader/limine.git
LIMINE_BRANCH := v8.x-binary

# Cargo configuration
CARGO := cargo
CARGO_BUILD_FLAGS := --release

# Colors for output
COLOR_RESET := \033[0m
COLOR_GREEN := \033[32m
COLOR_BLUE := \033[34m
COLOR_YELLOW := \033[33m

.PHONY: all build clean help iso limine run userspace

# Default target
all: build

# Build userspace init process
userspace:
	@echo "$(COLOR_BLUE)Building userspace init process...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR) && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_GREEN)✓ Userspace init built successfully!$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Binary location: $(INIT_BINARY)$(COLOR_RESET)"

# Build the kernel
build: userspace
	@echo "$(COLOR_BLUE)Cleaning previous build...$(COLOR_RESET)"
	@cd $(KERNEL_DIR) && $(CARGO) clean
	@echo "$(COLOR_BLUE)Building MelloOS kernel...$(COLOR_RESET)"
	@cd $(KERNEL_DIR) && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_GREEN)✓ Kernel built successfully!$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Binary location: $(KERNEL_BINARY)$(COLOR_RESET)"

# Download and setup Limine bootloader
limine:
	@if [ ! -d "$(LIMINE_DIR)" ]; then \
		echo "$(COLOR_BLUE)Downloading Limine bootloader...$(COLOR_RESET)"; \
		git clone $(LIMINE_REPO) --branch=$(LIMINE_BRANCH) --depth=1 $(LIMINE_DIR); \
		echo "$(COLOR_GREEN)✓ Limine downloaded successfully!$(COLOR_RESET)"; \
	else \
		echo "$(COLOR_YELLOW)Limine already exists, skipping download$(COLOR_RESET)"; \
	fi
	@if [ ! -f "$(LIMINE_DIR)/limine" ]; then \
		echo "$(COLOR_BLUE)Building Limine executable...$(COLOR_RESET)"; \
		$(MAKE) -C $(LIMINE_DIR); \
		echo "$(COLOR_GREEN)✓ Limine built successfully!$(COLOR_RESET)"; \
	fi

# Create bootable ISO image
iso: build limine
	@echo "$(COLOR_BLUE)Creating ISO image...$(COLOR_RESET)"
	
	# Check if limine config exists
	@if [ ! -f "boot/limine.conf" ] && [ ! -f "boot/limine.cfg" ]; then \
		echo "$(COLOR_YELLOW)Warning: boot/limine.conf or boot/limine.cfg not found. Please create it first.$(COLOR_RESET)"; \
		exit 1; \
	fi
	
	# Create ISO directory structure
	@mkdir -p $(ISO_ROOT)/boot
	@mkdir -p $(ISO_ROOT)/boot/limine
	@mkdir -p $(ISO_ROOT)/EFI/BOOT
	
	# Copy kernel binary
	@echo "$(COLOR_YELLOW)Copying kernel binary...$(COLOR_RESET)"
	@cp $(KERNEL_BINARY) $(ISO_ROOT)/boot/kernel.elf
	
	# Copy Limine bootloader files
	@echo "$(COLOR_YELLOW)Copying Limine bootloader files...$(COLOR_RESET)"
	@cp $(LIMINE_DIR)/limine-bios.sys $(ISO_ROOT)/boot/limine/
	@cp $(LIMINE_DIR)/limine-bios-cd.bin $(ISO_ROOT)/boot/limine/
	@cp $(LIMINE_DIR)/limine-uefi-cd.bin $(ISO_ROOT)/boot/limine/
	@cp $(LIMINE_DIR)/BOOTX64.EFI $(ISO_ROOT)/EFI/BOOT/
	@cp $(LIMINE_DIR)/BOOTIA32.EFI $(ISO_ROOT)/EFI/BOOT/
	
	# Copy Limine configuration
	@echo "$(COLOR_YELLOW)Copying bootloader configuration...$(COLOR_RESET)"
	@if [ -f "boot/limine.conf" ]; then \
		cp boot/limine.conf $(ISO_ROOT)/boot/limine/; \
	elif [ -f "boot/limine.cfg" ]; then \
		cp boot/limine.cfg $(ISO_ROOT)/boot/limine/; \
	fi
	
	# Create ISO image with xorriso
	@echo "$(COLOR_YELLOW)Creating ISO with xorriso...$(COLOR_RESET)"
	@xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
		-no-emul-boot -boot-load-size 4 -boot-info-table \
		--efi-boot boot/limine/limine-uefi-cd.bin \
		-efi-boot-part --efi-boot-image --protective-msdos-label \
		$(ISO_ROOT) -o $(ISO_NAME) 2>/dev/null
	
	# Install Limine bootloader to ISO
	@echo "$(COLOR_YELLOW)Installing Limine bootloader...$(COLOR_RESET)"
	@$(LIMINE_DIR)/limine bios-install $(ISO_NAME) 2>/dev/null
	
	@echo "$(COLOR_GREEN)✓ ISO image created successfully!$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)ISO location: $(ISO_NAME)$(COLOR_RESET)"

# Run kernel in QEMU
run: iso
	@echo "$(COLOR_BLUE)Starting QEMU...$(COLOR_RESET)"
	@./tools/qemu.sh

# Clean build artifacts
clean:
	@echo "$(COLOR_BLUE)Cleaning build artifacts...$(COLOR_RESET)"
	@cd $(KERNEL_DIR) && $(CARGO) clean
	@cd $(USERSPACE_DIR) && $(CARGO) clean
	@rm -rf $(ISO_ROOT)
	@rm -f $(ISO_NAME)
	@rm -rf $(LIMINE_DIR)
	@echo "$(COLOR_GREEN)✓ Clean complete!$(COLOR_RESET)"

# Help target
help:
	@echo "MelloOS Build System"
	@echo ""
	@echo "Available targets:"
	@echo "  make build    - Build the kernel (default)"
	@echo "  make iso      - Create bootable ISO image"
	@echo "  make run      - Build ISO and run kernel in QEMU"
	@echo "  make limine   - Download Limine bootloader"
	@echo "  make clean    - Clean build artifacts and ISO files"
	@echo "  make help     - Show this help message"
	@echo ""
	@echo "Configuration:"
	@echo "  KERNEL_DIR    = $(KERNEL_DIR)"
	@echo "  BUILD_MODE    = $(BUILD_MODE)"
	@echo "  ISO_NAME      = $(ISO_NAME)"
