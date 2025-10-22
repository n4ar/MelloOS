#![no_std]
#![no_main]
#![allow(static_mut_refs)]

extern crate alloc;

use alloc::format;
use core::panic::PanicInfo;

mod allocator;
mod syscalls;

use syscalls::{exit, write, get_irq_stats, IrqStatsEntry, sleep};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    allocator::init_heap();
    main();
    exit(0);
}

fn main() {
    write_str("IRQ Test - Monitoring Interrupt Distribution\n");
    write_str("==============================================\n\n");
    
    // Query IRQ statistics
    let mut stats = [IrqStatsEntry {
        irq: 0,
        _padding: [0; 7],
        cpu_counts: [0; 8],
    }; 32]; // Support up to 32 IRQs
    
    let count = get_irq_stats(&mut stats);
    
    if count < 0 {
        write_str("Error: Failed to query IRQ statistics\n");
        return;
    }
    
    if count == 0 {
        write_str("No IRQs registered\n");
        return;
    }
    
    write_str(&format!("Found {} registered IRQ(s)\n\n", count));
    
    // Display header
    write_str("IRQ  CPU0     CPU1     CPU2     CPU3     CPU4     CPU5     CPU6     CPU7     TOTAL\n");
    write_str("------------------------------------------------------------------------------------\n");
    
    // Display each IRQ's statistics
    for i in 0..(count as usize) {
        let entry = &stats[i];
        
        // Calculate total interrupts for this IRQ
        let total: u64 = entry.cpu_counts.iter().sum();
        
        // Format IRQ number
        let mut line = format!("{:<4} ", entry.irq);
        
        // Format per-CPU counts
        for cpu in 0..8 {
            line = format!("{}{:<8} ", line, entry.cpu_counts[cpu]);
        }
        
        // Add total
        line = format!("{}{}\n", line, total);
        
        write_str(&line);
    }
    
    write_str("\n");
    
    // Analyze interrupt distribution
    write_str("Interrupt Distribution Analysis:\n");
    write_str("--------------------------------\n");
    
    for i in 0..(count as usize) {
        let entry = &stats[i];
        let total: u64 = entry.cpu_counts.iter().sum();
        
        if total == 0 {
            write_str(&format!("IRQ {}: No interrupts received yet\n", entry.irq));
            continue;
        }
        
        // Find which CPUs handled this IRQ
        let mut active_cpus = 0;
        let mut primary_cpu = 0;
        let mut max_count = 0;
        
        for cpu in 0..8 {
            if entry.cpu_counts[cpu] > 0 {
                active_cpus += 1;
                if entry.cpu_counts[cpu] > max_count {
                    max_count = entry.cpu_counts[cpu];
                    primary_cpu = cpu;
                }
            }
        }
        
        write_str(&format!(
            "IRQ {}: {} total interrupts, {} active CPU(s), primary CPU {}\n",
            entry.irq, total, active_cpus, primary_cpu
        ));
        
        // Check if IOAPIC routing is working correctly
        if active_cpus == 1 {
            write_str(&format!("  ✓ IOAPIC routing working correctly (affinity to CPU {})\n", primary_cpu));
        } else if active_cpus > 1 {
            write_str(&format!("  ⚠ IRQ handled by multiple CPUs (may indicate load balancing)\n"));
        }
    }
    
    write_str("\n");
    
    // Test interrupt generation by triggering keyboard input
    write_str("Testing interrupt generation...\n");
    write_str("Press any key to generate keyboard interrupt (IRQ 1)\n");
    write_str("Or wait 3 seconds to skip...\n\n");
    
    // Sleep for 3 seconds to allow user to press a key
    sleep(3000);
    
    // Query statistics again to see if any new interrupts occurred
    let count2 = get_irq_stats(&mut stats);
    
    if count2 > 0 {
        write_str("Updated statistics after test:\n");
        write_str("IRQ  CPU0     CPU1     CPU2     CPU3     CPU4     CPU5     CPU6     CPU7     TOTAL\n");
        write_str("------------------------------------------------------------------------------------\n");
        
        for i in 0..(count2 as usize) {
            let entry = &stats[i];
            let total: u64 = entry.cpu_counts.iter().sum();
            
            let mut line = format!("{:<4} ", entry.irq);
            for cpu in 0..8 {
                line = format!("{}{:<8} ", line, entry.cpu_counts[cpu]);
            }
            line = format!("{}{}\n", line, total);
            write_str(&line);
        }
    }
    
    write_str("\nIRQ test complete.\n");
}

fn write_str(s: &str) {
    for byte in s.bytes() {
        write(1, &[byte]);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
