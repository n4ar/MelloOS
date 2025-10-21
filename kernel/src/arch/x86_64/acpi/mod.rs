use crate::config::MAX_CPUS;
/// ACPI (Advanced Configuration and Power Interface) support
/// This module provides ACPI table parsing, specifically the MADT
/// (Multiple APIC Description Table) for CPU and APIC discovery.
use crate::{serial_print, serial_println};
use core::slice;
use core::sync::atomic::{AtomicBool, Ordering};

/// Global MADT information
static mut MADT_INFO: Option<MadtInfo> = None;
static MADT_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// RSDP (Root System Description Pointer) structure
/// This is the first ACPI structure we need to find
#[repr(C, packed)]
struct Rsdp {
    signature: [u8; 8], // "RSD PTR "
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_address: u32,
}

/// Extended RSDP for ACPI 2.0+
#[repr(C, packed)]
struct RsdpExtended {
    rsdp: Rsdp,
    length: u32,
    xsdt_address: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

/// ACPI System Description Table Header
/// Common header for all ACPI tables
#[repr(C, packed)]
struct SdtHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

/// MADT (Multiple APIC Description Table) structure
#[repr(C, packed)]
struct Madt {
    header: SdtHeader,
    lapic_address: u32,
    flags: u32,
    // Followed by variable-length entries
}

/// MADT Entry Header
#[repr(C, packed)]
struct MadtEntryHeader {
    entry_type: u8,
    length: u8,
}

/// MADT Entry Type 0: Processor Local APIC
#[repr(C, packed)]
struct MadtLocalApic {
    header: MadtEntryHeader,
    processor_id: u8,
    apic_id: u8,
    flags: u32,
}

/// MADT Entry Type 1: I/O APIC
#[repr(C, packed)]
struct MadtIoApic {
    header: MadtEntryHeader,
    ioapic_id: u8,
    reserved: u8,
    ioapic_address: u32,
    gsi_base: u32,
}

/// CPU information extracted from MADT
#[derive(Debug, Clone, Copy)]
pub struct CpuInfo {
    pub apic_id: u8,
    pub processor_id: u8,
    pub enabled: bool,
}

/// I/O APIC information extracted from MADT
#[derive(Debug, Clone, Copy)]
pub struct IoApicInfo {
    pub id: u8,
    pub address: u32,
    pub gsi_base: u32,
}

/// Parsed MADT information
pub struct MadtInfo {
    pub lapic_address: u64,
    pub cpus: [Option<CpuInfo>; MAX_CPUS],
    pub cpu_count: usize,
    pub ioapics: [Option<IoApicInfo>; 8], // Support up to 8 I/O APICs
    pub ioapic_count: usize,
}

/// ACPI parsing errors
#[derive(Debug)]
pub enum AcpiError {
    InvalidRsdp,
    InvalidChecksum,
    MadtNotFound,
    InvalidMadt,
}

/// Validate ACPI table checksum
/// Returns true if checksum is valid
fn validate_checksum(data: &[u8]) -> bool {
    let sum: u8 = data.iter().fold(0u8, |acc, &byte| acc.wrapping_add(byte));
    sum == 0
}

/// Initialize ACPI and parse MADT
/// This should be called once during boot after memory initialization
///
/// # Arguments
/// * `rsdp_addr` - Physical address of the RSDP structure
///
/// # Returns
/// * `Ok(())` - ACPI initialized successfully
/// * `Err(AcpiError)` - Error if parsing fails
pub fn init_acpi(rsdp_addr: u64) -> Result<(), AcpiError> {
    let madt_info = parse_madt(rsdp_addr)?;

    // Log detected CPUs
    let mut apic_ids = [0u8; MAX_CPUS];
    let mut enabled_count = 0;

    for i in 0..madt_info.cpu_count {
        if let Some(cpu) = madt_info.cpus[i] {
            apic_ids[i] = cpu.apic_id;
            if cpu.enabled {
                enabled_count += 1;
            }
        }
    }

    serial_print!("[SMP] CPUs detected: {} (apic_ids=[", madt_info.cpu_count);
    for i in 0..madt_info.cpu_count {
        if i > 0 {
            serial_print!(",");
        }
        serial_print!("{}", apic_ids[i]);
    }
    serial_println!("])");
    serial_println!("[SMP] Enabled CPUs: {}", enabled_count);

    // Store MADT info globally
    unsafe {
        MADT_INFO = Some(madt_info);
    }
    MADT_INITIALIZED.store(true, Ordering::Release);

    Ok(())
}

/// Get reference to MADT information
/// Returns None if ACPI has not been initialized
pub fn get_madt_info() -> Option<&'static MadtInfo> {
    if MADT_INITIALIZED.load(Ordering::Acquire) {
        unsafe { MADT_INFO.as_ref() }
    } else {
        None
    }
}

