//! Process Creation Performance Benchmark
//!
//! Measures fork+exec latency at P95 (95th percentile).
//!
//! Performance Target:
//! - fork+exec P95 latency: < 1.5 ms
//!
//! Requirements: R15.6, R19.4

#![cfg(test)]

use std::process::Command;
use std::time::{Duration, Instant};

/// Performance target
const TARGET_P95_MS: f64 = 1.5;

/// Number of iterations for statistical significance
const ITERATIONS: usize = 1000;

/// Benchmark result structure
#[derive(Debug)]
struct ForkExecResult {
    samples: Vec<Duration>,
    min: Duration,
    max: Duration,
    mean: Duration,
    median: Duration,
    p95: Duration,
    p99: Duration,
    passed: bool,
}

impl ForkExecResult {
    fn from_samples(mut samples: Vec<Duration>) -> Self {
        samples.sort();
        
        let min = samples[0];
        let max = samples[samples.len() - 1];
        
        let sum: Duration = samples.iter().sum();
        let mean = sum / samples.len() as u32;
        
        let median = samples[samples.len() / 2];
        let p95 = samples[(samples.len() as f64 * 0.95) as usize];
        let p99 = samples[(samples.len() as f64 * 0.99) as usize];
        
        let p95_ms = p95.as_secs_f64() * 1000.0;
        let passed = p95_ms < TARGET_P95_MS;
        
        Self {
            samples,
            min,
            max,
            mean,
            median,
            p95,
            p99,
            passed,
        }
    }
    
    fn print(&self) {
        println!("\n========================================");
        println!("Fork+Exec Performance Benchmark");
        println!("========================================");
        println!("Iterations: {}", self.samples.len());
        println!();
        
        println!("Latency Statistics:");
        println!("  Min:    {:?} ({:.3} ms)", self.min, self.min.as_secs_f64() * 1000.0);
        println!("  Mean:   {:?} ({:.3} ms)", self.mean, self.mean.as_secs_f64() * 1000.0);
        println!("  Median: {:?} ({:.3} ms)", self.median, self.median.as_secs_f64() * 1000.0);
        println!("  P95:    {:?} ({:.3} ms)", self.p95, self.p95.as_secs_f64() * 1000.0);
        println!("  P99:    {:?} ({:.3} ms)", self.p99, self.p99.as_secs_f64() * 1000.0);
        println!("  Max:    {:?} ({:.3} ms)", self.max, self.max.as_secs_f64() * 1000.0);
        println!();
        
        let p95_ms = self.p95.as_secs_f64() * 1000.0;
        let status = if self.passed { "✓ PASS" } else { "✗ FAIL" };
        println!("{} P95 Latency: {:.3} ms (target: < {:.1} ms)", 
                 status, p95_ms, TARGET_P95_MS);
        println!("========================================\n");
    }
}

/// Benchmark fork+exec of a small binary
fn bench_fork_exec(binary: &str, iterations: usize) -> ForkExecResult {
    let mut samples = Vec::with_capacity(iterations);
    
    println!("Benchmarking fork+exec of '{}' ({} iterations)...", binary, iterations);
    
    // Warm up - run a few times to ensure binary is cached
    for _ in 0..10 {
        Command::new(binary)
            .output()
            .expect("Failed to execute command");
    }
    
    // Collect samples
    for i in 0..iterations {
        if i % 100 == 0 && i > 0 {
            println!("  Progress: {}/{}", i, iterations);
        }
        
        let start = Instant::now();
        let output = Command::new(binary)
            .output()
            .expect("Failed to execute command");
        let duration = start.elapsed();
        
        // Verify command succeeded
        assert!(output.status.success(), "Command failed");
        
        samples.push(duration);
    }
    
    ForkExecResult::from_samples(samples)
}

