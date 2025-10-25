#![no_std]
#![no_main]

extern crate alloc;

mod allocator;
mod syscalls;

use core::panic::PanicInfo;

/// MelloFS magic number: "MFSD"
const MFS_MAGIC: u32 = 0x4D465344;
const MFS_VERSION: u32 = 1;
const DEFAULT_BLOCK_SIZE: u32 = 4096;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Get command line arguments
    let args = syscalls::get_args();
    
    if args.len() < 2 {
        print_usage();
        syscalls::exit(1);
    }
    
    let device_path = args[1];
    
    // Parse options
    let mut block_size = DEFAULT_BLOCK_SIZE;
    let mut label = "MelloFS";
    
    let mut i = 2;
    while i < args.len() {
        match args[i] {
            "-b" | "--block-size" => {
                if i + 1 < args.len() {
                    block_size = parse_size(args[i + 1]).unwrap_or(DEFAULT_BLOCK_SIZE);
                    i += 2;
                } else {
                    println("Error: --block-size requires an argument");
                    syscalls::exit(1);
                }
            }
            "-l" | "--label" => {
                if i + 1 < args.len() {
                    label = args[i + 1];
                    i += 2;
                } else {
                    println("Error: --label requires an argument");
                    syscalls::exit(1);
                }
            }
            _ => {
                println("Error: Unknown option");
                print_usage();
                syscalls::exit(1);
            }
        }
    }
    
    // Validate block size
    if !is_valid_block_size(block_size) {
        println("Error: Invalid block size. Must be 4096, 8192, or 16384");
        syscalls::exit(1);
    }
    
    println("Creating MelloFS filesystem:");
    print("  Device: "); println(device_path);
    print("  Block size: "); print_num(block_size as usize); println(" bytes");
    print("  Label: "); println(label);
    
    // Format the device
    match format_device(device_path, block_size, label) {
        Ok(()) => {
            println("Filesystem created successfully!");
            syscalls::exit(0);
        }
        Err(e) => {
            print("Error: "); println(e);
            syscalls::exit(1);
        }
    }
}

fn format_device(device_path: &str, block_size: u32, label: &str) -> Result<(), &'static str> {
    // Open device
    let fd = syscalls::open(device_path, syscalls::O_RDWR)
        .map_err(|_| "Failed to open device")?;
    
    // Get device size
    let device_size = syscalls::lseek(fd, 0, syscalls::SEEK_END)
        .map_err(|_| "Failed to get device size")?;
    syscalls::lseek(fd, 0, syscalls::SEEK_SET)
        .map_err(|_| "Failed to seek to start")?;
    
    let total_blocks = device_size / (block_size as i64);
    
    if total_blocks < 64 {
        syscalls::close(fd);
        return Err("Device too small (minimum 64 blocks)");
    }
    
    println("Formatting device...");
    print("  Total blocks: "); print_num(total_blocks as usize); println("");
    
    // Create superblock
    let mut superblock = [0u8; 256];
    
    // Write magic and version
    write_u32(&mut superblock[0..4], MFS_MAGIC);
    write_u32(&mut superblock[4..8], MFS_VERSION);
    
    // Write UUID (zeros for now)
    // superblock[8..24] = UUID
    
    // Write TxG ID (0 for new filesystem)
    write_u64(&mut superblock[24..32], 0);
    
    // Write root B-tree pointer (placeholder)
    write_u64(&mut superblock[32..40], 0); // lba
    write_u32(&mut superblock[40..44], 0); // length
    write_u64(&mut superblock[44..52], 0); // checksum
    superblock[52] = 0; // level
    
    // Write allocator B-tree pointer (placeholder)
    write_u64(&mut superblock[64..72], 0); // lba
    write_u32(&mut superblock[72..76], 0); // length
    write_u64(&mut superblock[76..84], 0); // checksum
    superblock[84] = 0; // level
    
    // Write features (0 for now)
    write_u64(&mut superblock[96..104], 0);
    
    // Write block size
    write_u32(&mut superblock[104..108], block_size);
    
    // Write total and free blocks
    write_u64(&mut superblock[112..120], total_blocks as u64);
    write_u64(&mut superblock[120..128], (total_blocks - 64) as u64); // Reserve first 64 blocks
    
    // Write timestamps (0 for now)
    write_u64(&mut superblock[128..136], 0); // created_time
    write_u64(&mut superblock[136..144], 0); // modified_time
    write_u64(&mut superblock[144..152], 0); // mounted_time
    
    // Write mount count and state
    write_u32(&mut superblock[152..156], 0); // mount_count
    write_u32(&mut superblock[156..160], 0); // state (clean)
    
    // Write label
    let label_bytes = label.as_bytes();
    let copy_len = core::cmp::min(label_bytes.len(), 64);
    superblock[160..160+copy_len].copy_from_slice(&label_bytes[..copy_len]);
    
    // Compute checksum (simplified - just XOR for now)
    let checksum = compute_checksum(&superblock[0..248]);
    write_u64(&mut superblock[248..256], checksum);
    
    // Write superblock to device (sector 0)
    syscalls::write(fd, &superblock)
        .map_err(|_| "Failed to write superblock")?;
    
    // Write secondary superblock (last sectors)
    let secondary_offset = (total_blocks - 1) * (block_size as i64);
    syscalls::lseek(fd, secondary_offset, syscalls::SEEK_SET)
        .map_err(|_| "Failed to seek to secondary superblock")?;
    syscalls::write(fd, &superblock)
        .map_err(|_| "Failed to write secondary superblock")?;
    
    // Sync and close
    syscalls::fsync(fd)
        .map_err(|_| "Failed to sync device")?;
    syscalls::close(fd);
    
    Ok(())
}

