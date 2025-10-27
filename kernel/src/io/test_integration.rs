//! Integration test for I/O infrastructure
//!
//! This module demonstrates that the I/O infrastructure is properly integrated
//! and can be used by other kernel modules.

#[allow(dead_code)]
pub fn test_io_infrastructure() {
    // This function demonstrates that all I/O functions are accessible
    // In a real scenario, these would be called with actual hardware addresses

    // Port I/O functions are available
    let _port_ops = (
        crate::io::inb,
        crate::io::outb,
        crate::io::inw,
        crate::io::outw,
        crate::io::inl,
        crate::io::outl,
    );

    // MMIO functions are available
    let _mmio_ops = (
        crate::io::mmio_read::<u32>,
        crate::io::mmio_write::<u32>,
        crate::io::mmio_read32,
        crate::io::mmio_write32,
    );

    // IRQ management functions are available
    let _irq_ops = (
        crate::io::init_ioapic_routing,
        crate::io::register_irq_handler,
        crate::io::register_irq_handler_affinity,
        crate::io::unregister_irq_handler,
        crate::io::handle_irq,
        crate::io::is_irq_registered,
        crate::io::registered_irq_count,
    );

    crate::serial_println!("[IO] I/O infrastructure integration test passed");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_io_functions_exist() {
        // This test verifies that all I/O functions are accessible
        // The actual functionality is tested in the individual module tests
        super::test_io_infrastructure();
    }
}
