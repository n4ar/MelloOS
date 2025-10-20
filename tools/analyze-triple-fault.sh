#!/bin/bash

# Analyzes QEMU debug log to find triple fault cause

LOG_FILE="${1:-qemu-debug.log}"

if [ ! -f "$LOG_FILE" ]; then
    echo "Error: Log file '$LOG_FILE' not found"
    echo "Usage: $0 [log_file]"
    exit 1
fi

echo "Analyzing $LOG_FILE for triple fault..."
echo ""

# Find CPU reset events
echo "=== CPU RESET EVENTS ==="
grep -n "CPU Reset" "$LOG_FILE" | head -20

echo ""
echo "=== LAST EXCEPTION BEFORE RESET ==="
grep -B 20 "CPU Reset" "$LOG_FILE" | grep "Exception" | tail -5

echo ""
echo "=== CPU STATE AT LAST RESET ==="
grep -A 30 "CPU Reset" "$LOG_FILE" | head -35

echo ""
echo "=== INTERRUPT/EXCEPTION LOG ==="
grep -E "(check_exception|raise_exception|do_interrupt)" "$LOG_FILE" | tail -20

echo ""
echo "=== CRITICAL REGISTER VALUES ==="
# Extract last known register values before crash
awk '/CPU Reset/{found=1} found{print; if(/RIP=/){for(i=0;i<20;i++){getline; print}; exit}}' "$LOG_FILE"

echo ""
echo "Analysis complete. Key things to check:"
echo "1. RIP value - where did it crash?"
echo "2. CR3 value - is page table pointer valid?"
echo "3. CR0/CR4 - are paging/PAE/long mode bits set correctly?"
echo "4. RSP value - is stack pointer valid?"
echo "5. Exception number - what caused the fault?"
