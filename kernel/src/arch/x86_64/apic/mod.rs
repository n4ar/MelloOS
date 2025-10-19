/// APIC (Advanced Programmable Interrupt Controller) support
/// This module provides Local APIC management, timer configuration,
/// and Inter-Processor Interrupt (IPI) functionality.

pub mod ipi;

use core::ptr::{read_volatile, write_volatile};

// ============================================================================
// APIC Register Offsets
// ============================================================================

/// Local APIC ID register offset
const LAPIC_ID: u32 = 0x20;

/// End of Interrupt register offset
const LAPIC_EOI: u32 = 0xB0;

/// Spurious Interrupt Vector register offset
const LAPIC_SPURIOUS: u32 = 0xF0;

/// Interrupt Command Register (low 32 bits) offset
const LAPIC_ICR_LOW: u32 = 0x300;

/// Interrupt Command Register (high 32 bits) offset
const LAPIC_ICR_HIGH: u32 = 0x310;

/// Timer Local Vector Table entry offset
const LAPIC_TIMER_LVT: u32 = 0x320;

/// Timer Initial Count register offset
const LAPIC_TIMER_INIT_COUNT: u32 = 0x380;

/// Timer Current Count register offset
const LAPIC_TIMER_CURRENT_COUNT: u32 = 0x390;

/// Timer Divide Configuration register offset
const LAPIC_TIMER_DIVIDE: u32 = 0x3E0;

// ============================================================================
// APIC Constants
// ============================================================================

/// Spurious interrupt vector number
const SPURIOUS_VECTOR: u8 = 0xFF;

/// Timer interrupt vector number
const TIMER_VECTOR: u8 = 0x20;

/// Reschedule IPI vector number
const RESCHEDULE_IPI_VECTOR: u8 = 0x30;

/// APIC enable bit in spurious interrupt vector register
const APIC_ENABLE: u32 = 1 << 8;

/// ICR delivery status bit
const ICR_DELIVERY_STATUS: u32 = 1 << 12;

/// ICR delivery mode: INIT
const ICR_INIT: u32 = 0x500;

/// ICR delivery mode: Startup
const ICR_STARTUP: u32 = 0x600;

/// ICR level assert
const ICR_LEVEL_ASSERT: u32 = 1 << 14;

// ============================================================================
// Local APIC Driver
// ============================================================================

/// Local APIC driver structure
/// 
/// Provides access to the Local APIC through memory-mapped I/O.
/// Each CPU core has its own Local APIC instance.
pub struct LocalApic {
    /// Base address of the APIC memory-mapped registers
    base_addr: *mut u32,
}

impl LocalApic {
    /// Create a new LocalApic instance
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that `base_addr` points to a valid APIC
    /// memory-mapped region and that the address is properly mapped.
    /// 
    /// # Arguments
    /// 
    /// * `base_addr` - Physical address of the APIC registers (typically 0xFEE00000)
    pub unsafe fn new(base_addr: u64) -> Self {
        Self {
            base_addr: base_addr as *mut u32,
        }
    }

    /// Read a 32-bit value from an APIC register
    /// 
    /// # Arguments
    /// 
    /// * `offset` - Register offset in bytes
    #[inline]
    fn read(&self, offset: u32) -> u32 {
        unsafe {
            let reg_addr = (self.base_addr as usize + offset as usize) as *const u32;
            read_volatile(reg_addr)
        }
    }

    /// Write a 32-bit value to an APIC register
    /// 
    /// # Arguments
    /// 
    /// * `offset` - Register offset in bytes
    /// * `value` - Value to write
    #[inline]
    fn write(&mut self, offset: u32, value: u32) {
        unsafe {
            let reg_addr = (self.base_addr as usize + offset as usize) as *mut u32;
            write_volatile(reg_addr, value);
        }
    }

    /// Initialize the Local APIC
    /// 
    /// This function:
    /// 1. Sets the spurious interrupt vector to 0xFF
    /// 2. Enables the APIC by setting bit 8 in the spurious interrupt vector register
    pub fn init(&mut self) {
        // Set spurious interrupt vector and enable APIC
        let spurious_value = (SPURIOUS_VECTOR as u32) | APIC_ENABLE;
        self.write(LAPIC_SPURIOUS, spurious_value);
    }

    /// Get the APIC ID of this Local APIC
    /// 
    /// # Returns
    /// 
    /// The 8-bit APIC ID
    pub fn id(&self) -> u8 {
        // APIC ID is in bits 24-31 of the ID register
        ((self.read(LAPIC_ID) >> 24) & 0xFF) as u8
    }

