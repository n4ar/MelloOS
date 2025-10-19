//! Timer Interrupt Handling
//!
//! This module configures the hardware timer (PIT) and sets up the Interrupt
//! Descriptor Table (IDT) for timer interrupts. It handles periodic interrupts
//! that trigger the scheduler.

use x86_64::instructions::port::Port;
use core::sync::atomic::{AtomicUsize, Ordering};

/// PIT (Programmable Interval Timer) constants
const PIT_FREQUENCY: u32 = 1193182; // PIT base frequency in Hz
const PIT_COMMAND: u16 = 0x43;      // PIT command port
const PIT_CHANNEL_0: u16 = 0x40;    // PIT channel 0 data port

/// PIC (Programmable Interrupt Controller) constants
const PIC1_COMMAND: u16 = 0x20;     // Master PIC command port
const PIC1_DATA: u16 = 0x21;        // Master PIC data port
const PIC2_COMMAND: u16 = 0xA0;     // Slave PIC command port
const PIC2_DATA: u16 = 0xA1;        // Slave PIC data port

/// PIC initialization command words
const ICW1_INIT: u8 = 0x11;         // Initialize + ICW4 needed
const ICW4_8086: u8 = 0x01;         // 8086 mode

/// IDT Entry structure for manual IDT manipulation
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const fn new() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }
    
    fn set_handler(&mut self, handler: usize, selector: u16) {
        self.offset_low = (handler & 0xFFFF) as u16;
        self.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFFFFFF) as u32;
        self.selector = selector;
        self.ist = 0;
        // Type: Interrupt Gate (0xE), DPL=0, Present=1
        self.type_attr = 0x8E;
        self.reserved = 0;
    }
    
    fn set_handler_user(&mut self, handler: usize, selector: u16) {
        self.offset_low = (handler & 0xFFFF) as u16;
        self.offset_mid = ((handler >> 16) & 0xFFFF) as u16;
        self.offset_high = ((handler >> 32) & 0xFFFFFFFF) as u32;
        self.selector = selector;
        self.ist = 0;
        // Type: Interrupt Gate (0xE), DPL=3 (user-accessible), Present=1
        self.type_attr = 0xEE;
        self.reserved = 0;
    }
}

/// IDT Table structure
#[repr(C, packed)]
struct IdtTable {
    entries: [IdtEntry; 256],
}

impl IdtTable {
    const fn new() -> Self {
        Self {
            entries: [IdtEntry::new(); 256],
        }
    }
}

/// IDT Pointer structure for loading the IDT
#[repr(C, packed)]
struct IdtPointer {
    limit: u16,
    base: u64,
}

/// Global IDT
static mut IDT: IdtTable = IdtTable::new();

/// Counter for timer interrupts (for testing and debugging)
static TIMER_TICKS: AtomicUsize = AtomicUsize::new(0);

/// Initialize the Interrupt Descriptor Table (IDT)
///
/// This function:
/// 1. Creates a new IDT
/// 2. Registers the timer interrupt handler at vector 32 (IRQ0)
/// 3. Loads the IDT into the CPU
///
/// # Safety
/// This function is unsafe because it modifies the global IDT and loads it into the CPU.
/// It must be called only once during kernel initialization, before enabling interrupts.
pub unsafe fn init_idt() {
    use crate::serial_println;
    
    serial_println!("[TIMER] Setting up IDT...");
    
    // Get the code segment selector (0x08 for kernel code segment in most setups)
    let code_selector: u16 = 0x28; // Limine sets up GDT with kernel code at 0x28
    
    // Validate handler address
    let handler_addr = timer_interrupt_handler_wrapper as usize;
    if handler_addr == 0 {
        panic!("[TIMER] CRITICAL: Timer interrupt handler address is null");
    }
    
    // Set timer interrupt handler at vector 32 (IRQ0 after PIC remapping)
    IDT.entries[32].set_handler(handler_addr, code_selector);
    
    // Validate IDT setup
    if IDT.entries[32].offset_low == 0 && IDT.entries[32].offset_mid == 0 && IDT.entries[32].offset_high == 0 {
        panic!("[TIMER] CRITICAL: Failed to set timer interrupt handler in IDT");
    }
    
    // Set syscall handler at vector 0x80 (128)
    let syscall_handler_addr = crate::sys::syscall::syscall_entry as usize;
    if syscall_handler_addr == 0 {
        panic!("[TIMER] CRITICAL: Syscall handler address is null");
    }
    
    IDT.entries[0x80].set_handler_user(syscall_handler_addr, code_selector);
    
    // Validate syscall handler setup
    if IDT.entries[0x80].offset_low == 0 && IDT.entries[0x80].offset_mid == 0 && IDT.entries[0x80].offset_high == 0 {
        panic!("[TIMER] CRITICAL: Failed to set syscall handler in IDT");
    }
    
    serial_println!("[TIMER] Syscall handler registered at vector 0x80 (DPL=3)");
    
    // Create IDT pointer
    let idt_ptr = IdtPointer {
        limit: (core::mem::size_of::<IdtTable>() - 1) as u16,
        base: &raw const IDT as u64,
    };
    
    // Validate IDT pointer
    if idt_ptr.base == 0 {
        panic!("[TIMER] CRITICAL: IDT base address is null");
    }
    
    // Load the IDT using lidt instruction
    core::arch::asm!(
        "lidt [{}]",
        in(reg) &idt_ptr,
        options(readonly, nostack, preserves_flags)
    );
    
    serial_println!("[TIMER] IDT loaded successfully");
}

