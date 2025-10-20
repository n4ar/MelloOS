/// x86_64 architecture-specific modules

pub mod acpi;
pub mod apic;
pub mod fault;
pub mod gdt;
pub mod smp;
pub mod syscall;

// Re-export user_entry_trampoline for external use
pub use gdt::user_entry_trampoline;