fn is_valid_block_size(size: u32) -> bool {
    matches!(size, 4096 | 8192 | 16384)
}

fn parse_size(s: &str) -> Option<u32> {
    let mut num = 0u32;
    for c in s.chars() {
        if c >= '0' && c <= '9' {
            num = num * 10 + (c as u32 - '0' as u32);
        } else {
            return None;
        }
    }
    Some(num)
}

fn compute_checksum(data: &[u8]) -> u64 {
    let mut checksum = 0u64;
    for &byte in data {
        checksum ^= byte as u64;
        checksum = checksum.rotate_left(1);
    }
    checksum
}

fn write_u32(buf: &mut [u8], val: u32) {
    buf[0] = (val & 0xFF) as u8;
    buf[1] = ((val >> 8) & 0xFF) as u8;
    buf[2] = ((val >> 16) & 0xFF) as u8;
    buf[3] = ((val >> 24) & 0xFF) as u8;
}

fn write_u64(buf: &mut [u8], val: u64) {
    buf[0] = (val & 0xFF) as u8;
    buf[1] = ((val >> 8) & 0xFF) as u8;
    buf[2] = ((val >> 16) & 0xFF) as u8;
    buf[3] = ((val >> 24) & 0xFF) as u8;
    buf[4] = ((val >> 32) & 0xFF) as u8;
    buf[5] = ((val >> 40) & 0xFF) as u8;
    buf[6] = ((val >> 48) & 0xFF) as u8;
    buf[7] = ((val >> 56) & 0xFF) as u8;
}

fn print_usage() {
    println("Usage: mkfs.mfs [OPTIONS] <device>");
    println("");
    println("Options:");
    println("  -b, --block-size SIZE   Block size (4096, 8192, or 16384)");
    println("  -l, --label LABEL       Filesystem label");
}

fn println(s: &str) {
    syscalls::write(1, s.as_bytes()).ok();
    syscalls::write(1, b"\n").ok();
}

fn print(s: &str) {
    syscalls::write(1, s.as_bytes()).ok();
}

fn print_num(n: usize) {
    let mut buf = [0u8; 20];
    let mut i = 0;
    let mut num = n;
    
    if num == 0 {
        buf[0] = b'0';
        i = 1;
    } else {
        while num > 0 {
            buf[i] = (b'0' + (num % 10) as u8);
            num /= 10;
            i += 1;
        }
        // Reverse
        buf[0..i].reverse();
    }
    
    syscalls::write(1, &buf[0..i]).ok();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    syscalls::exit(1);
}
