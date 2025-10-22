# MelloOS Performance Benchmarks

This document describes the performance benchmarking suite for MelloOS Advanced Userland & Shell Environment (Phase 6.6).

## Overview

The benchmark suite measures five key performance metrics that are critical for interactive system usability:

1. **Shell Startup Time** - Time to launch and exit the shell
2. **Process Spawn Time** - Time to fork and execute a simple process
3. **Pipe Throughput** - Data transfer rate through pipes
4. **Directory Listing** - Time to list a directory with many files
5. **Syscall Latency** - Time for basic read/write system calls

## Performance Targets

Based on requirements 8.1-8.5, the following targets must be met:

| Metric | Target | Requirement |
|--------|--------|-------------|
| Shell startup | < 10ms | 8.1 |
| Process spawn (/bin/true) | < 2ms per iteration | 8.2 |
| Pipe throughput | > 200 MB/s | 8.3 |
| Directory listing (1000 files) | < 80ms | 8.4 |
| Syscall latency (4KB read/write) | < 5µs median | 8.5 |

## Benchmark Scripts

### 1. `benchmark_performance.sh`

General-purpose benchmark script that can run on the host system to test baseline performance.

**Usage:**
```bash
cd tools/testing
./benchmark_performance.sh
```

**Output:**
- Console output with pass/fail for each benchmark
- `benchmark_results.txt` with detailed results

### 2. `benchmark_mellos.sh`

MelloOS-specific benchmark preparation script. Generates commands to run inside MelloOS.

**Usage:**
```bash
cd tools/testing
./benchmark_mellos.sh
```

This script prepares benchmark commands but requires manual execution inside MelloOS.

## Running Benchmarks in MelloOS

### Prerequisites

1. Build MelloOS with all userland components:
   ```bash
   make build
   ```

2. Ensure the following are available:
   - `mello-sh` (shell)
   - `mellobox` with utilities: `ls`, `cat`, `echo`, `true`
   - `/proc` filesystem mounted
   - PTY subsystem functional

### Manual Benchmark Execution

1. **Boot MelloOS:**
   ```bash
   make run
   ```

2. **Run each benchmark manually in the shell:**

#### Benchmark 1: Shell Startup
```bash
time mello-sh -c 'exit'
```
**Expected:** < 10ms

#### Benchmark 2: Process Spawn
```bash
time mello-sh -c 'for i in {1..100}; do /bin/true; done'
```
**Expected:** Total < 200ms (< 2ms per spawn)

#### Benchmark 3: Pipe Throughput
```bash
dd if=/dev/zero bs=1M count=100 | cat > /dev/null
```
**Expected:** > 200 MB/s

#### Benchmark 4: Directory Listing
```bash
# Create test directory
mkdir -p /tmp/bench_test
for i in {1..1000}; do touch /tmp/bench_test/file_$i.txt; done

# Benchmark
time ls -la /tmp/bench_test

# Cleanup
rm -rf /tmp/bench_test
```
**Expected:** < 80ms

#### Benchmark 5: Syscall Latency
```bash
# Create 4KB test file
dd if=/dev/zero of=/tmp/test.dat bs=4096 count=1

# Warm cache
cat /tmp/test.dat > /dev/null

# Benchmark (10000 iterations)
time sh -c 'for i in {1..10000}; do cat /tmp/test.dat > /dev/null; done'

# Calculate per-call latency: total_time / 10000
```
**Expected:** < 5µs per call

## Interpreting Results

### Shell Startup Time

Measures the overhead of:
- Process creation (fork)
- ELF loading
- Shell initialization
- Environment setup
- Exit cleanup

**Optimization targets:**
- Fast ELF loader
- Minimal shell initialization
- Efficient memory allocation

### Process Spawn Time

Measures:
- Fork system call overhead
- Process group setup
- Execve system call
- Minimal program execution

**Optimization targets:**
- Fast fork (copy-on-write pages)
- Efficient scheduler
- Quick context switching

### Pipe Throughput

Measures:
- Pipe buffer management
- Data copying efficiency
- Context switching overhead
- Scheduler responsiveness

**Optimization targets:**
- Large pipe buffers (64KB+)
- Zero-copy where possible
- Efficient wake-up mechanisms