/// Parse MADT table and extract CPU and APIC information
///
/// # Arguments
/// * `rsdp_addr` - Physical address of the RSDP structure
///
/// # Returns
/// * `Ok(MadtInfo)` - Parsed MADT information with CPU list and APIC addresses
/// * `Err(AcpiError)` - Error if parsing fails
fn parse_madt(rsdp_addr: u64) -> Result<MadtInfo, AcpiError> {
    serial_println!("[ACPI] RSDP found at 0x{:x}", rsdp_addr);

    // Read RSDP structure
    let rsdp = unsafe { &*(rsdp_addr as *const Rsdp) };

    // Validate RSDP signature
    if &rsdp.signature != b"RSD PTR " {
        serial_println!("[ACPI] Invalid RSDP signature");
        return Err(AcpiError::InvalidRsdp);
    }

    // Validate RSDP checksum
    let rsdp_bytes = unsafe { slice::from_raw_parts(rsdp_addr as *const u8, 20) };
    if !validate_checksum(rsdp_bytes) {
        serial_println!("[ACPI] Invalid RSDP checksum");
        return Err(AcpiError::InvalidChecksum);
    }

    serial_println!("[ACPI] RSDP validated, revision: {}", rsdp.revision);

    // Determine which table to use (RSDT or XSDT)
    let madt_addr = if rsdp.revision >= 2 {
        // ACPI 2.0+: Use XSDT
        let rsdp_ext = unsafe { &*(rsdp_addr as *const RsdpExtended) };
        find_madt_in_xsdt(rsdp_ext.xsdt_address)?
    } else {
        // ACPI 1.0: Use RSDT
        find_madt_in_rsdt(rsdp.rsdt_address as u64)?
    };

    serial_println!("[ACPI] MADT found at 0x{:x}", madt_addr);

    // Parse MADT
    parse_madt_table(madt_addr)
}

/// Find MADT in RSDT (ACPI 1.0)
fn find_madt_in_rsdt(rsdt_addr: u64) -> Result<u64, AcpiError> {
    let header = unsafe { &*(rsdt_addr as *const SdtHeader) };

    // Validate RSDT signature
    if &header.signature != b"RSDT" {
        serial_println!("[ACPI] Invalid RSDT signature");
        return Err(AcpiError::InvalidRsdp);
    }

    // Validate checksum
    let rsdt_bytes =
        unsafe { slice::from_raw_parts(rsdt_addr as *const u8, header.length as usize) };
    if !validate_checksum(rsdt_bytes) {
        serial_println!("[ACPI] Invalid RSDT checksum");
        return Err(AcpiError::InvalidChecksum);
    }

    // Calculate number of entries
    let entries_offset = core::mem::size_of::<SdtHeader>();
    let entries_size = header.length as usize - entries_offset;
    let entry_count = entries_size / 4; // 32-bit pointers

    // Get pointer to entries array
    let entries_ptr = unsafe { (rsdt_addr as *const u8).add(entries_offset) as *const u32 };
    let entries = unsafe { slice::from_raw_parts(entries_ptr, entry_count) };

    // Search for MADT
    for &entry_addr in entries {
        let entry_header = unsafe { &*(entry_addr as u64 as *const SdtHeader) };
        if &entry_header.signature == b"APIC" {
            return Ok(entry_addr as u64);
        }
    }

    serial_println!("[ACPI] MADT not found in RSDT");
    Err(AcpiError::MadtNotFound)
}

/// Find MADT in XSDT (ACPI 2.0+)
fn find_madt_in_xsdt(xsdt_addr: u64) -> Result<u64, AcpiError> {
    let header = unsafe { &*(xsdt_addr as *const SdtHeader) };

    // Validate XSDT signature
    if &header.signature != b"XSDT" {
        serial_println!("[ACPI] Invalid XSDT signature");
        return Err(AcpiError::InvalidRsdp);
    }

    // Validate checksum
    let xsdt_bytes =
        unsafe { slice::from_raw_parts(xsdt_addr as *const u8, header.length as usize) };
    if !validate_checksum(xsdt_bytes) {
        serial_println!("[ACPI] Invalid XSDT checksum");
        return Err(AcpiError::InvalidChecksum);
    }

    // Calculate number of entries
    let entries_offset = core::mem::size_of::<SdtHeader>();
    let entries_size = header.length as usize - entries_offset;
    let entry_count = entries_size / 8; // 64-bit pointers

    // Get pointer to entries array
    let entries_ptr = unsafe { (xsdt_addr as *const u8).add(entries_offset) as *const u64 };
    let entries = unsafe { slice::from_raw_parts(entries_ptr, entry_count) };

    // Search for MADT
    for &entry_addr in entries {
        let entry_header = unsafe { &*(entry_addr as *const SdtHeader) };
        if &entry_header.signature == b"APIC" {
            return Ok(entry_addr);
        }
    }

    serial_println!("[ACPI] MADT not found in XSDT");
    Err(AcpiError::MadtNotFound)
}

