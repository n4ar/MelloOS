# User-Mode Support Design Document

## Overview

This document describes the design for implementing user-mode support in MelloOS, enabling the transition from kernel-mode (ring 0) to user-mode (ring 3) execution. The design builds upon the existing kernel infrastructure including the scheduler, memory management, and syscall framework to provide a complete user-kernel separation with process management capabilities.

The implementation will extend MelloOS from a kernel-only system to a full operating system capable of running user programs safely in a restricted environment while maintaining kernel control over system resources.

## Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Space (Ring 3)                     │
├─────────────────────────────────────────────────────────────────┤
│  Init Process (PID 1)  │  User Programs  │  Test Programs      │
│  - Hello from userland │  - fork/exec    │  - Stress tests     │
│  - Basic syscalls      │  - Process mgmt │  - Multi-process    │
└─────────────────────────────────────────────────────────────────┘
                                   │
                            Syscall Interface
                         (int 0x80 / syscall/sysret)
                                   │
┌─────────────────────────────────────────────────────────────────┐
│                       Kernel Space (Ring 0)                    │
├─────────────────────────────────────────────────────────────────┤
│  Syscall Dispatcher  │  Process Manager  │  ELF Loader         │
│  - sys_write         │  - fork/exec      │  - Parse ELF64      │
│  - sys_exit          │  - exit/wait      │  - Map segments     │
│  - sys_fork          │  - PID mgmt       │  - Setup memory     │
│  - sys_exec          │  - Process table  │  - Entry point     │
│  - sys_wait          │                   │                     │
│  - sys_yield         │                   │                     │
└─────────────────────────────────────────────────────────────────┘
                                   │
┌─────────────────────────────────────────────────────────────────┐
│                    Existing Kernel Infrastructure               │
├─────────────────────────────────────────────────────────────────┤
│  Scheduler (SMP)     │  Memory Mgmt      │  Sync Primitives    │
│  - Task switching    │  - PMM/Paging     │  - Spinlocks        │
│  - Per-CPU queues    │  - Virtual memory │  - Lock ordering    │
│  - Priority sched    │  - Page tables    │  - SMP safety       │
└─────────────────────────────────────────────────────────────────┘
```

### Memory Layout

```
Virtual Address Space Layout:

Kernel Space (Ring 0):
0xFFFF_8000_0000_0000 - 0xFFFF_FFFF_FFFF_FFFF  │ Kernel Space
├── 0xFFFF_A000_0000_0000 - 0xFFFF_A000_0100_0000  │ Kernel Heap (16MB)
├── 0xFFFF_8000_0000_0000 - 0xFFFF_9FFF_FFFF_FFFF  │ Direct Map (HHDM)
└── 0xFFFF_C000_0000_0000 - 0xFFFF_FFFF_FFFF_FFFF  │ Kernel Code/Data

User Space (Ring 3):
0x0000_0000_0000_0000 - 0x0000_7FFF_FFFF_FFFF  │ User Space
├── 0x0000_0000_0040_0000 - 0x0000_0000_0080_0000  │ User Code (.text)
├── 0x0000_0000_0080_0000 - 0x0000_0000_00C0_0000  │ User Data (.data/.bss)
├── 0x0000_7FFF_F000_0000 - 0x0000_7FFF_FFFF_0000  │ User Stack (8KB)
└── 0x0000_0000_1000_0000 - 0x0000_0000_8000_0000  │ User Heap (future)

Constants:
- USER_LIMIT = 0x0000_8000_0000_0000 (512GB user space limit)
- USER_STACK_TOP = 0x0000_7FFF_FFFF_0000
- USER_STACK_SIZE = 8192 (8KB)
```

## Components and Interfaces

### 1. Ring Transition Infrastructure

#### GDT Extensions
The existing Limine GDT will be extended with user-mode segments:

```rust
// Current Limine GDT layout (read-only, we cannot modify)
// 0x00: Null descriptor
// 0x08: Kernel code (16-bit, unused)
// 0x10: Kernel data (16-bit, unused)  
// 0x18: Kernel code (32-bit, unused)
// 0x20: Kernel data (32-bit, unused)
// 0x28: Kernel code (64-bit) - Ring 0
// 0x30: Kernel data (64-bit) - Ring 0

// We need to create a new GDT with user segments:
pub const KERNEL_CODE_SEG: u16 = 0x28;  // Ring 0 code
pub const KERNEL_DATA_SEG: u16 = 0x30;  // Ring 0 data
pub const USER_CODE_SEG: u16 = 0x3B;    // Ring 3 code (0x38 | 3)
pub const USER_DATA_SEG: u16 = 0x43;    // Ring 3 data (0x40 | 3)
pub const TSS_SEG: u16 = 0x48;          // TSS segment

struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

struct TssEntry {
    limit_low: u16,
    base_low: u16,
    base_mid: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
    base_upper: u32,
    reserved: u32,
}
```

#### Task State Segment (TSS)
```rust
#[repr(C, packed)]
struct TaskStateSegment {
    reserved1: u32,
    rsp0: u64,      // Ring 0 stack pointer (kernel stack) - CRITICAL for syscalls
    rsp1: u64,      // Ring 1 stack pointer (unused)
    rsp2: u64,      // Ring 2 stack pointer (unused)
    reserved2: u64,
    ist1: u64,      // IST 1: NMI handler stack
    ist2: u64,      // IST 2: Double fault handler stack  
    ist3: u64,      // IST 3: Page fault handler stack (optional)
    ist4: u64,      // IST 4: Reserved
    ist5: u64,      // IST 5: Reserved
    ist6: u64,      // IST 6: Reserved
    ist7: u64,      // IST 7: Reserved
    reserved3: u64,
    reserved4: u16,
    iomap_base: u16,
}

impl TaskStateSegment {
    const fn new() -> Self {
        Self {
            reserved1: 0,
            rsp0: 0,    // Will be set per-CPU
            rsp1: 0,
            rsp2: 0,
            reserved2: 0,
            ist1: 0,    // Will be set for NMI
            ist2: 0,    // Will be set for double fault
            ist3: 0,    // Optional for page fault
            ist4: 0,
            ist5: 0,
            ist6: 0,
            ist7: 0,
            reserved3: 0,
            reserved4: 0,
            iomap_base: core::mem::size_of::<TaskStateSegment>() as u16,
        }
    }
    
    // Set kernel stack for this CPU (called during context switch)
    pub fn set_kernel_stack(&mut self, stack_top: u64) {
        self.rsp0 = stack_top;
    }
    
