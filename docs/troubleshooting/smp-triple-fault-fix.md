# SMP Triple Fault Fix – October 2025

## Symptoms
- AP (Application Processor) hit a triple fault immediately after entering `ap_entry64`.
- Serial markers showed the AP reached Rust (`RJW123ABC`) but died before printing marker `E` inside `init_percpu`.
- QEMU `-d int` logs contained:
  - `#PF` (#14) at `RIP=ffffffff8000780e` with `error=0x0a`.
  - Rapid cascade to #GP and reset (`check_exception old: 0xe new 0xd`), confirming triple fault.

## Root Cause
- The BSP enables NX (No-Execute) and CR0.WP during early memory init.
- APs entered long mode with LME set but **without NXE** (bit 11 in EFER).
- When `init_percpu` wrote to `PerCpu.id`, the destination page was marked NX=1 from the BSP setup.
- CPU observed mismatched CPU feature state (NX disabled but page NX bit set) and raised `#PF` with RSVD bit set, escalating into a triple fault.

## Fix
1. **Match BSP feature configuration on each AP (`kernel/src/arch/x86_64/smp/mod.rs`).**
   - Call `crate::mm::enable_nx_bit()` and `crate::mm::enable_write_protect()` at the top of `ap_entry64`.
   - Ensures CR0.WP and EFER.NXE mirror the BSP before touching per-CPU data.

2. **Enable NX directly in the trampoline (`kernel/src/arch/x86_64/smp/boot_ap.S`).**
   - Change the EFER write to set both `LME` and `NXE` (`orl $0x900, %eax`).
   - Guarantees NX is active before the jump to higher-half Rust code, even if Rust-side helpers were skipped.

## Verification
1. `make iso`
2. `qemu-system-x86_64 -serial mon:stdio -smp 4 -m 2G -cdrom mellos.iso -no-reboot -d int -D qemu-debug.log`
3. Observe:
   - Serial markers include `RJW123ABCXYEFGHIJKLD456` for each AP.
   - No new `#PF` entries in `qemu-debug.log`.
   - Scheduler continues running Phase 4/5 test tasks; AP timers initialized.

## Remaining Notes / Follow-up
- During testing, LAPIC ID reads returned 0 for every AP, producing warnings:
  - `[SMP] AP#X warning: LAPIC ID mismatch (expected X, got 0)`
  - Core still boots, but confirm `LocalApic::id()` is reading the correct register (APIC version vs. ID) or evaluate using x2APIC MSR.
- Keep the debug markers in `ap_entry64` and the trampoline until the LAPIC ID quirk is resolved.
- If you revert either fix above, the triple fault returns immediately; keep both changes together.

## References
- Files touched:
  - `kernel/src/arch/x86_64/smp/mod.rs`
  - `kernel/src/arch/x86_64/smp/boot_ap.S`
- Related docs:
  - `docs/DEBUG-SMP-TRIPLE-FAULT.md` (detailed troubleshooting playbook)
