//! IRQ (Interrupt Request) Management System
//!
//! This module provides interrupt handling infrastructure for device drivers,
//! including IRQ handler registration, IOAPIC routing, and interrupt dispatch.
//!
//! # Features
//! - IRQ handler registration with CPU affinity support
//! - IOAPIC initialization and configuration
//! - Interrupt dispatch to registered handlers
//! - IRQ logging with CPU core ID tracking
//! - SMP-safe interrupt handling

use crate::serial_println;
use crate::sync::SpinLock;
use core::ptr::{read_volatile, write_volatile};

// ============================================================================
// IOAPIC Constants
// ============================================================================

/// Default IOAPIC base address (from ACPI MADT)
const IOAPIC_BASE: usize = 0xFEC00000;

/// IOAPIC register select offset
const IOAPIC_REG_SELECT: usize = 0x00;

/// IOAPIC register window offset
const IOAPIC_REG_WINDOW: usize = 0x10;

/// IOAPIC ID register
const IOAPIC_REG_ID: u32 = 0x00;

/// IOAPIC version register
const IOAPIC_REG_VER: u32 = 0x01;

/// IOAPIC redirection table base register
const IOAPIC_REG_REDTBL_BASE: u32 = 0x10;

// Redirection entry flags
const IOAPIC_DELIVERY_MODE_FIXED: u64 = 0x0 << 8;
const IOAPIC_DEST_MODE_PHYSICAL: u64 = 0x0 << 11;
const IOAPIC_PIN_POLARITY_HIGH: u64 = 0x0 << 13;
const IOAPIC_TRIGGER_MODE_EDGE: u64 = 0x0 << 15;
const IOAPIC_MASK: u64 = 0x1 << 16;

// ============================================================================
// IRQ Handler Types
// ============================================================================

/// Type alias for IRQ handler functions
pub type IrqHandler = fn();

/// Maximum number of IRQ lines supported
const MAX_IRQS: usize = 256;

// ============================================================================
// Global IRQ Handler Registry
// ============================================================================

/// Global registry of IRQ handlers
static IRQ_HANDLERS: SpinLock<[Option<IrqHandler>; MAX_IRQS]> = SpinLock::new([None; MAX_IRQS]);

/// Flag indicating whether IOAPIC has been initialized
static IOAPIC_INITIALIZED: SpinLock<bool> = SpinLock::new(false);

/// IRQ statistics per CPU (IRQ number -> count per CPU)
/// Format: [IRQ][CPU] = count
static IRQ_STATS: SpinLock<[[u64; 8]; MAX_IRQS]> = SpinLock::new([[0; 8]; MAX_IRQS]);

// ============================================================================
// IOAPIC Driver
// ============================================================================

/// IOAPIC (I/O Advanced Programmable Interrupt Controller) driver
struct IoApic {
    base_addr: usize,
}

impl IoApic {
    /// Create a new IOAPIC instance
    ///
    /// # Safety
    /// The caller must ensure the base address is valid and properly mapped
    unsafe fn new(base_addr: usize) -> Self {
        Self { base_addr }
    }

    /// Read from an IOAPIC register
    ///
    /// # Arguments
    /// * `reg` - Register index to read from
    ///
    /// # Returns
    /// The 32-bit value from the register
    unsafe fn read(&self, reg: u32) -> u32 {
        let select_ptr = self.base_addr as *mut u32;
        let window_ptr = (self.base_addr + IOAPIC_REG_WINDOW) as *const u32;

        write_volatile(select_ptr, reg);
        read_volatile(window_ptr)
    }

    /// Write to an IOAPIC register
    ///
    /// # Arguments
    /// * `reg` - Register index to write to
    /// * `value` - 32-bit value to write
    unsafe fn write(&mut self, reg: u32, value: u32) {
        let select_ptr = self.base_addr as *mut u32;
        let window_ptr = (self.base_addr + IOAPIC_REG_WINDOW) as *mut u32;

        write_volatile(select_ptr, reg);
        write_volatile(window_ptr, value);
    }