    // Set up IST stacks for critical handlers
    pub fn setup_ist_stacks(&mut self, cpu_id: usize) {
        // Allocate separate stacks for critical interrupt handlers
        let nmi_stack = alloc_kernel_stack(4096).expect("Failed to alloc NMI stack");
        let df_stack = alloc_kernel_stack(4096).expect("Failed to alloc DF stack");
        
        self.ist1 = nmi_stack + 4096;      // NMI stack (top)
        self.ist2 = df_stack + 4096;       // Double fault stack (top)
        
        serial_println!("[TSS] CPU {} IST stacks: NMI=0x{:x}, DF=0x{:x}", 
                       cpu_id, self.ist1, self.ist2);
    }
}

// Per-CPU TSS instances for SMP safety
static mut TSS_TABLE: [TaskStateSegment; MAX_CPUS] = [TaskStateSegment::new(); MAX_CPUS];

// Initialize TSS for a specific CPU
pub fn init_tss_for_cpu(cpu_id: usize) {
    unsafe {
        let tss = &mut TSS_TABLE[cpu_id];
        
        // Set up IST stacks for critical handlers
        tss.setup_ist_stacks(cpu_id);
        
        // Get per-CPU kernel stack from PerCpu structure
        let percpu = crate::arch::x86_64::smp::percpu::percpu_for(cpu_id);
        tss.set_kernel_stack(percpu.kernel_stack_top);
        
        // Install TSS in GDT and load it
        install_tss_in_gdt(cpu_id, tss as *const _ as u64);
        load_tss(TSS_SEG);
    }
}

// Update TSS.rsp0 when switching processes (if needed)
pub fn update_kernel_stack_for_process(cpu_id: usize, kernel_stack_top: u64) {
    unsafe {
        TSS_TABLE[cpu_id].set_kernel_stack(kernel_stack_top);
    }
}
```

#### User Entry Trampoline
```rust
// Assembly trampoline to transition to user mode
// File: kernel/src/arch/x86_64/user_entry.S

// Arguments: RDI = entry_point, RSI = user_stack_top
#[naked]
pub unsafe extern "C" fn user_entry_trampoline(entry_point: u64, user_stack: u64) -> ! {
    core::arch::naked_asm!(
        // Validate arguments are canonical user addresses
        "test rdi, 0xFFFF800000000000", // Check entry_point is canonical user addr
        "jnz .invalid_entry",
        "test rsi, 0xFFFF800000000000", // Check user_stack is canonical user addr  
        "jnz .invalid_stack",
        
        // In long mode, segment registers DS/ES are largely ignored
        // FS/GS use base MSRs rather than selectors
        // Only SS in IRET frame matters for privilege transition
        
        // Prepare IRET stack frame for transition to ring 3
        // Stack layout (pushed in reverse order):
        "push {user_ss}",       // SS (user data segment with RPL=3)
        "push rsi",             // RSP (user stack pointer - validated above)
        "pushfq",               // RFLAGS (current flags)
        "or qword [rsp], 0x200", // Set IF (interrupt enable) in saved RFLAGS
        "push {user_cs}",       // CS (user code segment with RPL=3)  
        "push rdi",             // RIP (entry point - validated above)
        
        // Transition to user mode (ring 3)
        // IRET will:
        // - Pop RIP, CS, RFLAGS, RSP, SS from stack
        // - Set CPL=3 (from CS.RPL)
        // - Enable interrupts (from RFLAGS.IF)
        "iretq",
        
        // Error handlers
        ".invalid_entry:",
        "mov rdi, 0xDEAD0001",  // Error code for invalid entry point
        "call {panic_handler}",
        
        ".invalid_stack:",
        "mov rdi, 0xDEAD0002",  // Error code for invalid stack
        "call {panic_handler}",
        
        user_cs = const USER_CODE_SEG,
        user_ss = const USER_DATA_SEG,
        panic_handler = sym kernel_panic_invalid_user_transition,
    )
}

// Panic handler for invalid user transitions
extern "C" fn kernel_panic_invalid_user_transition(error_code: u64) -> ! {
    panic!("[USER] Invalid user mode transition: error 0x{:x}", error_code);
}

// Helper function to set up user stack with guard page
pub fn setup_user_stack(process: &mut Process) -> Result<u64, ProcessError> {
    let stack_top = USER_STACK_TOP;
    let stack_size = USER_STACK_SIZE;
    let stack_bottom = stack_top - stack_size;
    let guard_page = stack_bottom - 4096;
    
    // Map stack pages (RW + NX)
    for addr in (stack_bottom..stack_top).step_by(4096) {
        let phys_frame = process.pmm.alloc_frame()
            .ok_or(ProcessError::OutOfMemory)?;
        
        process.mapper.map_page(
            addr,
            phys_frame,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | 
            PageTableFlags::USER | PageTableFlags::NO_EXECUTE,
            &mut process.pmm,
        )?;
    }
    
    // Leave guard page unmapped to catch stack overflow
    // (guard_page is intentionally not mapped)
    
    // Add memory region for tracking
    process.add_memory_region(MemoryRegion {
        start: stack_bottom,
        end: stack_top,
        flags: PageTableFlags::PRESENT | PageTableFlags::WRITABLE | 
               PageTableFlags::USER | PageTableFlags::NO_EXECUTE,
        region_type: MemoryRegionType::Stack,
    })?;
    
    Ok(stack_top)
}
```

### 2. Syscall Interface Enhancement

#### MSR Configuration for syscall/sysret
```rust
// Model Specific Registers for syscall/sysret
const EFER_MSR: u32 = 0xC0000080;   // Extended Feature Enable Register
const STAR_MSR: u32 = 0xC0000081;   // Syscall target address
const LSTAR_MSR: u32 = 0xC0000082;  // Long mode syscall target
const SFMASK_MSR: u32 = 0xC0000084; // Syscall flag mask
const KERNEL_GS_BASE_MSR: u32 = 0xC0000102; // Kernel GS base
const GS_BASE_MSR: u32 = 0xC0000101;        // User GS base

const SCE_BIT: u64 = 1 << 0;        // System Call Extensions enable

