---
inclusion: always
---

# macOS Environment - QEMU and Command Constraints

## QEMU Execution Requirements

**MANDATORY:** When running QEMU for this project, ALWAYS use `controlBashProcess` with action "start" to run it as a background process.

### Why:
- QEMU is a long-running process (emulator/VM)
- Blocking execution prevents other tasks from proceeding
- User needs to monitor output while continuing work

### Correct Pattern:

```
Use controlBashProcess with:
- action: "start"
- command: "make run" or specific QEMU command
- Monitor with getProcessOutput as needed
```

### Example Commands to Run as Background:
- `make run`
- `make qemu`
- Any script in `tools/qemu/` directory
- Direct QEMU invocations

## macOS Command Limitations

**CRITICAL:** macOS 15 does NOT have the `timeout` command available.

### What This Means:
- ❌ NEVER use `timeout` command in bash scripts or commands
- ❌ Don't suggest `timeout` as a solution for time-limited execution
- ✅ Use alternative approaches: background processes with manual termination, or built-in shell features

### Common Mistake to Avoid:
```bash
# ❌ WRONG - timeout doesn't exist on macOS
timeout 30 ./some-command

# ✅ CORRECT - Use background process or other approach
./some-command &
PROC_PID=$!
# Monitor and kill if needed
```

### Alternative Solutions:
1. **Background processes** with `controlBashProcess`
2. **Manual termination** when needed
3. **Perl/Python one-liners** if timeout behavior is absolutely required
4. **Built-in shell job control** (`&`, `jobs`, `kill`)

## Summary

- **QEMU = Background Process** (always)
- **No `timeout` command** on this macOS system
- Ask user before assuming command availability
