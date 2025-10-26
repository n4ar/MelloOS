# GDB initialization file for MelloOS kernel debugging

# Set architecture
set architecture i386:x86-64

# Enable pretty printing
set print pretty on
set print array on
set print array-indexes on

# Disable pagination
set pagination off

# Show source code context
set listsize 20

# Connect to QEMU (uncomment to auto-connect)
# target remote localhost:1234

# Load kernel symbols
# symbol-file kernel/target/x86_64-unknown-none/debug/kernel

# Useful breakpoints (uncomment as needed)
# break kernel_main
# break panic_handler
# break page_fault_handler

# Custom commands
define hook-stop
    # Show registers on every stop
    # info registers
end

# Print kernel log buffer (if available)
define klog
    # Implement based on your kernel's log structure
    printf "Kernel log not yet implemented\n"
end

# Print current task info
define task
    # Implement based on your TCB structure
    printf "Task info not yet implemented\n"
end

# Print page table info
define pgtable
    # Implement based on your page table structure
    printf "Page table info not yet implemented\n"
end

echo \n
echo ========================================\n
echo MelloOS Kernel Debugger\n
echo ========================================\n
echo Commands:\n
echo   target remote localhost:1234  - Connect to QEMU\n
echo   break kernel_main             - Break at kernel entry\n
echo   continue                      - Continue execution\n
echo   step / next                   - Step through code\n
echo   info registers                - Show CPU registers\n
echo   backtrace                     - Show call stack\n
echo ========================================\n
echo \n
