---
inclusion: always
---

# Complete Implementation Policy - MANDATORY

## Core Principle: No Half-Finished Work

**ALWAYS implement features completely. NEVER leave TODO comments or incomplete implementations.**

### Why This Matters

- TODO comments become technical debt
- Incomplete implementations cause bugs and confusion
- Future developers (including AI) waste time tracking down unfinished work
- Half-finished features are harder to complete later than doing it right the first time

## Required Behavior

### ❌ NEVER Do This:

```rust
// TODO: Add error handling later
fn process_data(data: &[u8]) -> Result<(), Error> {
    // Basic implementation only
    Ok(())
}

// TODO: Implement proper validation
fn validate_input(input: &str) -> bool {
    true  // Always returns true for now
}

// FIXME: This is a temporary hack
let value = unsafe { *(ptr as *const u32) };
```

### ✅ ALWAYS Do This:

```rust
// Complete implementation with full error handling
fn process_data(data: &[u8]) -> Result<ProcessedData, Error> {
    if data.is_empty() {
        return Err(Error::EmptyData);
    }
    
    // Full validation
    validate_data(data)?;
    
    // Complete processing logic
    let result = transform_data(data)?;
    
    Ok(result)
}

// Fully implemented validation
fn validate_input(input: &str) -> Result<(), ValidationError> {
    if input.is_empty() {
        return Err(ValidationError::Empty);
    }
    if input.len() > MAX_LENGTH {
        return Err(ValidationError::TooLong);
    }
    // All validation rules implemented
    Ok(())
}
```

## Implementation Checklist

Before considering any feature "done", verify:

- [ ] All error cases handled explicitly
- [ ] All edge cases covered
- [ ] Input validation complete
- [ ] Resource cleanup implemented (no leaks)
- [ ] Proper error types defined and used
- [ ] Safety invariants documented for unsafe code
- [ ] Integration with existing systems complete
- [ ] No TODO, FIXME, or HACK comments remain
- [ ] Code passes `cargo check` without warnings
- [ ] Functionality tested and verified

## Acceptable Markers

Only these markers are acceptable in production code:

### ✅ UNIMPLEMENTED (for explicit panics)

```rust
fn future_feature() -> Result<(), Error> {
    unimplemented!("This feature is planned for Phase 10")
}
```

Use only when:
- Feature is explicitly deferred to a future phase
- Calling code should panic if reached
- Documented in roadmap or spec

### ✅ Documentation TODOs (in comments only)

```rust
/// Processes network packets.
///
/// # TODO (Documentation)
/// Add examples once networking phase is complete.
pub fn process_packet(packet: &[u8]) -> Result<(), Error> {
    // Full implementation here
}
```

Use only for:
- Missing documentation examples
- Future documentation improvements
- Never for missing code

## Common Scenarios

### Scenario 1: Complex Feature

**Wrong approach:**
```rust
// Phase 1: Basic implementation
fn handle_signal(sig: Signal) {
    // TODO: Add signal masking
    // TODO: Add signal queuing
    // TODO: Handle signal inheritance
    deliver_signal(sig);
}
```

**Correct approach:**
```rust
// Complete implementation from the start
fn handle_signal(sig: Signal, task: &Task) -> Result<(), SignalError> {
    // Check if signal is masked
    if task.signal_mask.is_blocked(sig) {
        task.pending_signals.queue(sig)?;
        return Ok(());
    }
    
    // Handle signal inheritance for child processes
    if sig.should_inherit() {
        propagate_to_children(task, sig)?;
    }
    
    // Deliver signal with full error handling
    deliver_signal(task, sig)?;
    
    Ok(())
}
```

### Scenario 2: Error Handling

**Wrong approach:**
```rust
fn read_file(path: &str) -> Vec<u8> {
    // TODO: Handle errors properly
    std::fs::read(path).unwrap()
}
```

**Correct approach:**
```rust
fn read_file(path: &str) -> Result<Vec<u8>, FileError> {
    if path.is_empty() {
        return Err(FileError::InvalidPath);
    }
    
    let data = std::fs::read(path)
        .map_err(|e| FileError::ReadFailed(e))?;
    
    if data.is_empty() {
        return Err(FileError::EmptyFile);
    }
    
    Ok(data)
}
```

### Scenario 3: Resource Management

**Wrong approach:**
```rust
fn allocate_buffer() -> *mut u8 {
    // TODO: Add proper cleanup
    unsafe { alloc(Layout::new::<[u8; 4096]>()) }
}
```

**Correct approach:**
```rust
struct Buffer {
    ptr: *mut u8,
    layout: Layout,
}

impl Buffer {
    fn new(size: usize) -> Result<Self, AllocError> {
        let layout = Layout::from_size_align(size, 4096)
            .map_err(|_| AllocError::InvalidLayout)?;
        
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err(AllocError::OutOfMemory);
        }
        
        Ok(Buffer { ptr, layout })
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) }
    }
}
```

## Integration with Development Workflow

### When Writing New Code

1. **Plan the complete implementation** before writing
2. **Identify all error cases** upfront
3. **Write full implementation** in one go
4. **Test thoroughly** before moving on
5. **Run cargo check** to verify completeness

### When Reviewing Existing Code

If you encounter TODO/FIXME comments:
1. **Implement the missing functionality** immediately
2. **Remove the TODO comment** after implementation
3. **Test the completed feature**
4. **Update documentation** if needed

### When Implementing from Specs

Follow the spec's tasks.md completely:
- Don't mark a task complete until fully implemented
- Don't skip subtasks or edge cases
- Don't defer error handling or validation
- Complete all acceptance criteria before moving on

## Exceptions

The ONLY acceptable reasons to defer implementation:

### 1. Cross-Phase Dependencies

```rust
// Network stack not yet implemented (Phase 9)
// This will be completed in Phase 9 per roadmap
fn send_network_packet(data: &[u8]) -> Result<(), Error> {
    Err(Error::NotImplemented("Networking is Phase 9"))
}
```

### 2. Hardware Not Available

```rust
// AHCI driver deferred until real hardware testing
// virtio-blk provides sufficient functionality for current phase
fn init_ahci() -> Result<(), Error> {
    Err(Error::NotImplemented("AHCI deferred per Phase 7 notes"))
}
```

### 3. Explicitly Documented in Roadmap

Only if the roadmap explicitly states a feature is deferred to a future phase.

## Summary

**Remember:**
- Complete implementations only
- No TODO comments for missing code
- Handle all errors explicitly
- Test before moving on
- Technical debt is expensive

**Benefits:**
- Higher code quality
- Fewer bugs
- Easier maintenance
- No surprises later
- Faster overall development

---

**This policy is MANDATORY for all MelloOS development.**