/// Benchmark fork+exec with arguments
fn bench_fork_exec_with_args(binary: &str, args: &[&str], iterations: usize) -> ForkExecResult {
    let mut samples = Vec::with_capacity(iterations);
    
    println!("Benchmarking fork+exec of '{}' with args ({} iterations)...", 
             binary, iterations);
    
    // Warm up
    for _ in 0..10 {
        Command::new(binary)
            .args(args)
            .output()
            .expect("Failed to execute command");
    }
    
    // Collect samples
    for i in 0..iterations {
        if i % 100 == 0 && i > 0 {
            println!("  Progress: {}/{}", i, iterations);
        }
        
        let start = Instant::now();
        let output = Command::new(binary)
            .args(args)
            .output()
            .expect("Failed to execute command");
        let duration = start.elapsed();
        
        assert!(output.status.success(), "Command failed");
        samples.push(duration);
    }
    
    ForkExecResult::from_samples(samples)
}

#[test]
fn test_fork_exec_true() {
    // Benchmark /bin/true - smallest possible binary
    let result = bench_fork_exec("/bin/true", ITERATIONS);
    result.print();
    assert!(result.passed, "fork+exec P95 latency exceeds target");
}

#[test]
fn test_fork_exec_echo() {
    // Benchmark /bin/echo with arguments
    let result = bench_fork_exec_with_args("/bin/echo", &["hello", "world"], ITERATIONS);
    result.print();
    assert!(result.passed, "fork+exec P95 latency exceeds target");
}

#[test]
fn test_fork_exec_ls() {
    // Benchmark ls command (slightly larger binary)
    let result = bench_fork_exec_with_args("/bin/ls", &["/tmp"], ITERATIONS);
    result.print();
    // Note: ls is larger, so we don't assert on this one
}

#[test]
#[ignore] // Ignore by default - requires MelloOS environment
fn test_fork_exec_mellobox() {
    // Benchmark MelloOS-specific binary
    let result = bench_fork_exec("/bin/mellobox", ITERATIONS);
    result.print();
}

/// Run comprehensive fork+exec benchmarks
#[test]
fn run_all_fork_exec_benchmarks() {
    println!("\n========================================");
    println!("Process Creation Benchmarks");
    println!("========================================\n");
    
    // Test different binaries
    let binaries = vec![
        ("/bin/true", vec![]),
        ("/bin/echo", vec!["test"]),
        ("/bin/cat", vec!["/dev/null"]),
    ];
    
    let mut all_passed = true;
    
    for (binary, args) in binaries {
        let result = if args.is_empty() {
            bench_fork_exec(binary, ITERATIONS)
        } else {
            bench_fork_exec_with_args(binary, &args.iter().map(|s| *s).collect::<Vec<_>>(), ITERATIONS)
        };
        
        result.print();
        
        if !result.passed {
            all_passed = false;
        }
    }
    
    println!("\n========================================");
    if all_passed {
        println!("✓ All fork+exec benchmarks passed");
    } else {
        println!("✗ Some fork+exec benchmarks failed");
    }
    println!("========================================\n");
    
    assert!(all_passed, "Some fork+exec benchmarks failed to meet targets");
}

/// Benchmark fork+exec under load (concurrent processes)
#[test]
#[ignore] // Ignore by default - stress test
fn test_fork_exec_concurrent() {
    use std::thread;
    
    println!("\nBenchmarking fork+exec under concurrent load...");
    
    let num_threads = 4;
    let iterations_per_thread = 250;
    
    let handles: Vec<_> = (0..num_threads)
        .map(|i| {
            thread::spawn(move || {
                println!("Thread {} starting...", i);
                let mut samples = Vec::new();
                
                for _ in 0..iterations_per_thread {
                    let start = Instant::now();
                    Command::new("/bin/true")
                        .output()
                        .expect("Failed to execute");
                    samples.push(start.elapsed());
                }
                
                samples
            })
        })
        .collect();
    
    // Collect all samples
    let mut all_samples = Vec::new();
    for handle in handles {
        all_samples.extend(handle.join().unwrap());
    }
    
    let result = ForkExecResult::from_samples(all_samples);
    result.print();
    
    println!("Note: Concurrent load test - P95 target may be relaxed");
}
