// Block device driver module

pub mod virtio_blk;

pub use virtio_blk::{block_read, block_write, BlockDevice, BlockError};