pub fn init_syscall_msrs() {
    unsafe {
        // 1. Enable SCE (System Call Extensions) in EFER
        let mut efer = rdmsr(EFER_MSR);
        efer |= SCE_BIT;
        wrmsr(EFER_MSR, efer);
        
        // 2. STAR: Set kernel and user segment selectors (base selectors)
        // Hardware will derive SS = CS + 8 automatically for both kernel and user
        // Bits 63:48 = User CS base (USER_CODE_SEG without RPL bits)
        // Bits 47:32 = Kernel CS base (KERNEL_CODE_SEG)
        let user_cs_base = (USER_CODE_SEG & !3) as u64;  // Remove RPL bits
        let kernel_cs_base = KERNEL_CODE_SEG as u64;
        let star_value = (user_cs_base << 48) | (kernel_cs_base << 32);
        wrmsr(STAR_MSR, star_value);
        
        // 3. LSTAR: Set syscall entry point
        let lstar_value = syscall_entry_fast as u64;
        wrmsr(LSTAR_MSR, lstar_value);
        
        // 4. SFMASK: Mask RFLAGS bits during syscall
        // Clear IF (interrupt flag) during syscall for atomic entry
        let sfmask_value = 0x200; // IF bit
        wrmsr(SFMASK_MSR, sfmask_value);
        
        // 5. Set up GS base for per-CPU data access (per-CPU initialization)
        // KERNEL_GS_BASE will be swapped with GS_BASE by SWAPGS
        let cpu_id = crate::arch::x86_64::smp::cpu_id();
        let percpu_base = crate::arch::x86_64::smp::percpu::get_percpu_base(cpu_id);
        wrmsr(KERNEL_GS_BASE_MSR, percpu_base as u64);
        wrmsr(GS_BASE_MSR, 0); // User GS base (initially 0)
    }
}
```

#### Enhanced Syscall Dispatcher
```rust
// Extended syscall table
pub const SYS_WRITE: usize = 0;
pub const SYS_EXIT: usize = 1;
pub const SYS_FORK: usize = 2;
pub const SYS_EXEC: usize = 3;
pub const SYS_WAIT: usize = 4;
pub const SYS_YIELD: usize = 5;
pub const SYS_GETPID: usize = 6;
pub const SYS_MMAP: usize = 7;    // Reserved for future heap management
pub const SYS_BRK: usize = 8;     // Reserved for future heap management

// Syscall calling convention (x86-64 compatible with Linux):
// RAX = syscall number
// RDI = arg1, RSI = arg2, RDX = arg3
// R10 = arg4, R8 = arg5, R9 = arg6  (NOTE: R10 not RCX for arg4!)
// Return value in RAX
//
// IMPORTANT: RCX and R11 are clobbered by SYSCALL instruction:
// - RCX = user RIP (return address)
// - R11 = user RFLAGS
// Therefore arg4 uses R10 instead of RCX to avoid conflicts

pub fn syscall_dispatcher(
    syscall_id: usize,
    arg1: usize, arg2: usize, arg3: usize,
    arg4: usize, arg5: usize, arg6: usize,
) -> isize {
    // Get current CPU and process for logging
    let cpu_id = unsafe { crate::arch::x86_64::smp::percpu::percpu_current().id };
    let pid = get_current_process_id().unwrap_or(0);
    
    // Log syscall with CPU and PID
    serial_println!("[SYSCALL][cpu{} pid={}] {} ({})", 
                   cpu_id, pid, syscall_name(syscall_id), syscall_id);
    
    // Validate user pointers before processing (only for pointer arguments)
    match syscall_id {
        SYS_WRITE | SYS_EXEC => {
            if !is_user_pointer_valid(arg2) {
                return -EFAULT;
            }
        }
        _ => {}
    }
    
    match syscall_id {
        SYS_WRITE => sys_write(arg1, arg2, arg3),
        SYS_EXIT => sys_exit(arg1),
        SYS_FORK => sys_fork(),
        SYS_EXEC => sys_exec(arg1, arg2),
        SYS_WAIT => sys_wait(arg1),
        SYS_YIELD => sys_yield(),
        SYS_GETPID => sys_getpid(),
        _ => -ENOSYS,
    }
}

// Fast syscall entry point (assembly)
// File: kernel/src/arch/x86_64/syscall/entry.S
#[naked]
unsafe extern "C" fn syscall_entry_fast() {
    core::arch::naked_asm!(
        // SYSCALL instruction has already:
        // - Saved user RIP to RCX
        // - Saved user RFLAGS to R11
        // - Loaded kernel CS from STAR[47:32]
        // - Loaded kernel RIP from LSTAR
        // - Masked RFLAGS with SFMASK
        // - Set CPL=0
        
        // 1. Switch to kernel GS base for per-CPU data access
        "swapgs",
        
        // 2. Switch to kernel stack safely
        // Get kernel stack top from per-CPU area through GS
        "mov rax, gs:[{percpu_kernel_stack_offset}]",
        "mov [rax - 8], rsp",       // Save user RSP on kernel stack
        "mov rsp, rax",             // Switch to kernel stack safely
        
        // 3. Save user context on kernel stack
        "push r11",        // User RFLAGS
        "push rcx",        // User RIP
        "push rdi",        // arg1
        "push rsi",        // arg2  
        "push rdx",        // arg3
        "push r10",        // arg4 (NOT RCX!)
        "push r8",         // arg5
        "push r9",         // arg6
        
        // 4. Prepare arguments for dispatcher
        // syscall_dispatcher(rax, rdi, rsi, rdx, r10, r8, r9)
        "mov rdi, rax",    // syscall_id
        "mov rsi, [rsp+40]", // arg1 (saved rdi)
        "mov rdx, [rsp+32]", // arg2 (saved rsi) 
        "mov rcx, [rsp+24]", // arg3 (saved rdx)
        "mov r8, [rsp+16]",  // arg4 (saved r10)
        "mov r9, [rsp+8]",   // arg5 (saved r8)
        // arg6 is already in correct position on stack
        
        // 5. Call dispatcher
        "call {dispatcher}",
        
        // 6. Restore user context
        "pop r9",          // arg6
        "pop r8",          // arg5
        "pop r10",         // arg4
        "pop rdx",         // arg3
        "pop rsi",         // arg2
        "pop rdi",         // arg1
        "pop rcx",         // User RIP
        "pop r11",         // User RFLAGS
        
        // 7. Validate canonical addresses before SYSRET
        // SYSRET will #GP if RIP or RSP are non-canonical
        "test rcx, 0xFFFF800000000000",  // Check RIP canonical
        "jnz .bad_return",
        "test rsp, 0xFFFF800000000000",  // Check RSP canonical  
        "jnz .bad_return",
        
        // 8. Switch back to user GS base
        "swapgs",
        
        // 9. Return to user mode
        // SYSRET will:
        // - Load user CS from STAR[63:48] + 16 (with RPL=3)
        // - Load user SS from STAR[63:48] + 8 (with RPL=3)  
        // - Load user RIP from RCX
        // - Load user RFLAGS from R11
        // - Set CPL=3
        "sysretq",
        
        // Error handler for non-canonical addresses
        ".bad_return:",
        "swapgs",          // Restore kernel GS
        "mov rdi, rcx",    // Pass bad RIP as argument
        "mov rsi, rsp",    // Pass bad RSP as argument
        "call {bad_return_handler}",
        "ud2",             // Should never return",
        
        percpu_kernel_stack_offset = const 0x10, // Offset to kernel_stack in PerCpu
        dispatcher = sym syscall_dispatcher,
        bad_return_handler = sym handle_bad_syscall_return,
    )
}
```

### 3. ELF Binary Loader

#### ELF64 Parser
```rust
#[repr(C)]
struct Elf64Header {
    e_ident: [u8; 16],      // ELF identification
    e_type: u16,            // Object file type (ET_EXEC = 2)
    e_machine: u16,         // Machine type (EM_X86_64 = 62)
    e_version: u32,         // Object file version
    e_entry: u64,           // Entry point address
    e_phoff: u64,           // Program header offset
    e_shoff: u64,           // Section header offset
    e_flags: u32,           // Processor-specific flags
    e_ehsize: u16,          // ELF header size
    e_phentsize: u16,       // Program header entry size
    e_phnum: u16,           // Number of program header entries
    e_shentsize: u16,       // Section header entry size
    e_shnum: u16,           // Number of section header entries
    e_shstrndx: u16,        // Section header string table index
}

