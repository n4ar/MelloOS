# GDB script for debugging SMP initialization
# Usage: qemu-system-x86_64 -s -S ... (in one terminal)
#        gdb -x tools/gdb-smp.gdb (in another terminal)

# Connect to QEMU
target remote localhost:1234

# Set architecture
set architecture i386:x86-64

# Load kernel symbols (adjust path if needed)
# symbol-file target/x86_64-unknown-none/debug/mellos

# Breakpoints for SMP debugging
echo Setting breakpoints...\n

# Break at trampoline start (physical address 0x8000)
break *0x8000
commands
  echo \n=== TRAMPOLINE START (16-bit real mode) ===\n
  info registers cs eip
  x/4i $pc
  continue
end

# Break at protected mode entry (approximate)
break *0x8050
commands
  echo \n=== PROTECTED MODE ENTRY (32-bit) ===\n
  info registers eip cr0 cr3 cr4
  x/4i $pc
  continue
end

# Break at long mode entry (approximate)
break *0x80a0
commands
  echo \n=== LONG MODE ENTRY (64-bit) ===\n
  info registers rip rsp cr0 cr3 cr4
  x/4i $pc
  continue
end

# Break on triple fault (CPU exception)
catch signal SIGSEGV

# Display helpful info on each stop
define hook-stop
  echo \n=== CPU STATE ===\n
  info registers
  echo \n=== CODE ===\n
  x/5i $pc
  echo \n=== STACK ===\n
  x/8gx $rsp
  echo \n
end

# Helper commands
define show-paging
  echo \n=== PAGING STRUCTURES ===\n
  printf "CR3: 0x%lx\n", $cr3
  
  # Show PML4 entry 0
  set $pml4 = (unsigned long*)($cr3 & 0xffffffffff000)
  printf "PML4[0]: 0x%lx\n", *$pml4
  
  if ((*$pml4) & 1)
    # Show PDPT entry 0
    set $pdpt = (unsigned long*)((*$pml4) & 0xffffffffff000)
    printf "PDPT[0]: 0x%lx\n", *$pdpt
    
    if ((*$pdpt) & 1)
      # Show PD entry 0
      set $pd = (unsigned long*)((*$pdpt) & 0xffffffffff000)
      printf "PD[0]: 0x%lx\n", *$pd
    end
  end
  echo \n
end

define show-trampoline-data
  echo \n=== TRAMPOLINE DATA ===\n
  printf "Stack pointer: 0x%lx\n", *(unsigned long*)0x8300
  printf "Entry point:   0x%lx\n", *(unsigned long*)0x8308
  printf "CR3 value:     0x%lx\n", *(unsigned long*)0x8310
  printf "CPU ID:        %ld\n", *(unsigned long*)0x8318
  echo \n
end

define show-ap-state
  echo \n=== APPLICATION PROCESSOR STATE ===\n
  info threads
  thread 2
  info registers
  x/10i $pc
  echo \n
end

echo \n
echo GDB ready for SMP debugging\n
echo Commands:\n
echo   show-paging          - Display page table structure\n
echo   show-trampoline-data - Display trampoline configuration\n
echo   show-ap-state        - Show AP CPU state\n
echo   info threads         - List all CPU threads\n
echo   thread N             - Switch to CPU N\n
echo \n
echo Starting execution...\n

# Start execution
continue
