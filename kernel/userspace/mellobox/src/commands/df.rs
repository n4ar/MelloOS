//! df - report filesystem disk space usage

use crate::args::Args;
use crate::error::{Error, Result};
use crate::syscalls;
use alloc::{format, string::String, vec::Vec};

const MAX_MOUNTS: usize = 16;

pub fn main(argv: &'static [&'static str]) -> Result<i32> {
    let args = Args::parse(argv, "h")?;
    let human_readable = args.has_option('h');

    let mut mounts = [syscalls::MountInfo::default(); MAX_MOUNTS];
    let count = syscalls::get_mount_info(&mut mounts);
    if count < 0 {
        return Err(Error::from_errno(count));
    }

    let mount_slice = &mounts[..(count as usize)];
    let selected = if args.positional_count() > 0 {
        let target = args.get_positional(0).unwrap();
        if !target.starts_with('/') {
            return Err(Error::InvalidArgument);
        }
        match find_mount_for_path(mount_slice, target) {
            Some(entry) => {
                let mut v = Vec::new();
                v.push(*entry);
                v
            }
            None => {
                let msg = format!("df: '{}' is not mounted\n", target);
                syscalls::write(2, msg.as_bytes());
                return Ok(1);
            }
        }
    } else {
        mount_slice.to_vec()
    };

    print_header(human_readable);
    for entry in &selected {
        print_entry(entry, human_readable);
    }

    Ok(0)
}

fn print_header(human: bool) {
    if human {
        syscalls::write(1, b"Filesystem      Size  Used Avail Use% Mounted on\n");
    } else {
        syscalls::write(
            1,
            b"Filesystem     1K-blocks    Used Available Use% Mounted on\n",
        );
    }
}

fn print_entry(info: &syscalls::MountInfo, human: bool) {
    let fs_name = cstring_to_str(&info.fs_type);
    let mount_point = cstring_to_str(&info.mount_point);
    let block_size = info.block_size.max(1);

    let total_bytes = blocks_to_bytes(info.total_blocks, block_size);
    let free_bytes = blocks_to_bytes(info.free_blocks, block_size);
    let avail_bytes = blocks_to_bytes(info.available_blocks, block_size);
    let used_bytes = total_bytes.saturating_sub(free_bytes);
    let percent = usage_percent(used_bytes, total_bytes);

    let line = if human {
        format!(
            "{:<14} {:>6} {:>6} {:>6} {:>3}% {}\n",
            fs_name,
            format_size(total_bytes),
            format_size(used_bytes),
            format_size(avail_bytes),
            percent,
            mount_point
        )
    } else {
        let total_kib = bytes_to_kib(total_bytes);
        let used_kib = bytes_to_kib(used_bytes);
        let avail_kib = bytes_to_kib(avail_bytes);
        format!(
            "{:<14} {:>10} {:>7} {:>9} {:>3}% {}\n",
            fs_name, total_kib, used_kib, avail_kib, percent, mount_point
        )
    };

    syscalls::write(1, line.as_bytes());
}

fn find_mount_for_path<'a>(
    mounts: &'a [syscalls::MountInfo],
    path: &str,
) -> Option<&'a syscalls::MountInfo> {
    let mut best_idx: Option<usize> = None;
    let mut best_len = 0usize;

    for (idx, entry) in mounts.iter().enumerate() {
        let mount_path = cstring_to_str(&entry.mount_point);
        if mount_path.is_empty() {
            continue;
        }

        if mount_path == "/" {
            if path.starts_with('/') && best_len == 0 {
                best_idx = Some(idx);
                best_len = 1;
            }
            continue;
        }

        if path == mount_path
            || (path.starts_with(mount_path)
                && path.len() > mount_path.len()
                && path.as_bytes().get(mount_path.len()).copied() == Some(b'/'))
        {
            if mount_path.len() > best_len {
                best_idx = Some(idx);
                best_len = mount_path.len();
            }
        }
    }

    if let Some(idx) = best_idx {
        return Some(&mounts[idx]);
    }

    mounts
        .iter()
        .find(|entry| cstring_to_str(&entry.mount_point) == "/")
}

fn cstring_to_str(buf: &[u8]) -> &str {
    let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    core::str::from_utf8(&buf[..len]).unwrap_or("<invalid>")
}

fn blocks_to_bytes(blocks: u64, block_size: u64) -> u128 {
    (blocks as u128) * (block_size as u128)
}

fn bytes_to_kib(bytes: u128) -> u64 {
    ((bytes + 1023) / 1024) as u64
}

fn usage_percent(used: u128, total: u128) -> u64 {
    if total == 0 {
        0
    } else {
        (((used * 100) + (total / 2)) / total) as u64
    }
}

fn format_size(bytes: u128) -> String {
    const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
    let mut value = bytes;
    let mut unit = 0usize;

    while value >= 1024 && unit < UNITS.len() - 1 {
        value = (value + 1023) / 1024;
        unit += 1;
    }

    format!("{}{}", value, UNITS[unit])
}