#[repr(C)]
struct Elf64ProgramHeader {
    p_type: u32,            // Segment type (PT_LOAD = 1)
    p_flags: u32,           // Segment flags (PF_X=1, PF_W=2, PF_R=4)
    p_offset: u64,          // Segment file offset
    p_vaddr: u64,           // Segment virtual address
    p_paddr: u64,           // Segment physical address
    p_filesz: u64,          // Segment size in file
    p_memsz: u64,           // Segment size in memory
    p_align: u64,           // Segment alignment
}

pub struct ElfLoader {
    pmm: &'static mut PhysicalMemoryManager,
    mapper: &'static mut PageMapper,
}

impl ElfLoader {
    pub fn load_elf(&mut self, elf_data: &[u8]) -> Result<u64, ElfError> {
        // 1. Parse and validate ELF header
        let header = self.parse_elf_header(elf_data)?;
        
        // 2. Validate ELF format
        self.validate_elf(&header)?;
        
        // 3. Parse program headers
        let program_headers = self.parse_program_headers(elf_data, &header)?;
        
        // 4. Map PT_LOAD segments
        for phdr in program_headers {
            if phdr.p_type == PT_LOAD {
                self.map_segment(elf_data, &phdr)?;
            }
        }
        
        // 5. Set up user stack
        self.setup_user_stack()?;
        
        Ok(header.e_entry)
    }
    
    fn map_segment(&mut self, elf_data: &[u8], phdr: &Elf64ProgramHeader) -> Result<(), ElfError> {
        let vaddr = phdr.p_vaddr;
        let size = phdr.p_memsz;
        let file_size = phdr.p_filesz;
        
        // Validate virtual address is in user space
        if vaddr >= USER_LIMIT {
            return Err(ElfError::InvalidAddress);
        }
        
        // Validate entry point alignment (recommended)
        if vaddr % 4096 != 0 {
            serial_println!("[ELF] Warning: Segment at 0x{:x} not page-aligned", vaddr);
        }
        
        // Calculate page-aligned range
        let start_page = vaddr & !0xFFF;
        let end_page = (vaddr + size + 0xFFF) & !0xFFF;
        
        // Determine page flags from program header
        let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER;
        if phdr.p_flags & PF_W != 0 { 
            flags |= PageTableFlags::WRITABLE; 
        }
        if phdr.p_flags & PF_X == 0 { 
            flags |= PageTableFlags::NO_EXECUTE; 
        }
        
        // Map pages and copy data using kernel mapping
        for page_addr in (start_page..end_page).step_by(4096) {
            let phys_frame = self.pmm.alloc_frame()
                .ok_or(ElfError::OutOfMemory)?;
            
            // Map page in user space
            self.mapper.map_page(page_addr, phys_frame, flags, self.pmm)?;
            
            // Create temporary kernel mapping for safe data copying
            let kernel_vaddr = crate::mm::phys_to_virt(phys_frame);
            
            // Calculate what portion of this page needs data from ELF
            let page_offset = if page_addr >= vaddr { 0 } else { vaddr - page_addr };
            let page_file_start = if page_addr >= vaddr { 
                phdr.p_offset + (page_addr - vaddr) 
            } else { 
                phdr.p_offset 
            };
            let page_file_size = core::cmp::min(
                4096 - page_offset,
                if file_size > (page_addr - vaddr) { 
                    file_size - (page_addr - vaddr) 
                } else { 
                    0 
                }
            );
            
            // Zero the entire page first
            unsafe {
                let page_slice = core::slice::from_raw_parts_mut(
                    kernel_vaddr as *mut u8, 
                    4096
                );
                page_slice.fill(0);
            }
            
            // Copy ELF data to this page if any
            if page_file_size > 0 && page_file_start < elf_data.len() as u64 {
                let src_start = page_file_start as usize;
                let src_end = core::cmp::min(
                    src_start + page_file_size as usize,
                    elf_data.len()
                );
                let src = &elf_data[src_start..src_end];
                
                unsafe {
                    let dst = core::slice::from_raw_parts_mut(
                        (kernel_vaddr + page_offset) as *mut u8,
                        src.len()
                    );
                    dst.copy_from_slice(src);
                }
            }
            
            // Flush TLB for this page to ensure visibility
            unsafe {
                core::arch::asm!("invlpg [{}]", in(reg) page_addr);
                // TODO: IPI TLB shootdown for SMP when implementing full page table separation
            }
        }
        
        Ok(())
    }
}
```

### 4. Process Management

#### Process Control Block (PCB)
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Ready,      // Ready to run
    Running,    // Currently executing
    Sleeping,   // Sleeping for specified ticks
    Blocked,    // Blocked on I/O or IPC
    Zombie,     // Terminated, waiting for parent
    Terminated, // Fully cleaned up
}

pub struct Process {
    pub pid: ProcessId,
    pub parent_pid: Option<ProcessId>,
    pub state: ProcessState,
    pub exit_code: Option<i32>,
    
    // Memory management
    pub page_table: PageTable,
    pub memory_regions: Vec<MemoryRegion>,
    
    // CPU context (for context switching)
    pub context: CpuContext,
    
    // Scheduling
    pub priority: TaskPriority,
    pub wake_tick: Option<u64>,
    
    // File descriptors (future)
    pub fd_table: [Option<FileDescriptor>; MAX_FDS],
    
    // Process metadata
    pub name: [u8; 16],
    pub creation_time: u64,
    pub cpu_time: u64,
}

// Per-process fine-grained locking for SMP scalability
struct ProcessTableEntry {
    process: Option<Process>,
    lock: SpinLock<()>,  // Per-process lock
}

impl ProcessTableEntry {
    const fn new() -> Self {
        Self {
            process: None,
            lock: SpinLock::new(()),
        }
    }
}

// Global process table with per-process locks (SMP-friendly)
static PROCESS_TABLE: [ProcessTableEntry; MAX_PROCESSES] = [ProcessTableEntry::new(); MAX_PROCESSES];
static NEXT_PID: AtomicUsize = AtomicUsize::new(1);

pub fn alloc_pid() -> ProcessId {
    NEXT_PID.fetch_add(1, Ordering::Relaxed)
}
```