    /// Send End of Interrupt (EOI) signal
    /// 
    /// This must be called at the end of interrupt handlers to signal
    /// that interrupt processing is complete.
    pub fn eoi(&mut self) {
        self.write(LAPIC_EOI, 0);
    }

    /// Wait for IPI delivery to complete
    /// 
    /// Polls the delivery status bit in the ICR register until it clears,
    /// indicating that the IPI has been sent.
    /// 
    /// # Returns
    /// 
    /// `true` if delivery completed within timeout, `false` otherwise
    fn wait_for_delivery(&self) -> bool {
        // Wait up to ~1ms (approximate)
        for _ in 0..10000 {
            if (self.read(LAPIC_ICR_LOW) & ICR_DELIVERY_STATUS) == 0 {
                return true;
            }
            // Small delay using pause instruction
            unsafe {
                core::arch::asm!("pause");
            }
        }
        false
    }

    /// Send an Inter-Processor Interrupt (IPI) to a specific CPU
    /// 
    /// # Arguments
    /// 
    /// * `apic_id` - Target CPU's APIC ID
    /// * `vector` - Interrupt vector number to send
    /// 
    /// # Returns
    /// 
    /// `true` if IPI was sent successfully, `false` on timeout
    pub fn send_ipi(&mut self, apic_id: u8, vector: u8) -> bool {
        // Wait for any pending IPI to complete
        if !self.wait_for_delivery() {
            return false;
        }

        // Write destination APIC ID to ICR high register (bits 24-31)
        self.write(LAPIC_ICR_HIGH, (apic_id as u32) << 24);

        // Write vector and delivery mode to ICR low register
        // Delivery mode: Fixed (000b), Level: Assert
        self.write(LAPIC_ICR_LOW, vector as u32 | ICR_LEVEL_ASSERT);

        // Wait for delivery to complete
        self.wait_for_delivery()
    }

    /// Send INIT IPI to a specific CPU
    /// 
    /// The INIT IPI is used to initialize an Application Processor (AP)
    /// as part of the SMP boot sequence.
    /// 
    /// # Arguments
    /// 
    /// * `apic_id` - Target CPU's APIC ID
    /// 
    /// # Returns
    /// 
    /// `true` if IPI was sent successfully, `false` on timeout
    pub fn send_init_ipi(&mut self, apic_id: u8) -> bool {
        // Wait for any pending IPI to complete
        if !self.wait_for_delivery() {
            return false;
        }

        // Write destination APIC ID to ICR high register
        self.write(LAPIC_ICR_HIGH, (apic_id as u32) << 24);

        // Send INIT IPI: delivery mode = INIT (101b), level = assert
        self.write(LAPIC_ICR_LOW, ICR_INIT | ICR_LEVEL_ASSERT);

        // Wait for delivery to complete
        self.wait_for_delivery()
    }

    /// Send Startup IPI (SIPI) to a specific CPU
    /// 
    /// The SIPI is used to start an Application Processor (AP) at a specific
    /// memory address. The AP will begin execution at `start_page * 4096`.
    /// 
    /// # Arguments
    /// 
    /// * `apic_id` - Target CPU's APIC ID
    /// * `start_page` - Starting page number (address = start_page * 4096)
    /// 
    /// # Returns
    /// 
    /// `true` if IPI was sent successfully, `false` on timeout
    pub fn send_sipi(&mut self, apic_id: u8, start_page: u8) -> bool {
        // Wait for any pending IPI to complete
        if !self.wait_for_delivery() {
            return false;
        }

        // Write destination APIC ID to ICR high register
        self.write(LAPIC_ICR_HIGH, (apic_id as u32) << 24);

        // Send SIPI: delivery mode = Startup (110b), vector = start page
        self.write(LAPIC_ICR_LOW, ICR_STARTUP | (start_page as u32));

        // Wait for delivery to complete
        self.wait_for_delivery()
    }

