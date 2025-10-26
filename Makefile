# MelloOS Makefile
# Build automation for kernel compilation and ISO creation

# Configuration variables
KERNEL_DIR := kernel
USERSPACE_DIR := $(KERNEL_DIR)/userspace
KERNEL_BINARY := $(KERNEL_DIR)/target/x86_64-unknown-none/release/mellos-kernel
INIT_BINARY := $(USERSPACE_DIR)/init/target/x86_64-unknown-none/release/init
MELLO_TERM_BINARY := $(USERSPACE_DIR)/mello-term/target/x86_64-unknown-none/release/mello-term
MELLO_SH_BINARY := $(USERSPACE_DIR)/mello-sh/target/x86_64-unknown-none/release/mello-sh
MELLOBOX_BINARY := $(USERSPACE_DIR)/mellobox/target/x86_64-unknown-none/release/mellobox
KBD_TEST_BINARY := $(USERSPACE_DIR)/kbd_test/target/x86_64-unknown-none/release/kbd_test
SERIAL_TEST_BINARY := $(USERSPACE_DIR)/serial_test/target/x86_64-unknown-none/release/serial_test
DISK_BENCH_BINARY := $(USERSPACE_DIR)/disk_bench/target/x86_64-unknown-none/release/disk_bench
DMESG_BINARY := $(USERSPACE_DIR)/dmesg/target/x86_64-unknown-none/release/dmesg
LSDEV_BINARY := $(USERSPACE_DIR)/lsdev/target/x86_64-unknown-none/release/lsdev
DISKINFO_BINARY := $(USERSPACE_DIR)/diskinfo/target/x86_64-unknown-none/release/diskinfo
IRQ_TEST_BINARY := $(USERSPACE_DIR)/irq_test/target/x86_64-unknown-none/release/irq_test
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

# sccache configuration (optional - speeds up compilation)
# Set RUSTC_WRAPPER=sccache to enable caching
# Install: brew install sccache (macOS) or cargo install sccache
ifdef SCCACHE
	export RUSTC_WRAPPER := sccache
	SCCACHE_INFO := (with sccache)
else
	SCCACHE_INFO :=
endif