#### Process Management Operations
```rust
impl ProcessManager {
    pub fn fork(&mut self, parent_pid: ProcessId) -> Result<ProcessId, ProcessError> {
        let parent = self.get_process(parent_pid)?;
        let child_pid = alloc_pid();
        
        // Create child process
        let mut child = Process::new(child_pid, Some(parent_pid));
        
        // Copy parent's memory space (TODO: implement copy-on-write)
        child.page_table = parent.page_table.clone()?;
        child.memory_regions = parent.memory_regions.clone();
        
        // Copy CPU context (child returns 0, parent returns child PID)
        child.context = parent.context.clone();
        child.context.rax = 0; // Child return value
        
        // Add to process table
        self.add_process(child)?;
        
        // Enqueue child to scheduler
        crate::sched::enqueue_task(child_pid, None);
        
        Ok(child_pid)
    }
    
    pub fn exec(&mut self, pid: ProcessId, elf_path: &str) -> Result<(), ProcessError> {
        let process = self.get_process_mut(pid)?;
        
        // Clear current memory space
        self.clear_memory_space(process)?;
        
        // Load new ELF binary
        let entry_point = self.elf_loader.load_elf_from_path(elf_path)?;
        
        // Reset CPU context for new program
        process.context = CpuContext::new_user(entry_point, USER_STACK_TOP);
        
        // Reset process state
        process.state = ProcessState::Ready;
        
        Ok(())
    }
    
    pub fn exit(&mut self, pid: ProcessId, exit_code: i32) {
        if let Some(process) = self.get_process_mut(pid) {
            process.state = ProcessState::Zombie;
            process.exit_code = Some(exit_code);
            
            // Wake up parent if waiting
            if let Some(parent_pid) = process.parent_pid {
                self.wake_waiting_parent(parent_pid, pid);
            }
            
            // Clean up resources
            self.cleanup_process_resources(pid);
        }
    }
    
    pub fn wait(&mut self, parent_pid: ProcessId, child_pid: Option<ProcessId>) -> Result<(ProcessId, i32), ProcessError> {
        // Find zombie child
        let zombie_child = self.find_zombie_child(parent_pid, child_pid)?;
        
        if let Some((child_pid, exit_code)) = zombie_child {
            // Remove zombie child from process table
            self.remove_process(child_pid);
            Ok((child_pid, exit_code))
        } else {
            // No zombie children, block parent
            self.block_process(parent_pid, BlockReason::WaitingForChild);
            Err(ProcessError::WouldBlock)
        }
    }
}
```

### 5. Memory Protection and Safety

#### User Pointer Validation
```rust
pub const USER_LIMIT: usize = 0x0000_8000_0000_0000;

pub fn is_user_pointer_valid(ptr: usize) -> bool {
    ptr != 0 && ptr < USER_LIMIT
}

pub fn copy_from_user(dst: &mut [u8], src_ptr: usize, len: usize) -> Result<(), MemoryError> {
    // Validate source pointer is in user space
    if !is_user_pointer_valid(src_ptr) || !is_user_pointer_valid(src_ptr + len) {
        return Err(MemoryError::InvalidUserPointer);
    }
    
    // Check destination buffer size
    if len > dst.len() {
        return Err(MemoryError::BufferTooSmall);
    }
    
    // TODO: When implementing full page table separation, replace direct pointer
    // access with temporary kernel mapping (kmap_user_page()) for safety
    
    // Perform copy with page fault handling (current shared address space)
    unsafe {
        let src = core::slice::from_raw_parts(src_ptr as *const u8, len);
        dst[..len].copy_from_slice(src);
    }
    
    Ok(())
}

pub fn copy_to_user(dst_ptr: usize, src: &[u8]) -> Result<(), MemoryError> {
    // Validate destination pointer is in user space
    if !is_user_pointer_valid(dst_ptr) || !is_user_pointer_valid(dst_ptr + src.len()) {
        return Err(MemoryError::InvalidUserPointer);
    }
    
    // TODO: When implementing full page table separation, replace direct pointer
    // access with temporary kernel mapping (kmap_user_page()) for safety
    
    // Perform copy with page fault handling (current shared address space)
    unsafe {
        let dst = core::slice::from_raw_parts_mut(dst_ptr as *mut u8, src.len());
        dst.copy_from_slice(src);
    }
    
    Ok(())
}
```