/// Remap the PIC (Programmable Interrupt Controller)
///
/// This function remaps the PIC IRQs to avoid conflicts with CPU exceptions:
/// - Master PIC (IRQ 0-7) → Vectors 32-39 (0x20-0x27)
/// - Slave PIC (IRQ 8-15) → Vectors 40-47 (0x28-0x2F)
///
/// After remapping, it masks all IRQs except IRQ0 (timer).
///
/// # Safety
/// This function is unsafe because it directly manipulates hardware ports.
/// It must be called during kernel initialization before enabling interrupts.
pub unsafe fn remap_pic() {
    use crate::serial_println;
    
    serial_println!("[TIMER] Remapping PIC...");
    
    let mut pic1_command = Port::<u8>::new(PIC1_COMMAND);
    let mut pic1_data = Port::<u8>::new(PIC1_DATA);
    let mut pic2_command = Port::<u8>::new(PIC2_COMMAND);
    let mut pic2_data = Port::<u8>::new(PIC2_DATA);
    
    // Save masks (not used in this implementation, but good practice)
    let _mask1 = pic1_data.read();
    let _mask2 = pic2_data.read();
    
    // Start initialization sequence
    pic1_command.write(ICW1_INIT);
    io_wait();
    pic2_command.write(ICW1_INIT);
    io_wait();
    
    // Set vector offsets
    pic1_data.write(32); // Master PIC vector offset (32-39)
    io_wait();
    pic2_data.write(40); // Slave PIC vector offset (40-47)
    io_wait();
    
    // Tell Master PIC that there is a slave PIC at IRQ2
    pic1_data.write(4);
    io_wait();
    
    // Tell Slave PIC its cascade identity
    pic2_data.write(2);
    io_wait();
    
    // Set 8086 mode
    pic1_data.write(ICW4_8086);
    io_wait();
    pic2_data.write(ICW4_8086);
    io_wait();
    
    // Restore saved masks (or set new ones)
    // Mask all IRQs except IRQ0 (timer)
    pic1_data.write(0xFE); // 11111110 - only IRQ0 enabled
    io_wait();
    pic2_data.write(0xFF); // 11111111 - all slave IRQs disabled
    io_wait();
    
    serial_println!("[TIMER] PIC remapped: Master=32-39, Slave=40-47");
    serial_println!("[TIMER] IRQ0 (timer) enabled, all others masked");
}

/// Small delay for I/O operations
///
/// This function performs a small delay by writing to an unused port.
/// This is necessary because some hardware needs time between I/O operations.
#[inline]
unsafe fn io_wait() {
    let mut port = Port::<u8>::new(0x80);
    port.write(0);
}