    /// Calibrate the APIC timer using the PIT (Programmable Interval Timer)
    /// 
    /// This function uses the PIT as a reference clock to determine the
    /// LAPIC timer frequency. It programs the PIT for a 10ms one-shot,
    /// sets the LAPIC timer to maximum count, waits for the PIT to fire,
    /// and calculates the LAPIC frequency from the remaining count.
    /// 
    /// # Returns
    /// 
    /// The calibrated LAPIC timer frequency in Hz
    /// 
    /// # Safety
    /// 
    /// This function uses I/O ports and should only be called during
    /// initialization with interrupts disabled.
    pub unsafe fn calibrate_timer(&mut self) -> u64 {
        use x86_64::instructions::port::Port;
        
        // PIT constants
        const PIT_FREQUENCY: u32 = 1193182; // PIT base frequency in Hz
        const PIT_COMMAND: u16 = 0x43;
        const PIT_CHANNEL_2: u16 = 0x42;
        const PIT_CHANNEL_2_GATE: u16 = 0x61;
        
        // Calculate PIT divisor for 10ms (100 Hz)
        const CALIBRATION_MS: u32 = 10;
        const PIT_DIVISOR: u32 = PIT_FREQUENCY * CALIBRATION_MS / 1000;
        
        let mut pit_command = Port::<u8>::new(PIT_COMMAND);
        let mut pit_channel2 = Port::<u8>::new(PIT_CHANNEL_2);
        let mut pit_gate = Port::<u8>::new(PIT_CHANNEL_2_GATE);
        
        // Disable PIT channel 2 gate and speaker
        let gate_value = pit_gate.read();
        pit_gate.write(gate_value & 0xFC); // Clear bits 0 and 1
        
        // Configure PIT channel 2 for one-shot mode
        // Command: 10 11 000 0
        // - Channel 2 (10)
        // - Access mode: lobyte/hibyte (11)
        // - Mode 0: interrupt on terminal count (000)
        // - Binary mode (0)
        pit_command.write(0xB0);
        
        // Write divisor to PIT channel 2
        pit_channel2.write((PIT_DIVISOR & 0xFF) as u8);
        pit_channel2.write(((PIT_DIVISOR >> 8) & 0xFF) as u8);
        
        // Set LAPIC timer to maximum count
        self.write(LAPIC_TIMER_INIT_COUNT, 0xFFFFFFFF);
        
        // Enable PIT channel 2 gate to start counting
        let gate_value = pit_gate.read();
        pit_gate.write(gate_value | 0x01); // Set bit 0
        
        // Wait for PIT channel 2 to finish counting
        // Bit 5 of port 0x61 indicates the output state
        loop {
            let status = pit_gate.read();
            if (status & 0x20) != 0 {
                break;
            }
        }
        
        // Read the current LAPIC timer count
        let final_count = self.read(LAPIC_TIMER_CURRENT_COUNT);
        
        // Stop LAPIC timer
        self.write(LAPIC_TIMER_INIT_COUNT, 0);
        
        // Calculate ticks elapsed
        let ticks_elapsed = 0xFFFFFFFF - final_count;
        
        // Calculate frequency: ticks_per_10ms * 100 = ticks_per_second
        let frequency = (ticks_elapsed as u64) * 100;
        
        frequency
    }

    /// Initialize the APIC timer in periodic mode
    /// 
    /// This function configures the LAPIC timer to generate periodic interrupts
    /// at the specified frequency. It sets the timer divide value to 16 and
    /// calculates the initial count based on the calibrated frequency.
    /// 
    /// # Arguments
    /// 
    /// * `frequency_hz` - Calibrated LAPIC timer frequency in Hz
    /// * `target_hz` - Target interrupt frequency (e.g., 100 Hz for SCHED_HZ)
    /// 
    /// # Safety
    /// 
    /// This function should only be called after calibrating the timer
    /// and with interrupts disabled during initialization.
    pub unsafe fn init_timer(&mut self, frequency_hz: u64, target_hz: u64) {
        // Set timer divide value to 16
        // Divide configuration register: bits 0-1 and bit 3
        // For divide by 16: 0b0011 = 3
        self.write(LAPIC_TIMER_DIVIDE, 0x3);
        
        // Calculate initial count for desired frequency
        // initial_count = lapic_hz / (divide_value * target_hz)
        let initial_count = frequency_hz / (16 * target_hz);
        
        // Set timer vector and mode
        // Bit 17: Timer mode (1 = periodic, 0 = one-shot)
        // Bits 0-7: Vector number
        let timer_config = (1 << 17) | (TIMER_VECTOR as u32);
        self.write(LAPIC_TIMER_LVT, timer_config);
        
        // Set initial count to start the timer
        self.write(LAPIC_TIMER_INIT_COUNT, initial_count as u32);
    }
}
