#!/bin/bash
# Comprehensive Filesystem Test Suite Runner
# Executes all filesystem tests for Milestone M7

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results tracking
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0
SKIPPED_TESTS=0

# Log file
LOG_FILE="filesystem_test_results_$(date +%Y%m%d_%H%M%S).log"

echo "=========================================="
echo "MelloOS Filesystem Test Suite"
echo "=========================================="
echo ""
echo "Log file: $LOG_FILE"
echo ""

# Function to run a test and track results
run_test() {
    local test_name="$1"
    local test_command="$2"
    local required="$3"  # "required" or "optional"
    
    echo -e "${BLUE}Running:${NC} $test_name"
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    
    if eval "$test_command" >> "$LOG_FILE" 2>&1; then
        echo -e "${GREEN}✓ PASS${NC} $test_name"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        if [ "$required" = "required" ]; then
            echo -e "${RED}✗ FAIL${NC} $test_name (REQUIRED)"
            FAILED_TESTS=$((FAILED_TESTS + 1))
            return 1
        else
            echo -e "${YELLOW}⊘ SKIP${NC} $test_name (optional, not implemented)"
            SKIPPED_TESTS=$((SKIPPED_TESTS + 1))
            return 0
        fi
    fi
}

# Initialize log file
echo "MelloOS Filesystem Test Suite" > "$LOG_FILE"
echo "Date: $(date)" >> "$LOG_FILE"
echo "========================================" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"

#
# Milestone M1: VFS Core Infrastructure
#
echo ""
echo "=========================================="
echo "Milestone M1: VFS Core Infrastructure"
echo "=========================================="
echo ""

run_test "VFS Correctness Tests" \
    "cargo test --test fs_vfs_correctness" \
    "optional"

run_test "File Descriptor Operations" \
    "cargo test --test fs_fd_ops" \
    "optional"

run_test "Directory Operations" \
    "cargo test --test fs_dir_ops" \
    "optional"

#
# Milestone M2: Page Cache and Buffer Management
#
echo ""
echo "=========================================="
echo "Milestone M2: Page Cache and Buffer Management"
echo "=========================================="
echo ""

run_test "Cache Behavior Tests" \
    "cargo test --test fs_cache_behavior" \
    "optional"

run_test "Cache Performance Tests" \
    "cargo test --test fs_cache_perf" \
    "optional"

run_test "mmap Coherence Tests" \
    "cargo test --test fs_mmap_coherence" \
    "optional"

#
# Milestone M3: MelloFS RAM Filesystem
#
echo ""
echo "=========================================="
echo "Milestone M3: MelloFS RAM Filesystem"
echo "=========================================="
echo ""

run_test "mfs_ram Correctness Tests" \
    "cargo test --test mfs_ram_correctness" \
    "optional"

run_test "mfs_ram Performance Tests" \
    "cargo test --test mfs_ram_perf" \
    "optional"

#
# Milestone M4: MelloFS Disk Filesystem - Core
#
echo ""
echo "=========================================="
echo "Milestone M4: MelloFS Disk Filesystem - Core"
echo "=========================================="
echo ""

run_test "mfs_disk Metadata Tests" \
    "cargo test --test mfs_disk_meta" \
    "optional"

run_test "mfs_disk Allocation Tests" \
    "cargo test --test mfs_disk_alloc" \
    "optional"

#
# Milestone M5: Data Integrity and Compression
#
echo ""
echo "=========================================="
echo "Milestone M5: Data Integrity and Compression"
echo "=========================================="
echo ""

run_test "mfs_disk Checksum Tests" \
    "cargo test --test mfs_disk_checksum" \
    "optional"

run_test "mfs_disk Replay Tests" \
    "cargo test --test mfs_disk_replay" \
    "optional"

run_test "mfs_disk Compression Tests" \
    "cargo test --test mfs_disk_compress" \
    "optional"

#
# Milestone M6: Complete Syscalls and Userland
#
echo ""
echo "=========================================="
echo "Milestone M6: Complete Syscalls and Userland"
echo "=========================================="
echo ""

run_test "Stat Compatibility Tests" \
    "cargo test --test fs_stat_compat" \
    "optional"

run_test "Extended Attributes Tests" \
    "cargo test --test fs_xattr" \
    "optional"

run_test "Special Files Tests" \
    "cargo test --test fs_special_nodes" \
    "optional"

run_test "Syscall API Tests" \
    "cargo test --test fs_syscalls_api" \
    "optional"

run_test "Userland Smoke Tests" \
    "cargo test --test userland_smoke" \
    "optional"

#
# Milestone M7: Performance Tuning and Robustness
#
echo ""
echo "=========================================="
echo "Milestone M7: Performance Tuning and Robustness"
echo "=========================================="
echo ""

run_test "Fault Injection Tests" \
    "cargo test --test fs_faults" \
    "required"

run_test "Sequential/Random I/O Benchmarks" \
    "cargo test --test fs_seq_rand -- run_all_benchmarks --nocapture" \
    "optional"

run_test "Fork/Exec Benchmarks" \
    "cargo test --test fork_exec_p95 -- run_all_fork_exec_benchmarks --nocapture" \
    "optional"

#
# Summary
#
echo ""
echo "=========================================="
echo "Test Suite Summary"
echo "=========================================="
echo ""
echo "Total tests:   $TOTAL_TESTS"
echo -e "Passed:        ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed:        ${RED}$FAILED_TESTS${NC}"
echo -e "Skipped:       ${YELLOW}$SKIPPED_TESTS${NC}"
echo ""

# Calculate success rate
if [ $TOTAL_TESTS -gt 0 ]; then
    SUCCESS_RATE=$(echo "scale=1; ($PASSED_TESTS * 100) / $TOTAL_TESTS" | bc)
    echo "Success rate:  ${SUCCESS_RATE}%"
fi

echo ""
echo "Detailed results saved to: $LOG_FILE"
echo ""

# Exit with appropriate code
if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}All required tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some required tests failed.${NC}"
    echo "Review the log file for details: $LOG_FILE"
    exit 1
fi