#### Page Fault Handler
```rust
// Page fault error code bits
const PF_PRESENT: u64 = 1 << 0;    // Page was present
const PF_WRITE: u64 = 1 << 1;      // Write access
const PF_USER: u64 = 1 << 2;       // User mode access
const PF_RESERVED: u64 = 1 << 3;   // Reserved bit violation
const PF_INSTRUCTION: u64 = 1 << 4; // Instruction fetch

#[no_mangle]
extern "C" fn page_fault_handler() {
    let fault_addr: usize;
    let error_code: u64;
    
    // Read CR2 (fault address) and error code from stack
    unsafe {
        core::arch::asm!("mov {}, cr2", out(reg) fault_addr);
        // Error code is pushed by CPU, read from interrupt frame
        error_code = read_error_code_from_stack();
    }
    
    let cpu_id = unsafe { crate::arch::x86_64::smp::percpu::percpu_current().id };
    let current_pid = current_process_id().unwrap_or(0);
    
    // Detailed fault logging with CPU and process context
    serial_println!("[FAULT][cpu{} pid={}] Page fault at 0x{:x}, error: 0x{:x}", 
                   cpu_id, current_pid, fault_addr, error_code);
    
    // Decode error code for better diagnostics
    let present = error_code & PF_PRESENT != 0;
    let write = error_code & PF_WRITE != 0;
    let user = error_code & PF_USER != 0;
    let reserved = error_code & PF_RESERVED != 0;
    let instruction = error_code & PF_INSTRUCTION != 0;
    
    serial_println!("[FAULT] Details: present={}, write={}, user={}, reserved={}, instruction={}", 
                   present, write, user, reserved, instruction);
    
    // Determine fault handling based on context
    if user || fault_addr < USER_LIMIT {
        // User space fault or fault from user mode
        serial_println!("[FAULT] User space violation - terminating process {}", current_pid);
        
        // Terminate the faulting process
        if let Some(mut process_guard) = ProcessTable::get_process(current_pid) {
            let process = process_guard.process_mut();
            process.state = ProcessState::Zombie;
            process.exit_code = Some(-EFAULT as i32);
        }
        
        // Remove from scheduler and trigger reschedule
        crate::sched::remove_task(current_pid);
        crate::sched::yield_now(); // Switch to next task
        
    } else {
        // Kernel space fault - this is a kernel bug
        serial_println!("[FAULT] KERNEL PANIC: Kernel page fault at 0x{:x}", fault_addr);
        serial_println!("[FAULT] Error code: 0x{:x} ({}{}{}{}{})", 
                       error_code,
                       if present { "PRESENT " } else { "NOT_PRESENT " },
                       if write { "WRITE " } else { "READ " },
                       if user { "USER " } else { "KERNEL " },
                       if reserved { "RESERVED " } else { "" },
                       if instruction { "INSTRUCTION" } else { "DATA" });
        
        // Dump some registers for debugging
        dump_kernel_registers();
        panic!("[FAULT] Unrecoverable kernel page fault");
    }
}

// Register page fault handler in IDT with IST for safety
pub fn init_page_fault_handler() {
    unsafe {
        // Use IST 3 for page fault handler to avoid stack issues
        IDT.entries[14].set_handler_with_ist(
            page_fault_handler_wrapper as usize,
            KERNEL_CODE_SEG,
            3  // IST index
        );
    }
}

#[naked]
unsafe extern "C" fn page_fault_handler_wrapper() {
    core::arch::naked_asm!(
        // CPU has pushed error code and standard interrupt frame
        // Save all registers
        "push rax",
        "push rcx", 
        "push rdx",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9", 
        "push r10",
        "push r11",
        
        // Call handler
        "call {handler}",
        
        // Restore registers
        "pop r11",
        "pop r10", 
        "pop r9",
        "pop r8",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rax",
        
        // Skip error code and return
        "add rsp, 8",  // Skip error code
        "iretq",
        
        handler = sym page_fault_handler,
    )
}
```

## Data Models

### Process Table Structure
```rust
// Process table operations with fine-grained locking
impl ProcessTable {
    pub fn get_process(pid: ProcessId) -> Option<ProcessGuard> {
        if pid == 0 || pid >= MAX_PROCESSES { return None; }
        
        let entry = &PROCESS_TABLE[pid];
        let lock_guard = entry.lock.lock();
        
        if entry.process.is_some() {
            Some(ProcessGuard { entry, _lock: lock_guard })
        } else {
            None
        }
    }
    
    pub fn alloc_process_slot() -> Option<(ProcessId, ProcessSlotGuard)> {
        let pid = NEXT_PID.fetch_add(1, Ordering::Relaxed);
        if pid >= MAX_PROCESSES { return None; }
        
        let entry = &PROCESS_TABLE[pid];
        let lock_guard = entry.lock.lock();
        
        if entry.process.is_none() {
            Some((pid, ProcessSlotGuard { entry, _lock: lock_guard }))
        } else {
            // PID collision (very rare), try next
            None
        }
    }
}

// RAII guards for safe process access
pub struct ProcessGuard {
    entry: &'static ProcessTableEntry,
    _lock: SpinLockGuard<'static, ()>,
}

impl ProcessGuard {
    pub fn process(&self) -> &Process {
        self.entry.process.as_ref().unwrap()
    }
    
    pub fn process_mut(&mut self) -> &mut Process {
        // SAFETY: We hold the exclusive lock
        unsafe {
            let entry_mut = &mut *(self.entry as *const _ as *mut ProcessTableEntry);
            entry_mut.process.as_mut().unwrap()
        }
    }
}

pub struct ProcessSlotGuard {
    entry: &'static ProcessTableEntry,
    _lock: SpinLockGuard<'static, ()>,
}

impl ProcessSlotGuard {
    pub fn install_process(self, process: Process) {
        // SAFETY: We hold the exclusive lock and slot is empty
        unsafe {
            let entry_mut = &mut *(self.entry as *const _ as *mut ProcessTableEntry);
            entry_mut.process = Some(process);
        }
    }
}

// Helper to get current process without locking (read-only access)
pub fn current_process_id() -> Option<ProcessId> {
    unsafe {
        crate::arch::x86_64::smp::percpu::percpu_current().current_process
    }
}
```

### Memory Region Tracking
```rust
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start: usize,
    pub end: usize,
    pub flags: PageTableFlags,
    pub region_type: MemoryRegionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    Code,       // .text section
    Data,       // .data section
    Bss,        // .bss section
    Stack,      // User stack
    Heap,       // User heap (future)
}

impl Process {
    pub fn add_memory_region(&mut self, region: MemoryRegion) -> Result<(), ProcessError> {
        // Validate region doesn't overlap with existing regions
        for existing in &self.memory_regions {
            if region.overlaps(existing) {
                return Err(ProcessError::MemoryOverlap);
            }
        }
        
        self.memory_regions.push(region);
        Ok(())
    }
    
    pub fn find_memory_region(&self, addr: usize) -> Option<&MemoryRegion> {
        self.memory_regions.iter()
            .find(|region| addr >= region.start && addr < region.end)
    }
}
```