/// Parse MADT table and extract CPU and APIC information
fn parse_madt_table(madt_addr: u64) -> Result<MadtInfo, AcpiError> {
    let madt = unsafe { &*(madt_addr as *const Madt) };

    // Validate MADT signature
    if &madt.header.signature != b"APIC" {
        serial_println!("[ACPI] Invalid MADT signature");
        return Err(AcpiError::InvalidMadt);
    }

    // Validate checksum
    let madt_bytes =
        unsafe { slice::from_raw_parts(madt_addr as *const u8, madt.header.length as usize) };
    if !validate_checksum(madt_bytes) {
        serial_println!("[ACPI] Invalid MADT checksum");
        return Err(AcpiError::InvalidChecksum);
    }

    let lapic_address = madt.lapic_address as u64;
    serial_println!("[ACPI] Local APIC address: 0x{:x}", lapic_address);

    let mut cpus: [Option<CpuInfo>; MAX_CPUS] = [None; MAX_CPUS];
    let mut cpu_count = 0;
    let mut ioapics: [Option<IoApicInfo>; 8] = [None; 8];
    let mut ioapic_count = 0;

    // Parse MADT entries
    let entries_offset = core::mem::size_of::<Madt>();
    let entries_size = madt.header.length as usize - entries_offset;
    let entries_start = unsafe { (madt_addr as *const u8).add(entries_offset) };

    let mut offset = 0;
    while offset < entries_size {
        let entry_ptr = unsafe { entries_start.add(offset) };
        let entry_header = unsafe { &*(entry_ptr as *const MadtEntryHeader) };

        match entry_header.entry_type {
            0 => {
                // Processor Local APIC
                let local_apic_ptr = entry_ptr as *const MadtLocalApic;

                // Read values using raw pointers to avoid alignment issues
                let processor_id =
                    unsafe { core::ptr::addr_of!((*local_apic_ptr).processor_id).read() };
                let apic_id = unsafe { core::ptr::addr_of!((*local_apic_ptr).apic_id).read() };
                let flags =
                    unsafe { core::ptr::addr_of!((*local_apic_ptr).flags).read_unaligned() };
                let enabled = (flags & 0x1) != 0;

                if cpu_count < MAX_CPUS {
                    cpus[cpu_count] = Some(CpuInfo {
                        apic_id,
                        processor_id,
                        enabled,
                    });
                    cpu_count += 1;

                    serial_println!(
                        "[ACPI] CPU: processor_id={}, apic_id={}, enabled={}",
                        processor_id,
                        apic_id,
                        enabled
                    );
                } else {
                    serial_println!(
                        "[ACPI] Warning: MAX_CPUS limit reached, ignoring additional CPUs"
                    );
                }
            }
            1 => {
                // I/O APIC
                let ioapic_ptr = entry_ptr as *const MadtIoApic;

                // Read values using raw pointers to avoid alignment issues
                let ioapic_id = unsafe { core::ptr::addr_of!((*ioapic_ptr).ioapic_id).read() };
                let ioapic_address =
                    unsafe { core::ptr::addr_of!((*ioapic_ptr).ioapic_address).read_unaligned() };
                let gsi_base =
                    unsafe { core::ptr::addr_of!((*ioapic_ptr).gsi_base).read_unaligned() };

                if ioapic_count < 8 {
                    ioapics[ioapic_count] = Some(IoApicInfo {
                        id: ioapic_id,
                        address: ioapic_address,
                        gsi_base,
                    });
                    ioapic_count += 1;

                    serial_println!(
                        "[ACPI] I/O APIC: id={}, address=0x{:x}, gsi_base={}",
                        ioapic_id,
                        ioapic_address,
                        gsi_base
                    );
                } else {
                    serial_println!(
                        "[ACPI] Warning: I/O APIC limit reached, ignoring additional I/O APICs"
                    );
                }
            }
            _ => {
                // Other entry types (ignored for now)
                serial_println!(
                    "[ACPI] Skipping MADT entry type {}",
                    entry_header.entry_type
                );
            }
        }

        offset += entry_header.length as usize;
    }

    Ok(MadtInfo {
        lapic_address,
        cpus,
        cpu_count,
        ioapics,
        ioapic_count,
    })
}