# Colors for output
COLOR_RESET := \033[0m
COLOR_GREEN := \033[32m
COLOR_BLUE := \033[34m
COLOR_YELLOW := \033[33m

.PHONY: all build build-dev clean help iso iso-dev limine run run-dev userspace userspace-dev symlinks

# Default target
all: build

# Build userspace programs (release mode)
userspace:
	@echo "$(COLOR_BLUE)Building userspace programs (release)...$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Building init...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/init && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_YELLOW)Building mello-term...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/mello-term && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_YELLOW)Building mello-sh...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/mello-sh && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_YELLOW)Building mellobox...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/mellobox && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_YELLOW)Building kbd_test...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/kbd_test && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_YELLOW)Building serial_test...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/serial_test && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_YELLOW)Building disk_bench...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/disk_bench && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_YELLOW)Building dmesg...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/dmesg && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_YELLOW)Building lsdev...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/lsdev && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_YELLOW)Building diskinfo...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/diskinfo && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_YELLOW)Building irq_test...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/irq_test && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_GREEN)✓ All userspace programs built successfully!$(COLOR_RESET)"

# Build userspace programs in dev mode (fast compilation)
userspace-dev:
	@echo "$(COLOR_BLUE)Building userspace programs (dev mode - fast)...$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Building init...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/init && $(CARGO) build
	@echo "$(COLOR_YELLOW)Building mello-term...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/mello-term && $(CARGO) build
	@echo "$(COLOR_YELLOW)Building mello-sh...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/mello-sh && $(CARGO) build
	@echo "$(COLOR_YELLOW)Building mellobox...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/mellobox && $(CARGO) build
	@echo "$(COLOR_YELLOW)Building kbd_test...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/kbd_test && $(CARGO) build
	@echo "$(COLOR_YELLOW)Building serial_test...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/serial_test && $(CARGO) build
	@echo "$(COLOR_YELLOW)Building disk_bench...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/disk_bench && $(CARGO) build
	@echo "$(COLOR_YELLOW)Building dmesg...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/dmesg && $(CARGO) build
	@echo "$(COLOR_YELLOW)Building lsdev...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/lsdev && $(CARGO) build
	@echo "$(COLOR_YELLOW)Building diskinfo...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/diskinfo && $(CARGO) build
	@echo "$(COLOR_YELLOW)Building irq_test...$(COLOR_RESET)"
	@cd $(USERSPACE_DIR)/irq_test && $(CARGO) build
	@echo "$(COLOR_GREEN)✓ All userspace programs built successfully (dev mode)!$(COLOR_RESET)"

# Create symlinks for mellobox utilities
symlinks:
	@echo "$(COLOR_BLUE)Creating symlinks for mellobox utilities...$(COLOR_RESET)"
	@mkdir -p $(ISO_ROOT)/bin
	@if [ -f "$(MELLOBOX_BINARY)" ]; then \
		cp $(MELLOBOX_BINARY) $(ISO_ROOT)/bin/mellobox; \
		for util in ls cp mv rm cat grep ps kill mkdir touch echo pwd true false; do \
			ln -sf mellobox $(ISO_ROOT)/bin/$$util; \
		done; \
		echo "$(COLOR_GREEN)✓ Symlinks created successfully!$(COLOR_RESET)"; \
	else \
		echo "$(COLOR_YELLOW)Warning: mellobox binary not found, skipping symlinks$(COLOR_RESET)"; \
	fi

# Build the kernel (release mode - optimized but slow)
build: userspace
	@echo "$(COLOR_BLUE)Cleaning previous build...$(COLOR_RESET)"
	@cd $(KERNEL_DIR) && $(CARGO) clean
	@echo "$(COLOR_BLUE)Building MelloOS kernel (release mode)...$(COLOR_RESET)"
	@cd $(KERNEL_DIR) && $(CARGO) build $(CARGO_BUILD_FLAGS)
	@echo "$(COLOR_GREEN)✓ Kernel built successfully!$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Binary location: $(KERNEL_BINARY)$(COLOR_RESET)"

# Build the kernel in dev mode (fast compilation, no optimization)
build-dev: userspace-dev
	@echo "$(COLOR_BLUE)Cleaning previous build...$(COLOR_RESET)"
	@cd $(KERNEL_DIR) && $(CARGO) clean
	@echo "$(COLOR_BLUE)Building MelloOS kernel (dev mode - fast compile)...$(COLOR_RESET)"
	@cd $(KERNEL_DIR) && $(CARGO) build
	@echo "$(COLOR_GREEN)✓ Kernel built successfully (dev mode)!$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)Binary location: $(KERNEL_DIR)/target/x86_64-unknown-none/debug/mellos-kernel$(COLOR_RESET)"

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
iso: build limine symlinks
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
	@mkdir -p $(ISO_ROOT)/bin
	@mkdir -p $(ISO_ROOT)/dev
	@mkdir -p $(ISO_ROOT)/proc
	
	# Copy kernel binary
	@echo "$(COLOR_YELLOW)Copying kernel binary...$(COLOR_RESET)"
	@cp $(KERNEL_BINARY) $(ISO_ROOT)/boot/kernel.elf
	
	# Copy userspace binaries
	@echo "$(COLOR_YELLOW)Copying userspace binaries...$(COLOR_RESET)"
	@if [ -f "$(INIT_BINARY)" ]; then cp $(INIT_BINARY) $(ISO_ROOT)/bin/init; fi
	@if [ -f "$(MELLO_TERM_BINARY)" ]; then cp $(MELLO_TERM_BINARY) $(ISO_ROOT)/bin/mello-term; fi
	@if [ -f "$(MELLO_SH_BINARY)" ]; then cp $(MELLO_SH_BINARY) $(ISO_ROOT)/bin/mello-sh; fi
	@if [ -f "$(MELLOBOX_BINARY)" ]; then \
		cp $(MELLOBOX_BINARY) $(ISO_ROOT)/bin/mellobox; \
		for util in ls cp mv rm cat grep ps kill mkdir touch echo pwd true false; do \
			ln -sf mellobox $(ISO_ROOT)/bin/$$util 2>/dev/null || true; \
		done; \
	fi
	@if [ -f "$(KBD_TEST_BINARY)" ]; then cp $(KBD_TEST_BINARY) $(ISO_ROOT)/bin/kbd_test; fi
	@if [ -f "$(SERIAL_TEST_BINARY)" ]; then cp $(SERIAL_TEST_BINARY) $(ISO_ROOT)/bin/serial_test; fi
	@if [ -f "$(DISK_BENCH_BINARY)" ]; then cp $(DISK_BENCH_BINARY) $(ISO_ROOT)/bin/disk_bench; fi
	@if [ -f "$(DMESG_BINARY)" ]; then cp $(DMESG_BINARY) $(ISO_ROOT)/bin/dmesg; fi
	@if [ -f "$(LSDEV_BINARY)" ]; then cp $(LSDEV_BINARY) $(ISO_ROOT)/bin/lsdev; fi
	@if [ -f "$(DISKINFO_BINARY)" ]; then cp $(DISKINFO_BINARY) $(ISO_ROOT)/bin/diskinfo; fi
	@if [ -f "$(IRQ_TEST_BINARY)" ]; then cp $(IRQ_TEST_BINARY) $(ISO_ROOT)/bin/irq_test; fi
	
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

# Create bootable ISO image (dev mode - fast build)
iso-dev: build-dev limine symlinks
	@echo "$(COLOR_BLUE)Creating ISO image (dev mode)...$(COLOR_RESET)"
	
	# Create ISO directory structure
	@mkdir -p $(ISO_ROOT)/boot
	@mkdir -p $(ISO_ROOT)/boot/limine
	@mkdir -p $(ISO_ROOT)/EFI/BOOT
	@mkdir -p $(ISO_ROOT)/bin
	
	# Copy kernel binary (dev mode)
	@echo "$(COLOR_YELLOW)Copying kernel binary (dev)...$(COLOR_RESET)"
	@cp $(KERNEL_DIR)/target/x86_64-unknown-none/debug/mellos-kernel $(ISO_ROOT)/boot/kernel.elf
	
	# Copy userspace binaries (dev mode)
	@echo "$(COLOR_YELLOW)Copying userspace binaries (dev)...$(COLOR_RESET)"
	@if [ -f "$(USERSPACE_DIR)/init/target/x86_64-unknown-none/debug/init" ]; then cp $(USERSPACE_DIR)/init/target/x86_64-unknown-none/debug/init $(ISO_ROOT)/bin/init; fi
	@if [ -f "$(USERSPACE_DIR)/mello-term/target/x86_64-unknown-none/debug/mello-term" ]; then cp $(USERSPACE_DIR)/mello-term/target/x86_64-unknown-none/debug/mello-term $(ISO_ROOT)/bin/mello-term; fi
	@if [ -f "$(USERSPACE_DIR)/mello-sh/target/x86_64-unknown-none/debug/mello-sh" ]; then cp $(USERSPACE_DIR)/mello-sh/target/x86_64-unknown-none/debug/mello-sh $(ISO_ROOT)/bin/mello-sh; fi
	@if [ -f "$(USERSPACE_DIR)/mellobox/target/x86_64-unknown-none/debug/mellobox" ]; then \
		cp $(USERSPACE_DIR)/mellobox/target/x86_64-unknown-none/debug/mellobox $(ISO_ROOT)/bin/mellobox; \
		for util in ls cp mv rm cat grep ps kill mkdir touch echo pwd true false; do \
			ln -sf mellobox $(ISO_ROOT)/bin/$util 2>/dev/null || true; \
		done; \
	fi
	
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
		$(ISO_ROOT) -o $(ISO_NAME) 2>&1 | grep -v "^xorriso" || true
	
	# Install Limine bootloader to ISO
	@echo "$(COLOR_YELLOW)Installing Limine bootloader...$(COLOR_RESET)"
	@$(LIMINE_DIR)/limine bios-install $(ISO_NAME) 2>/dev/null || true
	@echo "$(COLOR_GREEN)✓ ISO image created successfully (dev mode)!$(COLOR_RESET)"
	@echo "$(COLOR_YELLOW)ISO location: $(ISO_NAME)$(COLOR_RESET)"

# Run kernel in QEMU (release mode)
run: iso
	@echo "$(COLOR_BLUE)Starting QEMU...$(COLOR_RESET)"
	@./tools/qemu/qemu.sh

# Run kernel in QEMU (dev mode - fast build)
run-dev: iso-dev
	@echo "$(COLOR_BLUE)Starting QEMU (dev mode)...$(COLOR_RESET)"
	@./tools/qemu/qemu.sh

# Clean build artifacts
clean:
	@echo "$(COLOR_BLUE)Cleaning build artifacts...$(COLOR_RESET)"
	@cd $(KERNEL_DIR) && $(CARGO) clean
	@cd $(USERSPACE_DIR)/init && $(CARGO) clean
	@cd $(USERSPACE_DIR)/mello-term && $(CARGO) clean
	@cd $(USERSPACE_DIR)/mello-sh && $(CARGO) clean
	@cd $(USERSPACE_DIR)/mellobox && $(CARGO) clean
	@cd $(USERSPACE_DIR)/kbd_test && $(CARGO) clean
	@cd $(USERSPACE_DIR)/serial_test && $(CARGO) clean
	@cd $(USERSPACE_DIR)/disk_bench && $(CARGO) clean
	@cd $(USERSPACE_DIR)/dmesg && $(CARGO) clean
	@cd $(USERSPACE_DIR)/lsdev && $(CARGO) clean
	@cd $(USERSPACE_DIR)/diskinfo && $(CARGO) clean
	@cd $(USERSPACE_DIR)/irq_test && $(CARGO) clean
	@rm -rf $(ISO_ROOT)
	@rm -f $(ISO_NAME)
	@rm -rf $(LIMINE_DIR)
	@echo "$(COLOR_GREEN)✓ Clean complete!$(COLOR_RESET)"

# Help target
help:
	@echo "MelloOS Build System"
	@echo ""
	@echo "Available targets:"
	@echo "  $(COLOR_GREEN)Release Mode (optimized, slow build):$(COLOR_RESET)"
	@echo "    make build       - Build kernel and userspace (release, optimized)"
	@echo "    make iso         - Create bootable ISO (release)"
	@echo "    make run         - Build ISO and run in QEMU (release)"
	@echo ""
	@echo "  $(COLOR_YELLOW)Dev Mode (fast build, no optimization):$(COLOR_RESET)"
	@echo "    make build-dev   - Build kernel and userspace (dev, FAST!)"
	@echo "    make iso-dev     - Create bootable ISO (dev)"
	@echo "    make run-dev     - Build ISO and run in QEMU (dev, FAST!)"
	@echo ""
	@echo "  $(COLOR_BLUE)Other targets:$(COLOR_RESET)"
	@echo "    make userspace   - Build all userspace programs (release)"
	@echo "    make userspace-dev - Build all userspace programs (dev)"
	@echo "    make symlinks    - Create symlinks for mellobox utilities"
	@echo "    make limine      - Download Limine bootloader"
	@echo "    make clean       - Clean build artifacts and ISO files"
	@echo ""
	@echo "  $(COLOR_YELLOW)💡 Tips:$(COLOR_RESET)"
	@echo "    • Use 'make run-dev' for fastest development cycle!"
	@echo "    • Enable sccache for even faster builds: SCCACHE=1 make build-dev"
	@echo "    • Install sccache: brew install sccache (macOS)"
	@echo ""
	@echo "Configuration:"
	@echo "  KERNEL_DIR    = $(KERNEL_DIR)"
	@echo "  BUILD_MODE    = $(BUILD_MODE)"
	@echo "  ISO_NAME      = $(ISO_NAME)"
ifdef SCCACHE
	@echo "  SCCACHE       = enabled ✓"
else
	@echo "  SCCACHE       = disabled (set SCCACHE=1 to enable)"
endif
