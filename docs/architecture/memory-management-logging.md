# Memory Management Logging Documentation

## Overview

The memory management logging utilities provide formatted logging for all memory management operations with the `[MM]` prefix. All addresses are displayed in hexadecimal format (0x...) and memory sizes are displayed in appropriate units (bytes, KB, MB).

## Location

The logging utilities are implemented in `kernel/src/mm/log.rs`.

## Available Macros

### `mm_log!`

Basic logging macro with `[MM]` prefix.

**Usage:**
```rust
mm_log!("Initializing memory management...");
mm_log!("Total memory: {} MB", total_mb);
mm_log!("Allocated frame at 0x{:x}", frame_addr);
```

**Output:**
```
[MM] Initializing memory management...
[MM] Total memory: 8192 MB
[MM] Allocated frame at 0x100000
```

### `mm_info!`

Alias for `mm_log!` - used for informational messages.

**Usage:**
```rust
mm_info!("Physical memory manager initialized");
mm_info!("Free memory: {} MB", free_mb);
```

**Output:**
```
[MM] Physical memory manager initialized
[MM] Free memory: 7680 MB
```

### `mm_error!`

Error logging macro with `[MM] ERROR:` prefix.

**Usage:**
```rust
mm_error!("Out of physical memory");
mm_error!("Failed to map page at 0x{:x}", virt_addr);
```

**Output:**
```
[MM] ERROR: Out of physical memory
[MM] ERROR: Failed to map page at 0xffff800000000000
```

### `mm_debug!`

Debug logging macro (only active in debug builds).

**Usage:**
```rust
mm_debug!("Scanning bitmap from frame {}", start_frame);
```

**Output (debug builds only):**
```
[MM] DEBUG: Scanning bitmap from frame 256
```

### `mm_test_ok!`

Test success logging macro with `[MM] ✓` prefix.

**Usage:**
```rust
mm_test_ok!("PMM allocation test passed");
mm_test_ok!("Allocated frame at 0x{:x}", frame_addr);
```

**Output:**
```
[MM] ✓ PMM allocation test passed
[MM] ✓ Allocated frame at 0x100000
```

### `mm_test_fail!`

Test failure logging macro with `[MM] ✗` prefix.

**Usage:**
```rust
mm_test_fail!("PMM allocation test failed");
mm_test_fail!("Expected 0x{:x}, got 0x{:x}", expected, actual);
```

**Output:**
```
[MM] ✗ PMM allocation test failed
[MM] ✗ Expected 0x100000, got 0x200000
```

## Utility Functions

### `format_size(bytes: usize) -> (usize, &'static str)`

Formats a size in bytes to appropriate units (bytes, KB, MB).

**Usage:**
```rust
use crate::mm::log::format_size;

let (value, unit) = format_size(16 * 1024 * 1024);
mm_log!("Heap size: {} {}", value, unit);
```

**Output:**
```
[MM] Heap size: 16 MB
```

**Examples:**
- `format_size(512)` → `(512, "bytes")`
- `format_size(1024)` → `(1, "KB")`
- `format_size(2048)` → `(2, "KB")`
- `format_size(1048576)` → `(1, "MB")`
- `format_size(16777216)` → `(16, "MB")`

### `format_addr(addr: usize) -> usize`

Helper function for formatting addresses (returns the address unchanged for use in format strings).

**Usage:**
```rust
let addr = 0x1000;
mm_log!("Address: 0x{:x}", addr);
```

## Address Formatting

All addresses should be displayed in hexadecimal format with the `0x` prefix using the `{:x}` format specifier.

**Examples:**
```rust
// Physical addresses
let phys_addr = 0x100000;
mm_log!("Physical address: 0x{:x}", phys_addr);

// Virtual addresses
let virt_addr = 0xFFFF_8000_0000_0000;
mm_log!("Virtual address: 0x{:x}", virt_addr);

// Frame addresses
let frame = 0x1000;
mm_log!("Allocated frame at 0x{:x}", frame);
```

## Size Formatting

Memory sizes should be displayed in appropriate units using the `format_size()` function.

**Examples:**
```rust
use crate::mm::log::format_size;

// Heap size
let heap_size = 16 * 1024 * 1024;
let (value, unit) = format_size(heap_size);
mm_log!("Heap: {} {}", value, unit);  // Output: [MM] Heap: 16 MB

// Total memory
let total_bytes = 8192 * 1024 * 1024;
let (total_mb, _) = format_size(total_bytes);
mm_log!("Total memory: {} MB", total_mb);  // Output: [MM] Total memory: 8192 MB

// Small allocations
let alloc_size = 64;
let (size, unit) = format_size(alloc_size);
mm_log!("Allocated {} {}", size, unit);  // Output: [MM] Allocated 64 bytes
```

## Complete Usage Example

```rust
use crate::{mm_log, mm_info, mm_error, mm_test_ok};
use crate::mm::log::format_size;

pub fn init_memory() {
    mm_log!("==========================================");
    mm_log!("Initializing Memory Management");
    mm_log!("==========================================");
    
    // Log memory information
    let total_mb = 8192;
    let free_mb = 7680;
    mm_info!("Total memory: {} MB", total_mb);
    mm_info!("Free memory: {} MB", free_mb);
    
    // Log frame allocation
    let frame_addr = 0x100000;
    mm_log!("Allocated frame at 0x{:x}", frame_addr);
    
    // Log heap setup
    let heap_start = 0xFFFF_A000_0000_0000;
    let heap_size = 16 * 1024 * 1024;
    let (size_val, size_unit) = format_size(heap_size);
    mm_log!("Kernel heap: 0x{:x} - 0x{:x} ({} {})", 
            heap_start, 
            heap_start + heap_size, 
            size_val, 
            size_unit);
    
    // Log errors if needed
    if frame_addr == 0 {
        mm_error!("Out of physical memory");
    }
    
    // Log test results
    mm_test_ok!("PMM tests passed");
    mm_test_ok!("Paging tests passed");
    mm_test_ok!("Allocator tests passed");
    
    mm_log!("==========================================");
    mm_log!("Memory management initialized successfully");
    mm_log!("==========================================");
}
```

**Expected Output:**
```
[MM] ==========================================
[MM] Initializing Memory Management
[MM] ==========================================
[MM] Total memory: 8192 MB
[MM] Free memory: 7680 MB
[MM] Allocated frame at 0x100000
[MM] Kernel heap: 0xffffa00000000000 - 0xffffa00001000000 (16 MB)
[MM] ✓ PMM tests passed
[MM] ✓ Paging tests passed
[MM] ✓ Allocator tests passed
[MM] ==========================================
[MM] Memory management initialized successfully
[MM] ==========================================
```

## Requirements Satisfied

This logging implementation satisfies the following requirements:

- **Requirement 5.4**: All memory management operations are logged with the `[MM]` prefix
- **Requirement 5.5**: All addresses are displayed in hexadecimal format (0x...) and memory sizes are displayed in appropriate units (bytes, KB, MB)

## Implementation Notes

The current implementation uses a placeholder `SerialWriter` that can be replaced with actual serial port output or framebuffer output as needed. The macros are designed to be zero-cost when logging is disabled in release builds (for `mm_debug!`).

## Future Enhancements

- Implement actual serial port output in `SerialWriter`
- Add support for different log levels (INFO, WARN, ERROR, DEBUG)
- Add timestamp support for log messages
- Add support for logging to framebuffer in addition to serial
- Add compile-time log level filtering
