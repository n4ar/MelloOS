# Design: exec() Implementation and Shell Launch

## Overview

This document describes the design for implementing the `exec()` system call and enabling the init process to launch an interactive shell, transforming MelloOS from a test-only system to a user-facing operating system with a working command-line interface.

**Architecture:** Kernel syscall → ELF loader → Process image replacement → Shell execution

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    User Space                               │
│                                                             │
│  ┌──────────┐                    ┌──────────┐             │
│  │   Init   │  exec("/bin/sh")   │ mello-sh │             │
│  │ (PID 1)  │ ─────────────────> │  Shell   │             │
│  └──────────┘                    └──────────┘             │
│       │                                 │                   │
│       │ sys_exec()                      │ read/write        │
│       ↓                                 ↓                   │
└───────────────────────────────────────────────────────────┘
        │                                 │
        ↓                                 ↓
┌─────────────────────────────────────────────────────────────┐
│                    Kernel Space                             │
│                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐│
│  │   Syscall    │───>│  ELF Loader  │───>│   Process    ││
│  │   Handler    │    │              │    │   Manager    ││
│  └──────────────┘    └──────────────┘    └──────────────┘│
│         │                   │                    │         │
│         ↓                   ↓                    ↓         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐│
│  │     VFS      │    │    Memory    │    │     Task     ││
│  │  (File I/O)  │    │   Manager    │    │   Scheduler  ││
│  └──────────────┘    └──────────────┘    └──────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## Component Design

### 1. exec() System Call Interface

**File:** `kernel/src/sys/syscall.rs`

```rust
/// Execute a new program
///
/// # Arguments
/// * `path` - Path to executable file
/// * `argv` - Array of argument strings (NULL-terminated)
/// * `envp` - Array of environment strings (NULL-terminated)
///
/// # Returns
/// * Does not return on success
/// * -errno on failure
pub fn sys_exec(
    path: *const u8,
    argv: *const *const u8,
    envp: *const *const u8,
) -> isize {
    // 1. Validate user pointers
    // 2. Copy path, argv, envp from user space
    // 3. Call kernel exec implementation
    // 4. On success: never returns
    // 5. On failure: return error code
}
```

**Design Decisions:**
- Use POSIX-style interface (path, argv, envp)
- Validate all pointers before dereferencing
- Copy strings to kernel space to prevent TOCTOU attacks
- Return -errno on failure (POSIX convention)

---

### 2. ELF Loader Integration

**File:** `kernel/src/user/exec.rs` (new file)

```rust
pub struct ExecContext {
    /// Path to executable
    path: String,
    /// Command-line arguments
    argv: Vec<String>,
    /// Environment variables
    envp: Vec<String>,
    /// Current task
    task: Arc<Task>,
}

impl ExecContext {
    /// Execute a new program in the current process
    pub fn exec(self) -> Result<!, ExecError> {
        // 1. Open and validate ELF file
        let elf_data = self.load_elf_from_fs()?;
        
        // 2. Parse ELF headers
        let elf_info = self.parse_elf(&elf_data)?;
        
        // 3. Save old memory mappings (for rollback on error)
        let old_mappings = self.task.save_memory_state();
        
        // 4. Clear old process image
        self.clear_old_image()?;
        
        // 5. Load new program segments
        self.load_segments(&elf_info)?;
        
        // 6. Setup new stack with argv/envp
        let stack_top = self.setup_stack(&elf_info)?;
        
        // 7. Close O_CLOEXEC file descriptors
        self.close_cloexec_fds()?;
        
        // 8. Update task state
        self.task.set_entry_point(elf_info.entry);
        self.task.set_stack_pointer(stack_top);
        
        // 9. Jump to new program (never returns)
        self.jump_to_userspace(elf_info.entry, stack_top)
    }
}
```

**Key Operations:**

1. **File Loading:**
   ```rust
   fn load_elf_from_fs(&self) -> Result<Vec<u8>, ExecError> {
       // Use VFS to open file
       let inode = vfs::path::resolve_path(&self.path, None)?;
       
       // Read entire file into memory
       let mut buffer = Vec::new();
       inode.read_at(0, &mut buffer)?;
       
       Ok(buffer)
   }
   ```