/// Configure the PIT (Programmable Interval Timer)
///
/// This function configures the PIT to generate interrupts at the specified frequency.
/// It uses PIT mode 3 (square wave generator) on channel 0.
///
/// # Arguments
/// * `frequency` - Desired interrupt frequency in Hz (e.g., 100 for 100 Hz)
///
/// # Safety
/// This function is unsafe because it directly manipulates hardware ports.
/// It must be called after remapping the PIC and setting up the IDT.
///
/// # Panics
/// Panics if the frequency is too low (< 18 Hz) or too high (> 1193182 Hz)
pub unsafe fn init_pit_timer(frequency: u32) {
    use crate::serial_println;
    
    serial_println!("[TIMER] Configuring PIT for {} Hz...", frequency);
    
    // Validate frequency range
    if frequency == 0 {
        panic!("[TIMER] CRITICAL: Frequency cannot be zero");
    }
    
    if frequency > PIT_FREQUENCY {
        panic!("[TIMER] CRITICAL: Frequency too high! Maximum is {} Hz", PIT_FREQUENCY);
    }
    
    // Calculate divisor for desired frequency
    let divisor = PIT_FREQUENCY / frequency;
    
    if divisor > 65535 {
        panic!("[TIMER] CRITICAL: Frequency too low! Minimum is {} Hz", PIT_FREQUENCY / 65535);
    }
    
    if divisor == 0 {
        panic!("[TIMER] CRITICAL: Calculated divisor is zero");
    }
    
    let mut command_port = Port::<u8>::new(PIT_COMMAND);
    let mut channel0_port = Port::<u8>::new(PIT_CHANNEL_0);
    
    // Set PIT to mode 3 (square wave generator)
    // Command byte: 00 11 011 0
    // - Channel 0 (00)
    // - Access mode: lobyte/hibyte (11)
    // - Operating mode 3: square wave (011)
    // - Binary mode (0)
    command_port.write(0x36);
    
    // Write divisor (low byte, then high byte)
    channel0_port.write((divisor & 0xFF) as u8);
    channel0_port.write(((divisor >> 8) & 0xFF) as u8);
    
    serial_println!("[TIMER] PIT configured with divisor {} ({} Hz)", divisor, frequency);
}

/// Send End of Interrupt (EOI) signal to PIC
///
/// This function sends an EOI signal to the PIC master controller,
/// indicating that the interrupt has been handled and the PIC can
/// send the next interrupt.
///
/// # Notes
/// - For IRQ0-7 (master PIC), only send EOI to master
/// - For IRQ8-15 (slave PIC), send EOI to both slave and master
///   (not implemented yet as we only use IRQ0)
///
/// # Safety
/// This function is unsafe because it directly manipulates hardware ports.
unsafe fn send_eoi() {
    let mut pic1_command = Port::<u8>::new(PIC1_COMMAND);
    
    // Send EOI to master PIC
    // For IRQ0 (timer), we only need to send to master
    // If we use IRQ >= 8 in the future, we need to send to slave PIC too:
    // Port::<u8>::new(PIC2_COMMAND).write(0x20);
    pic1_command.write(0x20);
}

/// Timer interrupt handler wrapper
///
/// This is a naked function that saves/restores registers and calls the actual handler.
/// We use a naked function because we're manually managing the IDT.
#[unsafe(naked)]
extern "C" fn timer_interrupt_handler_wrapper() {
    core::arch::naked_asm!(
        // The CPU has already pushed SS, RSP, RFLAGS, CS, RIP
        // We need to save all other registers
        
        "push rax",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        
        // Call the actual handler
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
        
        // Return from interrupt (pops RIP, CS, RFLAGS, RSP, SS)
        "iretq",
        
        handler = sym timer_interrupt_handler,
    )
}

/// Timer interrupt handler
///
/// This function is called by the wrapper when a timer interrupt (IRQ0) occurs.
/// It:
/// 1. Sends EOI to the PIC (to allow next interrupt)
/// 2. Increments the tick counter
/// 3. Calls the scheduler tick function
///
/// # Notes
/// - The CPU automatically disables interrupts (IF=0) when entering this handler
/// - The scheduler tick() function performs a context switch and doesn't return
/// - This is a "tail-switch" - we don't return to this handler
extern "C" fn timer_interrupt_handler() {
    // Increment tick counter (for testing and debugging)
    TIMER_TICKS.fetch_add(1, Ordering::Relaxed);
    
    // Send EOI to PIC first (so it can send next interrupt)
    unsafe {
        send_eoi();
    }
    
    // Call scheduler tick (this performs context switch and doesn't return)
    crate::sched::tick();
    
    // Note: We never reach here because tick() does a tail-switch
    // The next task will continue from where it was interrupted
}