## Error Handling

### Error Types
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserModeError {
    // ELF loading errors
    InvalidElfFormat,
    UnsupportedElfType,
    InvalidEntryPoint,
    SegmentLoadFailed,
    
    // Process management errors
    ProcessNotFound,
    InvalidProcessState,
    OutOfProcessSlots,
    MemoryAllocationFailed,
    
    // Memory protection errors
    InvalidUserPointer,
    MemoryAccessViolation,
    PageFaultInUserSpace,
    
    // Syscall errors
    InvalidSyscallNumber,
    InvalidSyscallArguments,
    PermissionDenied,
    
    // General errors
    OutOfMemory,
    NotImplemented,
}

// Error code constants (POSIX-compatible)
pub const ENOSYS: isize = -38;  // Function not implemented
pub const EFAULT: isize = -14;  // Bad address
pub const ENOMEM: isize = -12;  // Out of memory
pub const EINVAL: isize = -22;  // Invalid argument
pub const EPERM: isize = -1;    // Operation not permitted
```

### Error Recovery
```rust
impl ErrorHandler {
    pub fn handle_user_fault(pid: ProcessId, fault_type: FaultType, addr: usize) {
        match fault_type {
            FaultType::PageFault => {
                serial_println!("[ERROR] Process {} page fault at 0x{:x}", pid, addr);
                self.terminate_process(pid, -EFAULT as i32);
            }
            FaultType::ProtectionViolation => {
                serial_println!("[ERROR] Process {} protection violation at 0x{:x}", pid, addr);
                self.terminate_process(pid, -EPERM as i32);
            }
            FaultType::InvalidInstruction => {
                serial_println!("[ERROR] Process {} invalid instruction", pid);
                self.terminate_process(pid, -EINVAL as i32);
            }
        }
    }
    
    fn terminate_process(&self, pid: ProcessId, exit_code: i32) {
        // Mark process as zombie
        if let Some(process) = PROCESS_TABLE.get_mut(pid) {
            process.state = ProcessState::Zombie;
            process.exit_code = Some(exit_code);
            
            // Remove from scheduler queues
            crate::sched::remove_task(pid);
            
            // Wake parent if waiting
            if let Some(parent_pid) = process.parent_pid {
                self.wake_waiting_parent(parent_pid);
            }
        }
    }
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_elf_header_parsing() {
        let elf_data = create_test_elf();
        let loader = ElfLoader::new();
        
        let header = loader.parse_elf_header(&elf_data).unwrap();
        assert_eq!(header.e_type, ET_EXEC);
        assert_eq!(header.e_machine, EM_X86_64);
    }
    
    #[test]
    fn test_process_creation() {
        let mut pm = ProcessManager::new();
        let pid = pm.create_process("test", test_entry_point).unwrap();
        
        let process = pm.get_process(pid).unwrap();
        assert_eq!(process.state, ProcessState::Ready);
        assert_eq!(process.name, b"test");
    }
    
    #[test]
    fn test_user_pointer_validation() {
        assert!(is_user_pointer_valid(0x1000));
        assert!(is_user_pointer_valid(USER_LIMIT - 1));
        assert!(!is_user_pointer_valid(0));
        assert!(!is_user_pointer_valid(USER_LIMIT));
        assert!(!is_user_pointer_valid(0xFFFF_8000_0000_0000));
    }
}
```

### Integration Tests
```rust
pub mod integration_tests {
    pub fn test_user_mode_transition() {
        // 1. Create init process
        let init_pid = create_init_process().unwrap();
        
        // 2. Transition to user mode
        transition_to_user_mode(init_pid).unwrap();
        
        // 3. Verify process is running in ring 3
        assert_eq!(get_current_privilege_level(), 3);
        
        // 4. Test syscall functionality
        let result = sys_write(1, "Hello from user mode!\n".as_ptr(), 22);
        assert!(result > 0);
    }
    
    pub fn test_process_lifecycle() {
        // Test fork/exec/exit/wait cycle
        let parent_pid = create_test_process().unwrap();
        
        // Fork
        let child_pid = sys_fork().unwrap();
        if child_pid == 0 {
            // Child process
            sys_exec("/bin/test_program").unwrap();
        } else {
            // Parent process
            let (waited_pid, exit_code) = sys_wait(child_pid).unwrap();
            assert_eq!(waited_pid, child_pid);
            assert_eq!(exit_code, 0);
        }
    }
}
```

### Stress Tests
```rust
pub fn stress_test_process_creation() {
    const NUM_PROCESSES: usize = 100;
    let mut pids = Vec::new();
    
    // Create many processes
    for i in 0..NUM_PROCESSES {
        let pid = create_test_process(&format!("test_{}", i)).unwrap();
        pids.push(pid);
    }
    
    // Verify all processes exist
    for pid in &pids {
        assert!(process_exists(*pid));
    }
    
    // Clean up
    for pid in pids {
        terminate_process(pid);
    }
}

