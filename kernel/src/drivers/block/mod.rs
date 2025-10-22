// Block device driver module

pub mod virtio_blk;

pub use virtio_blk::{BlockDevice, BlockError, block_read, block_write};