/// Initialize the timer interrupt system
///
/// This function performs the complete timer initialization:
/// 1. Sets up the IDT with the timer interrupt handler
/// 2. Remaps the PIC to avoid conflicts with CPU exceptions
/// 3. Configures the PIT to generate interrupts at the specified frequency
///
/// # Arguments
/// * `frequency` - Desired interrupt frequency in Hz (recommended: 100 Hz)
///
/// # Safety
/// This function is unsafe because it modifies hardware configuration.
/// It must be called during kernel initialization, after memory management
/// is set up but before enabling interrupts.
///
/// # Example
/// ```no_run
/// unsafe {
///     init_timer(100); // 100 Hz = 10ms per tick
/// }
/// ```
pub unsafe fn init_timer(frequency: u32) {
    use crate::serial_println;
    
    serial_println!("[TIMER] Initializing timer interrupt system...");
    
    // 1. Set up IDT
    init_idt();
    
    // 2. Remap PIC
    remap_pic();
    
    // 3. Configure PIT
    init_pit_timer(frequency);
    
    serial_println!("[TIMER] Timer initialized at {} Hz", frequency);
}

/// Get the current tick count
///
/// Returns the number of timer interrupts that have occurred since boot.
/// Useful for testing and debugging.
pub fn get_tick_count() -> usize {
    TIMER_TICKS.load(Ordering::Relaxed)
}

// ============================================================================
// APIC Timer Interrupt Handler (for SMP)
// ============================================================================

/// APIC timer interrupt handler wrapper
///
/// This is a naked function that saves/restores registers and calls the actual handler.
/// This handler is used for APIC timer interrupts (vector 0x20) in SMP mode.
#[unsafe(naked)]
extern "C" fn apic_timer_interrupt_handler_wrapper() {
    core::arch::naked_asm!(
        // The CPU has already pushed SS, RSP, RFLAGS, CS, RIP
        // We need to save all other registers
        
        "push rax",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        
        // Call the actual handler
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
        
        // Return from interrupt (pops RIP, CS, RFLAGS, RSP, SS)
        "iretq",
        
        handler = sym apic_timer_interrupt_handler,
    )
}

/// APIC timer interrupt handler
///
/// This function is called when an APIC timer interrupt (vector 0x20) occurs.
/// It:
/// 1. Increments the per-CPU tick counter
/// 2. Sends EOI to the Local APIC
/// 3. Calls the scheduler tick function for the current core
///
/// # Notes
/// - The CPU automatically disables interrupts (IF=0) when entering this handler
/// - The scheduler tick() function performs a context switch and doesn't return
/// - This is a "tail-switch" - we don't return to this handler
extern "C" fn apic_timer_interrupt_handler() {
    use crate::arch::x86_64::smp::percpu::percpu_current_mut;
    use crate::arch::x86_64::apic::LocalApic;
    use crate::arch::x86_64::acpi::get_madt_info;
    use core::sync::atomic::Ordering;
    
    // Get current CPU's per-CPU data
    let percpu = unsafe { percpu_current_mut() };
    
    // Increment per-CPU tick counter
    percpu.ticks.fetch_add(1, Ordering::Relaxed);
    
    // Also increment global tick counter for compatibility
    TIMER_TICKS.fetch_add(1, Ordering::Relaxed);
    
    // Send EOI to Local APIC
    unsafe {
        let madt_info = get_madt_info().expect("MADT info not available");
        let mut lapic = LocalApic::new(madt_info.lapic_address);
        lapic.eoi();
    }
    
    // Call scheduler tick (this performs context switch and doesn't return)
    crate::sched::tick();
    
    // Note: We never reach here because tick() does a tail-switch
}

/// Initialize APIC timer interrupt handler in IDT
///
/// This function registers the APIC timer interrupt handler at vector 0x20
/// in the IDT. It should be called after init_idt() but before enabling
/// the APIC timer.
///
/// # Safety
/// This function is unsafe because it modifies the global IDT.
/// It must be called during kernel initialization.
pub unsafe fn init_apic_timer_handler() {
    use crate::serial_println;
    
    serial_println!("[TIMER] Registering APIC timer handler at vector 0x20...");
    
    // Get the code segment selector
    let code_selector: u16 = 0x28; // Limine sets up GDT with kernel code at 0x28
    
    // Validate handler address
    let handler_addr = apic_timer_interrupt_handler_wrapper as usize;
    if handler_addr == 0 {
        panic!("[TIMER] CRITICAL: APIC timer interrupt handler address is null");
    }
    
    // Set APIC timer interrupt handler at vector 0x20 (32)
    IDT.entries[32].set_handler(handler_addr, code_selector);
    
    // Validate IDT setup
    if IDT.entries[32].offset_low == 0 && IDT.entries[32].offset_mid == 0 && IDT.entries[32].offset_high == 0 {
        panic!("[TIMER] CRITICAL: Failed to set APIC timer interrupt handler in IDT");
    }
    
    serial_println!("[TIMER] APIC timer handler registered successfully");
}