    /// Get the maximum number of redirection entries
    ///
    /// # Returns
    /// The number of IRQ lines supported by this IOAPIC
    unsafe fn max_redirects(&self) -> u8 {
        let ver = self.read(IOAPIC_REG_VER);
        ((ver >> 16) & 0xFF) as u8
    }

    /// Set an IRQ redirection entry
    ///
    /// # Arguments
    /// * `irq` - IRQ number (0-23 typically)
    /// * `vector` - Interrupt vector to map to (typically IRQ + 32)
    /// * `dest_apic_id` - Target CPU's APIC ID
    unsafe fn set_redirect(&mut self, irq: u8, vector: u8, dest_apic_id: u8) {
        // Calculate redirection table entry registers
        let low_reg = IOAPIC_REG_REDTBL_BASE + (irq as u32 * 2);
        let high_reg = low_reg + 1;

        // Build redirection entry
        // Low 32 bits: vector, delivery mode, destination mode, polarity, trigger mode
        let mut entry_low: u64 = vector as u64;
        entry_low |= IOAPIC_DELIVERY_MODE_FIXED;
        entry_low |= IOAPIC_DEST_MODE_PHYSICAL;
        entry_low |= IOAPIC_PIN_POLARITY_HIGH;
        entry_low |= IOAPIC_TRIGGER_MODE_EDGE;
        // Don't set MASK bit - we want the interrupt enabled

        // High 32 bits: destination APIC ID (bits 56-63 of full entry)
        let entry_high: u64 = (dest_apic_id as u64) << 24;

        // Write the redirection entry
        self.write(high_reg, (entry_high >> 32) as u32);
        self.write(low_reg, entry_low as u32);
    }

    /// Mask (disable) an IRQ
    ///
    /// # Arguments
    /// * `irq` - IRQ number to mask
    unsafe fn mask_irq(&mut self, irq: u8) {
        let low_reg = IOAPIC_REG_REDTBL_BASE + (irq as u32 * 2);
        let mut entry = self.read(low_reg) as u64;
        entry |= IOAPIC_MASK;
        self.write(low_reg, entry as u32);
    }