2. **ELF Parsing:**
   ```rust
   fn parse_elf(&self, data: &[u8]) -> Result<ElfInfo, ExecError> {
       // Validate ELF magic
       if &data[0..4] != b"\x7FELF" {
           return Err(ExecError::InvalidFormat);
       }
       
       // Parse headers
       let header = ElfHeader::from_bytes(data)?;
       let program_headers = self.parse_program_headers(data, &header)?;
       
       Ok(ElfInfo {
           entry: header.entry,
           segments: program_headers,
       })
   }
   ```

3. **Memory Clearing:**
   ```rust
   fn clear_old_image(&self) -> Result<(), ExecError> {
       let mut task = self.task.lock();
       
       // Unmap all user memory regions
       for region in task.memory_regions() {
           if region.is_user_space() {
               task.unmap_region(region)?;
           }
       }
       
       // Reset heap
       task.reset_heap();
       
       Ok(())
   }
   ```

4. **Segment Loading:**
   ```rust
   fn load_segments(&self, elf_info: &ElfInfo) -> Result<(), ExecError> {
       for segment in &elf_info.segments {
           if segment.type != PT_LOAD {
               continue;
           }
           
           // Allocate memory for segment
           let vaddr = segment.vaddr;
           let size = segment.memsz;
           let flags = self.segment_flags_to_prot(segment.flags);
           
           self.task.map_region(vaddr, size, flags)?;
           
           // Copy segment data
           if segment.filesz > 0 {
               self.task.write_memory(vaddr, &segment.data)?;
           }
           
           // Zero BSS section
           if segment.memsz > segment.filesz {
               let bss_start = vaddr + segment.filesz;
               let bss_size = segment.memsz - segment.filesz;
               self.task.zero_memory(bss_start, bss_size)?;
           }
       }
       
       Ok(())
   }
   ```

5. **Stack Setup:**
   ```rust
   fn setup_stack(&self, elf_info: &ElfInfo) -> Result<u64, ExecError> {
       const STACK_SIZE: usize = 8 * 1024 * 1024; // 8MB
       const STACK_TOP: u64 = 0x0000_7FFF_FFFF_0000;
       
       // Allocate stack
       let stack_bottom = STACK_TOP - STACK_SIZE as u64;
       self.task.map_region(
           stack_bottom,
           STACK_SIZE,
           PROT_READ | PROT_WRITE
       )?;
       
       // Build stack layout:
       // [envp strings] [argv strings] [padding] [envp array] [argv array] [argc]
       let mut sp = STACK_TOP;
       
       // 1. Copy environment strings
       let envp_ptrs = self.copy_strings_to_stack(&mut sp, &self.envp)?;
       
       // 2. Copy argument strings
       let argv_ptrs = self.copy_strings_to_stack(&mut sp, &self.argv)?;
       
       // 3. Align stack to 16 bytes
       sp = sp & !0xF;
       
       // 4. Push envp array (NULL-terminated)
       sp = self.push_ptr_array(&mut sp, &envp_ptrs)?;
       
       // 5. Push argv array (NULL-terminated)
       sp = self.push_ptr_array(&mut sp, &argv_ptrs)?;
       
       // 6. Push argc
       sp -= 8;
       self.task.write_u64(sp, self.argv.len() as u64)?;
       
       Ok(sp)
   }
   ```

---

### 3. Process Image Replacement

**Memory Layout Transition:**

```
Before exec():                    After exec():
┌─────────────────┐              ┌─────────────────┐
│  Kernel Space   │              │  Kernel Space   │
│  (unchanged)    │              │  (unchanged)    │
├─────────────────┤              ├─────────────────┤
│   Old Stack     │              │   New Stack     │
│                 │              │  [argc/argv]    │
├─────────────────┤              ├─────────────────┤
│   Old Heap      │   ──────>    │   New Heap      │
│                 │              │  (empty)        │
├─────────────────┤              ├─────────────────┤
│   Old Data      │              │   New Data      │
│                 │              │  (.data/.bss)   │
├─────────────────┤              ├─────────────────┤
│   Old Code      │              │   New Code      │
│  (.text)        │              │  (.text)        │
└─────────────────┘              └─────────────────┘
```

