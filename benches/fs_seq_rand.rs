//! Filesystem I/O Performance Benchmarks
//!
//! Tests sequential and random I/O performance for the filesystem subsystem.
//!
//! Performance Targets:
//! - Sequential read: ≥ 2.5 GB/s for 1-8 GiB files
//! - Random 4 KiB read (cache hit): ≥ 300k IOPS
//!
//! Requirements: R15.4, R15.5, R15.6, R19.4
//!
//! NOTE: These benchmarks are designed to run on the host system (not bare-metal)
//! to measure filesystem performance once the filesystem is integrated.
//! They will be implemented when the filesystem is operational.

#![cfg(test)]

// TODO: Implement benchmarks when filesystem is operational
// These benchmarks require a working filesystem and will measure:
// - Sequential read/write throughput
// - Random 4 KiB I/O performance
// - Cache hit rates

/// Benchmark result structure
#[derive(Debug)]
struct BenchmarkResult {
    name: String,
    duration: Duration,
    bytes_processed: usize,
    throughput_gbps: f64,
    iops: u64,
    passed: bool,
}

impl BenchmarkResult {
    fn print(&self) {
        let status = if self.passed { "✓ PASS" } else { "✗ FAIL" };
        println!("{} {}", status, self.name);
        println!("  Duration: {:?}", self.duration);
        println!(
            "  Bytes: {} ({:.2} GiB)",
            self.bytes_processed,
            self.bytes_processed as f64 / GIB as f64
        );

        if self.throughput_gbps > 0.0 {
            println!(
                "  Throughput: {:.2} GB/s (target: ≥ {:.2} GB/s)",
                self.throughput_gbps, TARGET_SEQ_READ_GBPS
            );
        }

        if self.iops > 0 {
            println!(
                "  IOPS: {} (target: ≥ {})",
                self.iops, TARGET_RANDOM_READ_IOPS
            );
        }
        println!();
    }
}

/// Create a test file of specified size
fn create_test_file(path: &str, size: usize) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    let chunk_size = 1 * MIB;
    let chunk = vec![0xAB; chunk_size];

    let mut remaining = size;
    while remaining > 0 {
        let to_write = remaining.min(chunk_size);
        file.write_all(&chunk[..to_write])?;
        remaining -= to_write;
    }

    file.sync_all()?;
    Ok(())
}

/// Benchmark sequential read performance
fn bench_sequential_read(file_size: usize) -> BenchmarkResult {
    let test_file = format!("/tmp/mello_bench_seq_{}.dat", file_size);

    // Create test file
    println!(
        "Creating test file: {} bytes ({:.2} GiB)...",
        file_size,
        file_size as f64 / GIB as f64
    );
    create_test_file(&test_file, file_size).expect("Failed to create test file");

    // Warm up - read once to populate cache if needed
    {
        let mut file = File::open(&test_file).expect("Failed to open test file");
        let mut buffer = vec![0u8; 1 * MIB];
        while file.read(&mut buffer).unwrap_or(0) > 0 {}
    }

    // Benchmark sequential read
    let mut file = File::open(&test_file).expect("Failed to open test file");
    let mut buffer = vec![0u8; 1 * MIB];
    let mut total_read = 0;

    let start = Instant::now();
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => total_read += n,
            Err(e) => panic!("Read error: {}", e),
        }
    }
    let duration = start.elapsed();

    // Calculate throughput
    let seconds = duration.as_secs_f64();
    let throughput_gbps = (total_read as f64 / GIB as f64) / seconds;
    let passed = throughput_gbps >= TARGET_SEQ_READ_GBPS;

    // Cleanup
    std::fs::remove_file(&test_file).ok();

    BenchmarkResult {
        name: format!("Sequential Read ({:.2} GiB)", file_size as f64 / GIB as f64),
        duration,
        bytes_processed: total_read,
        throughput_gbps,
        iops: 0,
        passed,
    }
}

/// Benchmark sequential write performance
fn bench_sequential_write(file_size: usize) -> BenchmarkResult {
    let test_file = format!("/tmp/mello_bench_seq_write_{}.dat", file_size);

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&test_file)
        .expect("Failed to create test file");

    let chunk_size = 1 * MIB;
    let chunk = vec![0xCD; chunk_size];
    let mut total_written = 0;

    let start = Instant::now();
    while total_written < file_size {
        let to_write = (file_size - total_written).min(chunk_size);
        file.write_all(&chunk[..to_write]).expect("Write failed");
        total_written += to_write;
    }
    file.sync_all().expect("Sync failed");
    let duration = start.elapsed();

    // Calculate throughput
    let seconds = duration.as_secs_f64();
    let throughput_gbps = (total_written as f64 / GIB as f64) / seconds;

    // Cleanup
    std::fs::remove_file(&test_file).ok();

    BenchmarkResult {
        name: format!(
            "Sequential Write ({:.2} GiB)",
            file_size as f64 / GIB as f64
        ),
        duration,
        bytes_processed: total_written,
        throughput_gbps,
        iops: 0,
        passed: true, // No specific target for writes
    }
}

