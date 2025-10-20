/// Inter-Processor Interrupt (IPI) utilities
/// 
/// This module provides high-level IPI functions for sending interrupts
/// between CPU cores. IPIs are used for:
/// - Rescheduling tasks on remote cores (RESCHEDULE_IPI)
/// - TLB shootdown (future)
/// - Halting cores (future)

use super::LocalApic;
use crate::arch::x86_64::smp::{get_cpu_count, is_cpu_online};
use crate::arch::x86_64::smp::percpu::percpu_for;
use crate::arch::x86_64::acpi::get_madt_info;
use crate::serial_println;

/// RESCHEDULE_IPI vector number
/// This IPI triggers the scheduler on the target core
pub const RESCHEDULE_IPI_VECTOR: u8 = 0x30;

/// TLB_FLUSH_IPI vector number (future use)
#[allow(dead_code)]
pub const TLB_FLUSH_IPI_VECTOR: u8 = 0x31;

/// HALT_IPI vector number (future use)
#[allow(dead_code)]
pub const HALT_IPI_VECTOR: u8 = 0x32;

/// Send an Inter-Processor Interrupt to a specific CPU
/// 
/// This function sends an IPI with the specified vector to the target CPU.
/// It uses the Local APIC's send_ipi() method to deliver the interrupt.
/// 
/// # Arguments
/// * `target_apic_id` - APIC ID of the target CPU
/// * `vector` - Interrupt vector number to send
/// 
/// # Returns
/// `true` if the IPI was sent successfully, `false` on timeout
/// 
/// # Example
/// ```
/// // Send RESCHEDULE_IPI to CPU with APIC ID 1
/// send_ipi(1, RESCHEDULE_IPI_VECTOR);
/// ```
pub fn send_ipi(target_apic_id: u8, vector: u8) -> bool {
    // Get MADT info to access LAPIC base address
    let madt_info = match get_madt_info() {
        Some(info) => info,
        None => {
            serial_println!("[IPI] ERROR: Failed to get MADT info");
            return false;
        }
    };
    
    // Create a temporary LocalApic instance for sending IPI
    let mut lapic = unsafe { LocalApic::new(madt_info.lapic_address) };
    
    // Send the IPI
    lapic.send_ipi(target_apic_id, vector)
}

/// Broadcast an Inter-Processor Interrupt to all CPUs
/// 
/// This function sends an IPI with the specified vector to all online CPUs.
/// It can optionally exclude the current CPU from the broadcast.
/// 
/// # Arguments
/// * `vector` - Interrupt vector number to send
/// * `exclude_self` - If true, don't send IPI to the current CPU
/// 
/// # Returns
/// The number of CPUs that successfully received the IPI
/// 
/// # Example
/// ```
/// // Send RESCHEDULE_IPI to all other CPUs
/// broadcast_ipi(RESCHEDULE_IPI_VECTOR, true);
/// ```
pub fn broadcast_ipi(vector: u8, exclude_self: bool) -> usize {
    let cpu_count = get_cpu_count();
    let mut success_count = 0;
    
    // Get current CPU's APIC ID if we need to exclude self
    let current_apic_id = if exclude_self {
        let percpu = crate::arch::x86_64::smp::percpu::percpu_current();
        Some(percpu.apic_id)
    } else {
        None
    };
    
    // Send IPI to each online CPU
    for cpu_id in 0..cpu_count {
        // Skip if this CPU is not online
        if !is_cpu_online(cpu_id) {
            continue;
        }
        
        // Get APIC ID for this CPU
        let target_apic_id = percpu_for(cpu_id).apic_id;
        
        // Skip if this is the current CPU and exclude_self is true
        if let Some(current_id) = current_apic_id {
            if target_apic_id == current_id {
                continue;
            }
        }
        
        // Send IPI to this CPU
        if send_ipi(target_apic_id, vector) {
            success_count += 1;
        } else {
            serial_println!("[IPI] WARNING: Failed to send IPI to CPU {} (APIC ID {})",
                          cpu_id, target_apic_id);
        }
    }
    
    success_count
}

/// Send a RESCHEDULE_IPI to a specific CPU
/// 
/// This is a convenience wrapper around send_ipi() that sends a RESCHEDULE_IPI
/// to the specified CPU. The RESCHEDULE_IPI triggers the scheduler on the target
/// core, causing it to preempt the current task and select a new one.
/// 
/// This is used when:
/// - A new task is enqueued to a remote CPU
/// - A task is migrated to a different CPU during load balancing
/// - A task on a remote CPU needs to be woken up
/// 
/// # Arguments
/// * `cpu_id` - Logical CPU ID of the target CPU
/// 
/// # Returns
/// `true` if the IPI was sent successfully, `false` on error
/// 
/// # Example
/// ```
/// // Wake up CPU 1 to schedule a newly enqueued task
/// send_reschedule_ipi(1);
/// ```
pub fn send_reschedule_ipi(cpu_id: usize) -> bool {
    // Validate CPU ID
    if cpu_id >= get_cpu_count() {
        serial_println!("[IPI] ERROR: Invalid CPU ID {} (max: {})",
                      cpu_id, get_cpu_count() - 1);
        return false;
    }
    
    // Check if target CPU is online
    if !is_cpu_online(cpu_id) {
        serial_println!("[IPI] ERROR: CPU {} is not online", cpu_id);
        return false;
    }
    
    // Get target CPU's APIC ID
    let target_apic_id = percpu_for(cpu_id).apic_id;
    
    // Log the IPI send (as required by the spec)
    serial_println!("[SCHED] send RESCHED IPI → core{}", cpu_id);
    
    // Send the IPI
    send_ipi(target_apic_id, RESCHEDULE_IPI_VECTOR)
}