/// Manual test functions for timer interrupt system
#[cfg(not(test))]
pub mod manual_tests {
    use super::*;
    use crate::serial_println;
    
    /// Test that timer interrupt fires
    ///
    /// This test initializes the timer and waits for interrupts to occur.
    /// It checks that the tick counter increments.
    pub fn test_timer_interrupt_fires() {
        serial_println!("[TEST] Testing timer interrupt fires...");
        
        unsafe {
            // Initialize timer at 100 Hz
            init_timer(100);
            
            // Enable interrupts
            core::arch::asm!("sti");
            
            // Wait a bit for interrupts to fire
            let start_ticks = get_tick_count();
            serial_println!("[TEST] Initial tick count: {}", start_ticks);
            
            // Busy wait
            for _ in 0..10_000_000 {
                core::arch::asm!("nop");
            }
            
            let end_ticks = get_tick_count();
            serial_println!("[TEST] Final tick count: {}", end_ticks);
            
            // Disable interrupts
            core::arch::asm!("cli");
            
            if end_ticks > start_ticks {
                serial_println!("[TEST] ✓ Timer interrupt fires test passed!");
                serial_println!("[TEST]   {} interrupts occurred", end_ticks - start_ticks);
            } else {
                serial_println!("[TEST] ✗ Timer interrupt fires test FAILED!");
                serial_println!("[TEST]   No interrupts occurred");
            }
        }
    }
    
    /// Test that interrupt handler is called
    ///
    /// This test verifies that the timer interrupt handler is being invoked
    /// by checking the tick counter increments over time.
    pub fn test_interrupt_handler_called() {
        serial_println!("[TEST] Testing interrupt handler is called...");
        
        unsafe {
            // Enable interrupts
            core::arch::asm!("sti");
            
            let tick1 = get_tick_count();
            
            // Wait
            for _ in 0..5_000_000 {
                core::arch::asm!("nop");
            }
            
            let tick2 = get_tick_count();
            
            // Wait again
            for _ in 0..5_000_000 {
                core::arch::asm!("nop");
            }
            
            let tick3 = get_tick_count();
            
            // Disable interrupts
            core::arch::asm!("cli");
            
            serial_println!("[TEST] Tick counts: {} -> {} -> {}", tick1, tick2, tick3);
            
            if tick3 > tick2 && tick2 > tick1 {
                serial_println!("[TEST] ✓ Interrupt handler called test passed!");
            } else {
                serial_println!("[TEST] ✗ Interrupt handler called test FAILED!");
            }
        }
    }
    
    /// Test that tick counter increments
    ///
    /// This test verifies that the TIMER_TICKS counter is properly
    /// incremented by the interrupt handler.
    pub fn test_tick_counter_increments() {
        serial_println!("[TEST] Testing tick counter increments...");
        
        unsafe {
            // Enable interrupts
            core::arch::asm!("sti");
            
            let start = get_tick_count();
            serial_println!("[TEST] Starting tick count: {}", start);
            
            // Wait for several ticks
            for _ in 0..20_000_000 {
                core::arch::asm!("nop");
            }
            
            let end = get_tick_count();
            serial_println!("[TEST] Ending tick count: {}", end);
            
            // Disable interrupts
            core::arch::asm!("cli");
            
            let diff = end - start;
            
            if diff > 0 {
                serial_println!("[TEST] ✓ Tick counter increments test passed!");
                serial_println!("[TEST]   Counter increased by {}", diff);
            } else {
                serial_println!("[TEST] ✗ Tick counter increments test FAILED!");
            }
        }
    }
    
    /// Run all timer tests
    pub fn run_all_tests() {
        serial_println!("[TEST] ========================================");
        serial_println!("[TEST] Running Timer Interrupt Tests");
        serial_println!("[TEST] ========================================");
        
        test_timer_interrupt_fires();
        test_interrupt_handler_called();
        test_tick_counter_increments();
        
        serial_println!("[TEST] ========================================");
        serial_println!("[TEST] All Timer Tests Completed!");
        serial_println!("[TEST] ========================================");
    }
}