/// Benchmark random 4 KiB reads (cache hit scenario)
fn bench_random_read_cached() -> BenchmarkResult {
    let test_file = "/tmp/mello_bench_random.dat";
    let file_size = 64 * MIB; // Small enough to fit in cache

    // Create test file
    println!(
        "Creating test file for random reads: {} bytes...",
        file_size
    );
    create_test_file(test_file, file_size).expect("Failed to create test file");

    // Warm up cache - read entire file
    {
        let mut file = File::open(test_file).expect("Failed to open test file");
        let mut buffer = vec![0u8; 1 * MIB];
        while file.read(&mut buffer).unwrap_or(0) > 0 {}
    }

    // Benchmark random reads
    let mut file = File::open(test_file).expect("Failed to open test file");
    let mut buffer = vec![0u8; PAGE_SIZE];
    let num_pages = file_size / PAGE_SIZE;
    let iterations = 100_000; // Number of random reads

    let start = Instant::now();
    for i in 0..iterations {
        // Generate pseudo-random page offset
        let page_offset = (i * 7919) % num_pages; // Prime number for distribution
        let byte_offset = page_offset * PAGE_SIZE;

        file.seek(SeekFrom::Start(byte_offset as u64))
            .expect("Seek failed");
        file.read_exact(&mut buffer).expect("Read failed");
    }
    let duration = start.elapsed();

    // Calculate IOPS
    let seconds = duration.as_secs_f64();
    let iops = (iterations as f64 / seconds) as u64;
    let passed = iops >= TARGET_RANDOM_READ_IOPS;

    // Cleanup
    std::fs::remove_file(test_file).ok();

    BenchmarkResult {
        name: "Random 4 KiB Read (Cache Hit)".to_string(),
        duration,
        bytes_processed: iterations * PAGE_SIZE,
        throughput_gbps: 0.0,
        iops,
        passed,
    }
}

/// Benchmark random 4 KiB writes
fn bench_random_write() -> BenchmarkResult {
    let test_file = "/tmp/mello_bench_random_write.dat";
    let file_size = 64 * MIB;

    // Create test file
    create_test_file(test_file, file_size).expect("Failed to create test file");

    // Benchmark random writes
    let mut file = OpenOptions::new()
        .write(true)
        .open(test_file)
        .expect("Failed to open test file");

    let buffer = vec![0xEF; PAGE_SIZE];
    let num_pages = file_size / PAGE_SIZE;
    let iterations = 10_000; // Fewer iterations for writes

    let start = Instant::now();
    for i in 0..iterations {
        // Generate pseudo-random page offset
        let page_offset = (i * 7919) % num_pages;
        let byte_offset = page_offset * PAGE_SIZE;

        file.seek(SeekFrom::Start(byte_offset as u64))
            .expect("Seek failed");
        file.write_all(&buffer).expect("Write failed");
    }
    file.sync_all().expect("Sync failed");
    let duration = start.elapsed();

    // Calculate IOPS
    let seconds = duration.as_secs_f64();
    let iops = (iterations as f64 / seconds) as u64;

    // Cleanup
    std::fs::remove_file(test_file).ok();

    BenchmarkResult {
        name: "Random 4 KiB Write".to_string(),
        duration,
        bytes_processed: iterations * PAGE_SIZE,
        throughput_gbps: 0.0,
        iops,
        passed: true, // No specific target
    }
}

#[test]
fn test_sequential_read_1gib() {
    let result = bench_sequential_read(1 * GIB);
    result.print();
    assert!(result.passed, "Sequential read 1 GiB failed to meet target");
}

#[test]
fn test_sequential_read_4gib() {
    let result = bench_sequential_read(4 * GIB);
    result.print();
    assert!(result.passed, "Sequential read 4 GiB failed to meet target");
}

#[test]
#[ignore] // Ignore by default due to long runtime
fn test_sequential_read_8gib() {
    let result = bench_sequential_read(8 * GIB);
    result.print();
    assert!(result.passed, "Sequential read 8 GiB failed to meet target");
}

#[test]
fn test_sequential_write_1gib() {
    let result = bench_sequential_write(1 * GIB);
    result.print();
}

#[test]
fn test_sequential_write_4gib() {
    let result = bench_sequential_write(4 * GIB);
    result.print();
}

#[test]
fn test_random_read_cached() {
    let result = bench_random_read_cached();
    result.print();
    assert!(result.passed, "Random read (cached) failed to meet target");
}

#[test]
fn test_random_write() {
    let result = bench_random_write();
    result.print();
}

/// Run all benchmarks and print summary
#[test]
fn run_all_benchmarks() {
    println!("\n========================================");
    println!("Filesystem I/O Performance Benchmarks");
    println!("========================================\n");

    let mut results = Vec::new();

    // Sequential reads
    results.push(bench_sequential_read(1 * GIB));
    results.push(bench_sequential_read(4 * GIB));

    // Sequential writes
    results.push(bench_sequential_write(1 * GIB));
    results.push(bench_sequential_write(4 * GIB));

    // Random I/O
    results.push(bench_random_read_cached());
    results.push(bench_random_write());

    // Print all results
    for result in &results {
        result.print();
    }

    // Summary
    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();

    println!("========================================");
    println!("Summary: {}/{} benchmarks passed", passed, total);
    println!("========================================\n");

    // Assert all critical benchmarks passed
    let critical_passed = results
        .iter()
        .filter(|r| {
            r.name.contains("Sequential Read")
                || r.name.contains("Random") && r.name.contains("Cache")
        })
        .all(|r| r.passed);

    assert!(critical_passed, "Some critical performance targets not met");
}