    /// Unmask (enable) an IRQ
    ///
    /// # Arguments
    /// * `irq` - IRQ number to unmask
    unsafe fn unmask_irq(&mut self, irq: u8) {
        let low_reg = IOAPIC_REG_REDTBL_BASE + (irq as u32 * 2);
        let mut entry = self.read(low_reg) as u64;
        entry &= !IOAPIC_MASK;
        self.write(low_reg, entry as u32);
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Initialize IOAPIC routing before driver registration
///
/// This function must be called during kernel initialization before any
/// device drivers attempt to register IRQ handlers. It sets up the IOAPIC
/// hardware and prepares it for interrupt routing.
///
/// # Safety
/// This function should only be called once during kernel initialization
/// with interrupts disabled.
pub fn init_ioapic_routing() {
    let mut initialized = IOAPIC_INITIALIZED.lock();
    if *initialized {
        serial_println!("[IRQ] IOAPIC already initialized, skipping");
        return;
    }

    serial_println!("[IRQ] Initializing IOAPIC routing for device drivers");

    unsafe {
        // Map IOAPIC MMIO region if not already mapped
        // Note: In a full implementation, we would use the memory manager
        // to create a proper mapping. For now, we assume identity mapping
        // or that the bootloader has mapped it.

        let mut ioapic = IoApic::new(IOAPIC_BASE);

        // Read IOAPIC version and max redirects
        let max_redirects = ioapic.max_redirects();
        serial_println!(
            "[IRQ] IOAPIC supports {} redirection entries",
            max_redirects + 1
        );

        // Initialize all redirection entries to masked state
        for irq in 0..=max_redirects {
            ioapic.mask_irq(irq);
        }

        serial_println!("[IRQ] IOAPIC initialization complete");
    }

    *initialized = true;
}

/// Register an IRQ handler without CPU affinity
///
/// This is a convenience wrapper around `register_irq_handler_affinity`
/// that lets the system choose which CPU to route the interrupt to.
///
/// # Arguments
/// * `irq` - IRQ number to register (0-255)
/// * `handler` - Function to call when the interrupt occurs
///
/// # Returns
/// * `Ok(())` - Handler registered successfully
/// * `Err(&str)` - Registration failed (IRQ already registered or invalid)
///
/// # Example
/// ```
/// fn keyboard_irq_handler() {
///     // Handle keyboard interrupt
/// }
///
/// register_irq_handler(1, keyboard_irq_handler)?;
/// ```
pub fn register_irq_handler(irq: u8, handler: IrqHandler) -> Result<(), &'static str> {
    register_irq_handler_affinity(irq, handler, None)
}

/// Register an IRQ handler with optional CPU affinity
///
/// This function registers an interrupt handler for a specific IRQ line
/// and configures the IOAPIC to route interrupts to the specified CPU.
///
/// # Arguments
/// * `irq` - IRQ number to register (0-255)
/// * `handler` - Function to call when the interrupt occurs
/// * `cpu_affinity` - Optional target CPU core ID (None = system chooses)
///
/// # Returns
/// * `Ok(())` - Handler registered successfully
/// * `Err(&str)` - Registration failed
///
/// # Example
/// ```
/// // Route keyboard interrupts to CPU 0
/// register_irq_handler_affinity(1, keyboard_irq_handler, Some(0))?;
/// ```
pub fn register_irq_handler_affinity(
    irq: u8,
    handler: IrqHandler,
    cpu_affinity: Option<u8>,
) -> Result<(), &'static str> {
    // Check if IOAPIC is initialized
    let initialized = IOAPIC_INITIALIZED.lock();
    if !*initialized {
        return Err("IOAPIC not initialized - call init_ioapic_routing() first");
    }
    drop(initialized);

    // Register the handler
    let mut handlers = IRQ_HANDLERS.lock();

    if handlers[irq as usize].is_some() {
        return Err("IRQ already registered");
    }

    handlers[irq as usize] = Some(handler);
    drop(handlers);

    // Determine target CPU
    let target_cpu = cpu_affinity.unwrap_or(0);

    serial_println!(
        "[IRQ] Registered IRQ {} handler (CPU affinity: {})",
        irq,
        target_cpu
    );

    // Configure IOAPIC routing
    configure_ioapic_irq(irq, target_cpu);

    Ok(())
}

/// Unregister an IRQ handler
///
/// This function removes a previously registered IRQ handler and masks
/// the interrupt in the IOAPIC.
///
/// # Arguments
/// * `irq` - IRQ number to unregister
pub fn unregister_irq_handler(irq: u8) {
    let mut handlers = IRQ_HANDLERS.lock();
    handlers[irq as usize] = None;
    drop(handlers);

    // Mask the IRQ in IOAPIC
    unsafe {
        let mut ioapic = IoApic::new(IOAPIC_BASE);
        ioapic.mask_irq(irq);
    }

    serial_println!("[IRQ] Unregistered IRQ {} handler", irq);
}

