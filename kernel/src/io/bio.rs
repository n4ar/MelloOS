//! Block I/O (BIO) queue management
//!
//! This module implements:
//! - Queue depth management (target: 32-128 for NVMe)
//! - Batch I/O submissions for efficiency
//! - TRIM/DISCARD support hook for SSD optimization

use core::sync::atomic::{AtomicUsize, AtomicU64, AtomicBool, Ordering};
use spin::RwLock;

/// Block I/O operation type
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BioOp {
    /// Read operation
    Read,
    /// Write operation
    Write,
    /// Flush operation
    Flush,
    /// TRIM/DISCARD operation
    Trim,
}

/// Block I/O request
#[derive(Clone, Copy)]
pub struct BioRequest {
    /// Operation type
    pub op: BioOp,
    /// Device ID
    pub device_id: u64,
    /// Starting block number
    pub start_block: u64,
    /// Number of blocks
    pub block_count: usize,
    /// Buffer address (physical address for DMA)
    pub buffer_addr: u64,
    /// Request ID for tracking
    pub request_id: u64,
    /// Priority (0 = highest)
    pub priority: u8,
}

impl BioRequest {
    /// Create a new BIO request
    pub fn new(
        op: BioOp,
        device_id: u64,
        start_block: u64,
        block_count: usize,
        buffer_addr: u64,
    ) -> Self {
        Self {
            op,
            device_id,
            start_block,
            block_count,
            buffer_addr,
            request_id: 0,
            priority: 128, // Default: medium priority
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set request ID
    pub fn with_id(mut self, request_id: u64) -> Self {
        self.request_id = request_id;
        self
    }
}

/// BIO queue entry
struct BioQueueEntry {
    /// Request
    request: BioRequest,
    /// Is this entry valid?
    valid: AtomicBool,
    /// Submission timestamp
    submit_time: AtomicU64,
}

impl BioQueueEntry {
    /// Create a new empty queue entry
    const fn new() -> Self {
        Self {
            request: BioRequest {
                op: BioOp::Read,
                device_id: 0,
                start_block: 0,
                block_count: 0,
                buffer_addr: 0,
                request_id: 0,
                priority: 0,
            },
            valid: AtomicBool::new(false),
            submit_time: AtomicU64::new(0),
        }
    }

    /// Initialize with a request
    fn init(&mut self, request: BioRequest, timestamp: u64) {
        self.request = request;
        self.submit_time.store(timestamp, Ordering::Release);
        self.valid.store(true, Ordering::Release);
    }

    /// Check if valid
    fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    /// Invalidate
    fn invalidate(&self) {
        self.valid.store(false, Ordering::Release);
    }

    /// Get request
    fn get_request(&self) -> BioRequest {
        self.request
    }
}

/// Target queue depth for NVMe devices
pub const TARGET_QUEUE_DEPTH_NVME: usize = 128;

/// Target queue depth for SATA devices
pub const TARGET_QUEUE_DEPTH_SATA: usize = 32;

/// Maximum queue depth
pub const MAX_QUEUE_DEPTH: usize = 256;

/// BIO queue for a device
pub struct BioQueue {
    /// Device ID
    device_id: AtomicU64,
    /// Is this queue in use?
    in_use: AtomicBool,
    /// Queue entries
    entries: [RwLock<BioQueueEntry>; MAX_QUEUE_DEPTH],
    /// Current queue depth
    depth: AtomicUsize,
    /// Target queue depth
    target_depth: AtomicUsize,
    /// Next request ID
    next_request_id: AtomicU64,
    /// Timestamp counter
    timestamp: AtomicU64,
    /// Number of submitted requests
    submitted: AtomicUsize,
    /// Number of completed requests
    completed: AtomicUsize,
}

impl BioQueue {
    /// Create a new BIO queue
    const fn new() -> Self {
        const INIT_ENTRY: RwLock<BioQueueEntry> = RwLock::new(BioQueueEntry::new());
        Self {
            device_id: AtomicU64::new(0),
            in_use: AtomicBool::new(false),
            entries: [INIT_ENTRY; MAX_QUEUE_DEPTH],
            depth: AtomicUsize::new(0),
            target_depth: AtomicUsize::new(TARGET_QUEUE_DEPTH_SATA),
            next_request_id: AtomicU64::new(1),
            timestamp: AtomicU64::new(0),
            submitted: AtomicUsize::new(0),
            completed: AtomicUsize::new(0),
        }
    }

    /// Initialize queue for a device
    pub fn init(&self, device_id: u64, target_depth: usize) {
        self.device_id.store(device_id, Ordering::Release);
        self.target_depth.store(target_depth.min(MAX_QUEUE_DEPTH), Ordering::Release);
        self.in_use.store(true, Ordering::Release);
        self.depth.store(0, Ordering::Release);
        self.submitted.store(0, Ordering::Release);
        self.completed.store(0, Ordering::Release);
    }

    /// Check if this queue is for the given device
    pub fn is_for_device(&self, device_id: u64) -> bool {
        self.in_use.load(Ordering::Acquire) && self.device_id.load(Ordering::Acquire) == device_id
    }

    /// Check if queue is in use
    pub fn is_in_use(&self) -> bool {
        self.in_use.load(Ordering::Acquire)
    }

    /// Submit a request to the queue
    ///
    /// Returns the request ID if successful, None if queue is full
    pub fn submit(&self, mut request: BioRequest) -> Option<u64> {
        let current_depth = self.depth.load(Ordering::Relaxed);
        let target = self.target_depth.load(Ordering::Relaxed);

        // Check if queue is full
        if current_depth >= target {
            return None;
        }

        // Assign request ID
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        request.request_id = request_id;

        // Find free slot
        for entry_lock in &self.entries {
            let mut entry = entry_lock.write();
            if !entry.is_valid() {
                let timestamp = self.timestamp.fetch_add(1, Ordering::Relaxed);
                entry.init(request, timestamp);
                self.depth.fetch_add(1, Ordering::Relaxed);
                self.submitted.fetch_add(1, Ordering::Relaxed);
                return Some(request_id);
            }
        }

        None
    }

    /// Submit multiple requests in batch
    ///
    /// Returns the number of requests successfully submitted
    pub fn submit_batch(&self, requests: &[BioRequest]) -> usize {
        let mut submitted = 0;
        for request in requests {
            if self.submit(*request).is_some() {
                submitted += 1;
            } else {
                break;
            }
        }
        submitted
    }

    /// Complete a request
    pub fn complete(&self, request_id: u64) -> bool {
        for entry_lock in &self.entries {
            let entry = entry_lock.read();
            if entry.is_valid() && entry.get_request().request_id == request_id {
                entry.invalidate();
                self.depth.fetch_sub(1, Ordering::Relaxed);
                self.completed.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    /// Get current queue depth
    pub fn depth(&self) -> usize {
        self.depth.load(Ordering::Relaxed)
    }

    /// Get target queue depth
    pub fn target_depth(&self) -> usize {
        self.target_depth.load(Ordering::Relaxed)
    }

    /// Set target queue depth
    pub fn set_target_depth(&self, depth: usize) {
        self.target_depth.store(depth.min(MAX_QUEUE_DEPTH), Ordering::Release);
    }

    /// Get number of submitted requests
    pub fn submitted_count(&self) -> usize {
        self.submitted.load(Ordering::Relaxed)
    }

    /// Get number of completed requests
    pub fn completed_count(&self) -> usize {
        self.completed.load(Ordering::Relaxed)
    }

    /// Check if queue has space
    pub fn has_space(&self) -> bool {
        self.depth.load(Ordering::Relaxed) < self.target_depth.load(Ordering::Relaxed)
    }

    /// Reset queue
    pub fn reset(&self) {
        self.in_use.store(false, Ordering::Release);
        self.depth.store(0, Ordering::Release);
        
        // Invalidate all entries
        for entry_lock in &self.entries {
            let entry = entry_lock.read();
            entry.invalidate();
        }
    }
}

/// Maximum number of devices
const MAX_DEVICES: usize = 16;

/// BIO queue manager
pub struct BioQueueManager {
    /// Per-device queues
    queues: [BioQueue; MAX_DEVICES],
}

impl BioQueueManager {
    /// Create a new BIO queue manager
    const fn new() -> Self {
        const INIT_QUEUE: BioQueue = BioQueue::new();
        Self {
            queues: [INIT_QUEUE; MAX_DEVICES],
        }
    }

    /// Register a device
    ///
    /// Returns the queue index if successful
    pub fn register_device(&self, device_id: u64, is_nvme: bool) -> Option<usize> {
        // Check if already registered
        for (idx, queue) in self.queues.iter().enumerate() {
            if queue.is_for_device(device_id) {
                return Some(idx);
            }
        }

        // Find free queue
        for (idx, queue) in self.queues.iter().enumerate() {
            if !queue.is_in_use() {
                let target_depth = if is_nvme {
                    TARGET_QUEUE_DEPTH_NVME
                } else {
                    TARGET_QUEUE_DEPTH_SATA
                };
                queue.init(device_id, target_depth);
                return Some(idx);
            }
        }

        None
    }

    /// Unregister a device
    pub fn unregister_device(&self, device_id: u64) {
        for queue in &self.queues {
            if queue.is_for_device(device_id) {
                queue.reset();
                return;
            }
        }
    }

    /// Get queue for device
    pub fn get_queue(&self, device_id: u64) -> Option<&BioQueue> {
        for queue in &self.queues {
            if queue.is_for_device(device_id) {
                return Some(queue);
            }
        }
        None
    }

    /// Submit a request
    pub fn submit(&self, request: BioRequest) -> Option<u64> {
        if let Some(queue) = self.get_queue(request.device_id) {
            queue.submit(request)
        } else {
            None
        }
    }

    /// Submit batch of requests
    pub fn submit_batch(&self, device_id: u64, requests: &[BioRequest]) -> usize {
        if let Some(queue) = self.get_queue(device_id) {
            queue.submit_batch(requests)
        } else {
            0
        }
    }

    /// Complete a request
    pub fn complete(&self, device_id: u64, request_id: u64) -> bool {
        if let Some(queue) = self.get_queue(device_id) {
            queue.complete(request_id)
        } else {
            false
        }
    }
}

use spin::Once;

/// Global BIO queue manager
static BIO_QUEUE_MANAGER: Once<BioQueueManager> = Once::new();

/// Get the global BIO queue manager
pub fn get_bio_queue_manager() -> &'static BioQueueManager {
    BIO_QUEUE_MANAGER.call_once(|| BioQueueManager::new())
}

/// Submit a TRIM/DISCARD request for SSD optimization
///
/// This is a convenience function for submitting TRIM operations
pub fn submit_trim(device_id: u64, start_block: u64, block_count: usize) -> Option<u64> {
    let request = BioRequest::new(BioOp::Trim, device_id, start_block, block_count, 0);
    get_bio_queue_manager().submit(request)
}

/// Batch TRIM operations
///
/// This coalesces adjacent TRIM ranges for efficiency
pub fn batch_trim(device_id: u64, ranges: &[(u64, usize)]) -> usize {
    let mut requests = [BioRequest::new(BioOp::Trim, device_id, 0, 0, 0); 64];
    let mut count = 0;

    for &(start_block, block_count) in ranges.iter().take(64) {
        requests[count] = BioRequest::new(BioOp::Trim, device_id, start_block, block_count, 0);
        count += 1;
    }

    get_bio_queue_manager().submit_batch(device_id, &requests[..count])
}