### Directory Listing

Measures:
- Filesystem performance
- System call overhead (getdents)
- String formatting
- Terminal output

**Optimization targets:**
- Efficient directory iteration
- Batch system calls
- Fast string operations

### Syscall Latency

Measures:
- Syscall entry/exit overhead
- User/kernel transition
- Cache effects
- TLB performance

**Optimization targets:**
- Fast syscall instruction (SYSCALL/SYSRET)
- Minimal register saving
- Hot cache paths

## Profiling and Optimization

### Tools

1. **Kernel Metrics** (`/proc/stat`):
   - Context switches
   - Syscall counts
   - Interrupt counts

2. **Per-Process Metrics** (`/proc/<pid>/stat`):
   - User time
   - System time
   - Page faults

3. **Debug Logging**:
   - Enable TRACE level for hot paths
   - Measure time between log points

### Hot Paths to Profile

1. **Syscall Entry/Exit**:
   - `kernel/src/arch/x86_64/syscall/entry.S`
   - `kernel/src/sys/syscall.rs`

2. **PTY Read/Write**:
   - `kernel/src/dev/pty/mod.rs`
   - Ring buffer operations

3. **Signal Delivery**:
   - `kernel/src/signal/mod.rs`
   - Signal handler invocation

4. **Scheduler**:
   - `kernel/src/sched/mod.rs`
   - Task switching
   - Priority calculations

### Optimization Strategies

1. **Reduce Lock Contention**:
   - Use per-CPU data structures
   - Fine-grained locking
   - Lock-free algorithms where possible

2. **Optimize Memory Access**:
   - Cache-friendly data structures
   - Reduce cache line bouncing
   - Align hot structures

3. **Minimize Context Switches**:
   - Batch operations
   - Reduce unnecessary wake-ups
   - Efficient wait queues

4. **Fast Paths**:
   - Inline hot functions
   - Avoid branches in critical paths
   - Use likely/unlikely hints

## Continuous Integration

### Automated Benchmarking

Add to CI pipeline:

```yaml
benchmark:
  script:
    - make build
    - ./tools/testing/benchmark_performance.sh
  artifacts:
    paths:
      - benchmark_results.txt
  only:
    - main
    - performance/*
```

### Performance Regression Detection

Track metrics over time:
- Store results in database
- Compare against baseline
- Alert on regressions > 10%

### Benchmark Environments

Run benchmarks in consistent environments:
- **KVM**: 4 vCPUs, 2GB RAM
- **CPU Governor**: performance mode
- **Isolated CPUs**: Reduce noise
- **Multiple runs**: Average 10 iterations

## Troubleshooting

### Benchmark Fails to Run

**Problem:** Script exits with error

**Solutions:**
- Check MelloOS is built: `make build`
- Verify QEMU is installed
- Check file permissions: `chmod +x benchmark_*.sh`

### Results Don't Meet Targets

**Problem:** Performance below expectations

**Investigation:**
1. Check CPU governor: `cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor`
2. Verify no background load
3. Run multiple times to average
4. Profile hot paths with kernel metrics

### Inconsistent Results

**Problem:** High variance between runs

**Solutions:**
- Warm up caches before measuring
- Run more iterations
- Isolate CPUs
- Disable frequency scaling

## Future Enhancements

### Additional Benchmarks

1. **Memory Allocation**:
   - Malloc/free latency
   - Fragmentation over time

2. **IPC Performance**:
   - Message passing latency
   - Shared memory throughput

3. **Signal Delivery**:
   - Signal latency
   - Signal handling overhead

4. **Job Control**:
   - Process group operations
   - Foreground/background switching

### Advanced Profiling

1. **Hardware Performance Counters**:
   - Cache misses
   - Branch mispredictions
   - TLB misses

2. **Flame Graphs**:
   - Visualize hot paths
   - Identify bottlenecks

3. **Lock Contention Analysis**:
   - Track lock wait times
   - Identify contended locks

## References

- Requirements: `.kiro/specs/advanced-userland-shell/requirements.md`
- Design: `.kiro/specs/advanced-userland-shell/design.md`
- Performance targets: Requirements 8.1-8.5