/// Handle an IRQ interrupt
///
/// This function is called by the interrupt dispatcher when a hardware
/// interrupt occurs. It looks up the registered handler and invokes it,
/// logging the IRQ number and handling CPU core ID.
///
/// # Arguments
/// * `irq` - IRQ number that fired
///
/// # Note
/// This function must be called from interrupt context. The caller is
/// responsible for sending EOI to the LAPIC after this function returns.
pub fn handle_irq(irq: u8) {
    let handlers = IRQ_HANDLERS.lock();

    if let Some(handler) = handlers[irq as usize] {
        let cpu_id = crate::arch::x86_64::smp::percpu::percpu_current().id;
        
        // Update IRQ statistics
        {
            let mut stats = IRQ_STATS.lock();
            if (cpu_id as usize) < 8 {
                stats[irq as usize][cpu_id as usize] += 1;
            }
        }
        
        // Note: Using trace-level logging here would be too verbose
        // Only log in debug builds
        #[cfg(debug_assertions)]
        serial_println!("[IRQ] Handling IRQ {} on CPU {}", irq, cpu_id);

        // Drop the lock before calling the handler to avoid deadlock
        drop(handlers);

        // Call the registered handler
        handler();
    } else {
        serial_println!("[IRQ] Warning: Unhandled IRQ {}", irq);
    }
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

/// Configure IOAPIC for IRQ routing with CPU affinity
///
/// This function sets up the IOAPIC redirection entry to route a specific
/// IRQ to a specific CPU core.
///
/// # Arguments
/// * `irq` - IRQ number to configure
/// * `target_cpu` - Target CPU's APIC ID
fn configure_ioapic_irq(irq: u8, target_cpu: u8) {
    unsafe {
        let mut ioapic = IoApic::new(IOAPIC_BASE);

        // Map IRQ to interrupt vector (IRQ + 32 for x86_64)
        let vector = irq + 32;

        // Set up redirection entry
        ioapic.set_redirect(irq, vector, target_cpu);

        // Unmask the IRQ to enable it
        ioapic.unmask_irq(irq);

        #[cfg(debug_assertions)]
        serial_println!(
            "[IRQ] Configured IOAPIC: IRQ {} -> Vector {} -> CPU {}",
            irq,
            vector,
            target_cpu
        );
    }
}

// ============================================================================
// Public Helper Functions
// ============================================================================

/// Check if an IRQ has a registered handler
///
/// # Arguments
/// * `irq` - IRQ number to check
///
/// # Returns
/// `true` if a handler is registered, `false` otherwise
pub fn is_irq_registered(irq: u8) -> bool {
    let handlers = IRQ_HANDLERS.lock();
    handlers[irq as usize].is_some()
}

/// Get the number of registered IRQ handlers
///
/// # Returns
/// The count of currently registered handlers
pub fn registered_irq_count() -> usize {
    let handlers = IRQ_HANDLERS.lock();
    handlers.iter().filter(|h| h.is_some()).count()
}

/// Get IRQ statistics for a specific IRQ across all CPUs
///
/// # Arguments
/// * `irq` - IRQ number to query
/// * `cpu_stats` - Output buffer for per-CPU counts (must be at least 8 elements)
///
/// # Returns
/// `true` if the IRQ is registered, `false` otherwise
pub fn get_irq_stats(irq: u8, cpu_stats: &mut [u64; 8]) -> bool {
    let handlers = IRQ_HANDLERS.lock();
    let is_registered = handlers[irq as usize].is_some();
    drop(handlers);
    
    if is_registered {
        let stats = IRQ_STATS.lock();
        cpu_stats.copy_from_slice(&stats[irq as usize]);
        true
    } else {
        false
    }
}

/// Get all IRQ statistics
///
/// # Arguments
/// * `buffer` - Output buffer for IRQ statistics (IRQ number, CPU counts)
/// * `max_entries` - Maximum number of entries to return
///
/// # Returns
/// The number of registered IRQs with statistics
pub fn get_all_irq_stats(buffer: &mut [(u8, [u64; 8])], max_entries: usize) -> usize {
    let handlers = IRQ_HANDLERS.lock();
    let stats = IRQ_STATS.lock();
    
    let mut count = 0;
    for irq in 0..MAX_IRQS {
        if count >= max_entries {
            break;
        }
        
        if handlers[irq].is_some() {
            buffer[count] = (irq as u8, stats[irq]);
            count += 1;
        }
    }
    
    count
}