**Replacement Steps:**
1. Save current state (for rollback)
2. Unmap all user memory regions
3. Map new program segments
4. Setup new stack
5. Update task registers (RIP, RSP)
6. Jump to new entry point

---

### 4. Init Process Modification

**File:** `kernel/userspace/init/src/main.rs`

**Current Behavior:**
```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Run tests
    test_privilege_level();
    test_syscalls();
    test_fork_chain();
    
    // Exit
    sys_exit(0);
}
```

**New Behavior:**
```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    sys_write("MelloOS Init Process Starting...\n");
    
    // Try to exec shell
    let shell_path = "/bin/sh\0";
    let argv = [
        shell_path.as_ptr(),
        core::ptr::null(),
    ];
    let envp = [
        b"PATH=/bin\0".as_ptr(),
        b"HOME=/root\0".as_ptr(),
        core::ptr::null(),
    ];
    
    let result = sys_exec(
        shell_path.as_ptr(),
        argv.as_ptr(),
        envp.as_ptr(),
    );
    
    // If exec fails, print error and loop
    sys_write("ERROR: Failed to exec shell\n");
    sys_write("Falling back to test mode...\n");
    
    // Run tests as fallback
    test_privilege_level();
    test_syscalls();
    
    // Hang
    loop {
        sys_yield();
    }
}
```

---

### 5. Shell Binary Embedding

**Approach:** Embed mello-sh binary in kernel, extract to filesystem on boot

**File:** `kernel/build.rs`

```rust
// Already embeds binaries:
// - init_binary.bin
// - mello_term_binary.bin
// - mello_sh_binary.bin  ← We need this!
// - mellobox_binary.bin

// Add to kernel initialization:
fn extract_binaries_to_fs() {
    // Create /bin directory
    vfs::mkdir("/bin")?;
    
    // Extract mello-sh
    let sh_data = include_bytes!(env!("MELLO_SH_BINARY"));
    vfs::write_file("/bin/sh", sh_data)?;
    
    // Make executable
    vfs::chmod("/bin/sh", 0o755)?;
    
    // Extract other utilities
    let mellobox_data = include_bytes!(env!("MELLOBOX_BINARY"));
    vfs::write_file("/bin/mellobox", mellobox_data)?;
    vfs::chmod("/bin/mellobox", 0o755)?;
    
    // Create symlinks for mellobox commands
    vfs::symlink("/bin/mellobox", "/bin/ls")?;
    vfs::symlink("/bin/mellobox", "/bin/cat")?;
    vfs::symlink("/bin/mellobox", "/bin/echo")?;
    // ... etc
}
```

---

### 6. File Descriptor Handling

**O_CLOEXEC Implementation:**

```rust
fn close_cloexec_fds(&self) -> Result<(), ExecError> {
    let task = self.task.lock();
    let fd_table = task.fd_table();
    
    for fd in 0..MAX_FDS {
        if let Some(file) = fd_table.get(fd) {
            if file.flags & O_CLOEXEC != 0 {
                fd_table.close(fd);
            }
        }
    }
    
    Ok(())
}
```

**Standard FD Preservation:**
- FD 0 (stdin): Preserved
- FD 1 (stdout): Preserved
- FD 2 (stderr): Preserved
- Other FDs: Preserved unless O_CLOEXEC

---

### 7. Error Handling Strategy

**Error Codes:**
```rust
pub enum ExecError {
    FileNotFound,      // ENOENT (-2)
    PermissionDenied,  // EACCES (-13)
    InvalidFormat,     // ENOEXEC (-8)
    OutOfMemory,       // ENOMEM (-12)
    InvalidArgument,   // EINVAL (-22)
    IoError,           // EIO (-5)
}

impl ExecError {
    fn to_errno(&self) -> isize {
        match self {
            ExecError::FileNotFound => -2,
            ExecError::PermissionDenied => -13,
            ExecError::InvalidFormat => -8,
            ExecError::OutOfMemory => -12,
            ExecError::InvalidArgument => -22,
            ExecError::IoError => -5,
        }
    }
}
```