pub fn stress_test_fork_chain() {
    const CHAIN_LENGTH: usize = 10;
    
    for i in 0..CHAIN_LENGTH {
        let child_pid = sys_fork().unwrap();
        if child_pid == 0 {
            // Child continues the chain
            if i < CHAIN_LENGTH - 1 {
                continue;
            } else {
                // Last child exits
                sys_exit(0);
            }
        } else {
            // Parent waits for child
            let (_, exit_code) = sys_wait(child_pid).unwrap();
            assert_eq!(exit_code, 0);
            break;
        }
    }
}
```

## Performance Considerations

### Context Switch Optimization
- Minimize register saves/restores in syscall path
- Use SWAPGS for fast kernel/user GS base switching
- Implement lazy FPU context switching
- Cache frequently accessed process data in per-CPU structures

### Memory Management Optimization
- Implement copy-on-write for fork() to reduce memory usage
- Use demand paging for ELF loading
- Implement memory region coalescing to reduce fragmentation
- Add memory usage tracking and limits per process

### SMP Scalability
- Per-CPU process scheduling queues (already implemented)
- Lock-free PID allocation using atomic operations
- Minimize global lock contention in process table
- Use RCU for read-heavy process table operations

### Syscall Performance
- Fast syscall/sysret path for common syscalls
- Batch syscall validation to reduce overhead
- Implement syscall restart mechanism for interrupted calls
- Add syscall performance counters and profiling

## Implementation Phases

To manage complexity and enable incremental testing, the implementation should follow these phases:

### Phase 6.1: Ring Transition Infrastructure
**Goal**: Establish basic kernel-user mode transitions

**Components**:
- New GDT with user segments (USER_CODE_SEG, USER_DATA_SEG, TSS_SEG)
- TSS setup with per-CPU kernel stacks and IST stacks
- Assembly trampoline (`user_entry.S`) for ring 0 → ring 3 transition
- Basic privilege level validation and canonical address checking

**Testing**: Verify successful transition to ring 3 and ability to return to kernel

### Phase 6.2: Syscall Interface
**Goal**: Enable user programs to call kernel services

**Components**:
- MSR configuration (EFER.SCE, STAR, LSTAR, SFMASK, GS_BASE)
- Fast syscall entry/exit assembly stub with SWAPGS
- Enhanced syscall dispatcher with R10-based calling convention
- Implementation of sys_write, sys_yield, sys_getpid for testing

**Testing**: User program successfully calls syscalls and receives correct responses

### Phase 6.3: ELF Loader and Init Process
**Goal**: Load and execute actual user programs from binary format

**Components**:
- ELF64 parser with validation and security checks
- Segment mapping with proper page flags (NX, USER, WRITABLE)
- User stack setup with guard pages
- Init process creation and "Hello from userland!" demonstration

**Testing**: Init process loads, executes, and prints message via syscall

### Phase 6.4: Process Management
**Goal**: Full process lifecycle management

**Components**:
- Process table with fine-grained locking
- sys_fork implementation with memory space duplication
- sys_exec implementation with ELF loading
- sys_exit and sys_wait for process termination and cleanup
- Page fault handler for memory protection

**Testing**: Multi-process scenarios, fork chains, process cleanup verification

This phased approach allows for testing and debugging at each stage while building toward the complete user-mode support system.

## Security Considerations

### Memory Protection
- NX bit enforcement prevents code execution on data pages
- User space limit (USER_LIMIT) prevents access to kernel memory
- Guard pages detect stack overflow and heap corruption
- Page fault handler terminates processes for memory violations

### Privilege Separation
- Ring 3 execution prevents direct hardware access
- Syscall interface provides controlled kernel service access
- TSS ensures proper kernel stack usage during privilege transitions
- Separate page tables per process provide memory isolation

### Input Validation
- All user pointers validated before kernel access
- ELF binaries validated for format correctness and security
- Syscall arguments checked for validity and bounds
- Canonical address validation prevents processor exceptions

This design provides a comprehensive foundation for implementing user-mode support in MelloOS while maintaining compatibility with the existing kernel infrastructure and ensuring proper security boundaries between user and kernel space.
// 
Handler for bad syscall returns (non-canonical addresses)
extern "C" fn handle_bad_syscall_return(bad_rip: u64, bad_rsp: u64) -> ! {
    let cpu_id = unsafe { crate::arch::x86_64::smp::percpu::percpu_current().id };
    let pid = current_process_id().unwrap_or(0);
    
    serial_println!("[SYSCALL][cpu{} pid={}] FATAL: Non-canonical return address", cpu_id, pid);
    serial_println!("[SYSCALL] Bad RIP: 0x{:x}, Bad RSP: 0x{:x}", bad_rip, bad_rsp);
    
    // Terminate the process
    if let Some(mut process_guard) = ProcessTable::get_process(pid) {
        let process = process_guard.process_mut();
        process.state = ProcessState::Zombie;
        process.exit_code = Some(-EFAULT as i32);
    }
    
    // Remove from scheduler and switch to next task
    crate::sched::remove_task(pid);
    crate::sched::yield_now();
    
    // Should never reach here
    panic!("[SYSCALL] Failed to switch away from bad process");
}

// Helper function to get current privilege level for testing
pub fn get_current_privilege_level() -> u8 {
    let cs: u16;
    unsafe { 
        core::arch::asm!("mov {0:x}, cs", out(reg) cs); 
    }
    (cs & 3) as u8
}

// Helper function to read current RIP for debugging
pub fn read_current_rip() -> u64 {
    let rip: u64;
    unsafe {
        core::arch::asm!(
            "lea {}, [rip]",
            out(reg) rip
        );
    }
    rip
}

// Enhanced syscall dispatcher with detailed logging
pub fn syscall_dispatcher_with_logging(
    syscall_id: usize,
    arg1: usize, arg2: usize, arg3: usize,
    arg4: usize, arg5: usize, arg6: usize,
) -> isize {
    // Get current CPU and process for detailed logging
    let cpu_id = unsafe { crate::arch::x86_64::smp::percpu::percpu_current().id };
    let pid = current_process_id().unwrap_or(0);
    let rip = read_current_rip();
    
    // Log syscall with CPU, PID, and RIP for debugging
    serial_println!("[SYSCALL][cpu{} pid={} rip=0x{:x}] {} ({})", 
                   cpu_id, pid, rip, syscall_name(syscall_id), syscall_id);
    
    // Call the actual dispatcher
    let result = syscall_dispatcher(syscall_id, arg1, arg2, arg3, arg4, arg5, arg6);
    
    // Log return value for debugging
    serial_println!("[SYSCALL][cpu{} pid={}] {} returned: {}", 
                   cpu_id, pid, syscall_name(syscall_id), result);
    
    result
}

// Syscall name helper for logging
fn syscall_name(id: usize) -> &'static str {
    match id {
        SYS_WRITE => "SYS_WRITE",
        SYS_EXIT => "SYS_EXIT", 
        SYS_FORK => "SYS_FORK",
        SYS_EXEC => "SYS_EXEC",
        SYS_WAIT => "SYS_WAIT",
        SYS_YIELD => "SYS_YIELD",
        SYS_GETPID => "SYS_GETPID",
        SYS_MMAP => "SYS_MMAP",
        SYS_BRK => "SYS_BRK",
        _ => "UNKNOWN",
    }
}

// Fast path optimization for sys_write (inline for small writes)
pub fn sys_write_fastpath(fd: usize, buf_ptr: usize, len: usize) -> isize {
    // Fast path for small writes to stdout
    if fd == 1 && len <= 256 && is_user_pointer_valid(buf_ptr) {
        unsafe {
            let buffer = core::slice::from_raw_parts(buf_ptr as *const u8, len);
            if let Ok(s) = core::str::from_utf8(buffer) {
                serial_print!("{}", s);
                return len as isize;
            }
        }
    }
    
    // Fall back to full sys_write implementation
    sys_write(fd, buf_ptr, len)
}
```