**Rollback on Failure:**
```rust
fn exec_with_rollback(ctx: ExecContext) -> Result<!, ExecError> {
    // Save state
    let saved_state = ctx.task.save_state();
    
    // Try to exec
    match ctx.exec() {
        Ok(never) => never, // Never returns
        Err(e) => {
            // Restore old state
            ctx.task.restore_state(saved_state);
            Err(e)
        }
    }
}
```

---

### 8. Security Considerations

**Pointer Validation:**
```rust
fn validate_user_pointer(ptr: *const u8, len: usize) -> Result<(), ExecError> {
    let addr = ptr as usize;
    
    // Check NULL
    if addr == 0 {
        return Err(ExecError::InvalidArgument);
    }
    
    // Check user space range
    if addr >= KERNEL_BASE || addr + len >= KERNEL_BASE {
        return Err(ExecError::InvalidArgument);
    }
    
    // Check mapped
    if !current_task().is_mapped(addr, len) {
        return Err(ExecError::InvalidArgument);
    }
    
    Ok(())
}
```

**W^X Enforcement:**
```rust
fn segment_flags_to_prot(flags: u32) -> u32 {
    let mut prot = 0;
    
    if flags & PF_R != 0 {
        prot |= PROT_READ;
    }
    if flags & PF_W != 0 {
        prot |= PROT_WRITE;
    }
    if flags & PF_X != 0 {
        prot |= PROT_EXEC;
    }
    
    // Enforce W^X: cannot be both writable and executable
    if (prot & PROT_WRITE) != 0 && (prot & PROT_EXEC) != 0 {
        prot &= !PROT_EXEC; // Remove execute permission
    }
    
    prot
}
```

---

## Data Structures

### ExecContext
```rust
pub struct ExecContext {
    path: String,
    argv: Vec<String>,
    envp: Vec<String>,
    task: Arc<Task>,
}
```

### ElfInfo
```rust
struct ElfInfo {
    entry: u64,
    segments: Vec<ProgramSegment>,
}

struct ProgramSegment {
    type: u32,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
    flags: u32,
    data: Vec<u8>,
}
```

---

## Testing Strategy

### Unit Tests
1. ELF parsing with valid/invalid binaries
2. Pointer validation
3. Stack setup with various argv/envp
4. Error handling and rollback

### Integration Tests
1. exec() simple program (hello world)
2. exec() with arguments
3. exec() with environment
4. exec() failure cases
5. File descriptor inheritance
6. O_CLOEXEC handling

### System Tests
1. Boot → Shell appears
2. Shell accepts commands
3. Shell can exec other programs
4. Shell respawns on exit

---

## Performance Considerations

- **exec() latency:** Target < 10ms for typical binaries
- **Memory efficiency:** Reuse process structures, don't allocate new
- **File I/O:** Read entire ELF in one operation
- **Stack setup:** Minimize memory copies

---

## Implementation Phases

### Phase 1: Core exec() (2-3 hours)
- sys_exec() syscall handler
- ELF loading from filesystem
- Process image replacement
- Basic error handling

### Phase 2: Stack Setup (1-2 hours)
- argv/envp copying
- Stack layout
- Argument passing

### Phase 3: Init Integration (30 min)
- Modify init to exec shell
- Binary extraction to /bin
- Error fallback

### Phase 4: Testing & Polish (1-2 hours)
- Integration tests
- Error handling
- Security validation
- Performance tuning

**Total Estimated Time:** 5-8 hours

---

## Success Metrics

1. ✅ exec() syscall implemented and working
2. ✅ Shell launches on boot
3. ✅ User sees "mello$ " prompt
4. ✅ Commands execute correctly
5. ✅ No memory leaks
6. ✅ Error handling works
7. ✅ Security validation passes

---

## References

- Linux exec() implementation (fs/exec.c)
- POSIX exec family specification
- ELF format specification (elf.h)
- MelloOS ELF loader (kernel/src/user/elf.rs)

---

**Document Version:** 1.0  
**Status:** Ready for Implementation  
**Estimated Effort:** 5-8 hours
