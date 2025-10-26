# Requirements: exec() Implementation and Shell Launch

## Introduction

This specification defines the requirements for implementing the `exec()` system call and enabling the init process to launch an interactive shell (mello-sh), replacing the current test-only init behavior with a proper user-facing shell environment.

**Goal:** Boot MelloOS → Init launches shell → User sees interactive prompt

## Glossary

- **exec()**: System call that replaces the current process image with a new program
- **Init Process**: First userspace process (PID 1) that launches the shell
- **mello-sh**: MelloOS shell program providing interactive command-line interface
- **ELF Loader**: Kernel component that loads executable files into memory
- **Process Image**: Memory layout of a running process (code, data, stack, heap)

---

## Requirements

### Requirement 1: exec() System Call

**User Story:** As a process, I want to replace my current program with a new program, so that I can launch different executables without creating new processes.

#### Acceptance Criteria

1.1 WHEN a process calls `exec(path, argv, envp)`, THE System SHALL load the ELF binary from the specified path

1.2 WHEN the ELF binary is loaded successfully, THE System SHALL replace the current process image with the new program

1.3 WHEN the process image is replaced, THE System SHALL preserve the process ID (PID) and file descriptors

1.4 WHEN exec() succeeds, THE System SHALL NOT return to the caller (execution continues at new program's entry point)

1.5 WHEN exec() fails, THE System SHALL return -1 and preserve the original process image

1.6 WHEN exec() is called, THE System SHALL close all file descriptors marked with O_CLOEXEC flag

1.7 WHEN the new program starts, THE System SHALL pass command-line arguments (argv) to the program

1.8 WHEN the new program starts, THE System SHALL pass environment variables (envp) to the program

---

### Requirement 2: ELF Binary Loading from Filesystem

**User Story:** As the kernel, I want to load ELF binaries from the filesystem, so that exec() can load programs stored on disk.

#### Acceptance Criteria

2.1 WHEN exec() is called with a file path, THE System SHALL resolve the path using VFS

2.2 WHEN the file is found, THE System SHALL open the file and read its contents

2.3 WHEN the file is read, THE System SHALL parse the ELF header and validate the format

2.4 WHEN the ELF is valid, THE System SHALL load program segments into memory at specified addresses

2.5 WHEN segments are loaded, THE System SHALL set appropriate memory permissions (read, write, execute)

2.6 WHEN loading is complete, THE System SHALL close the file descriptor

---

### Requirement 3: Process Image Replacement

**User Story:** As the kernel, I want to safely replace a process's memory image, so that exec() can switch programs without memory leaks or corruption.

#### Acceptance Criteria

3.1 WHEN replacing the process image, THE System SHALL deallocate the old program's memory (code, data, heap)

3.2 WHEN deallocating memory, THE System SHALL preserve the kernel stack and process control structures

3.3 WHEN allocating new memory, THE System SHALL map the new program's segments at correct virtual addresses

3.4 WHEN setting up the new image, THE System SHALL create a new user stack with argv and envp

3.5 WHEN the new stack is created, THE System SHALL place argc, argv, and envp at the top of the stack

3.6 WHEN memory setup is complete, THE System SHALL update the process's instruction pointer to the new entry point

---

### Requirement 4: Init Process Shell Launch

**User Story:** As a user, I want the init process to launch an interactive shell on boot, so that I can interact with the operating system.

#### Acceptance Criteria

4.1 WHEN the init process starts, THE System SHALL locate the mello-sh binary in the filesystem

4.2 WHEN mello-sh is found, THE Init Process SHALL call exec() to replace itself with mello-sh

4.3 WHEN exec() succeeds, THE System SHALL display the shell prompt to the user

4.4 WHEN the shell is running, THE System SHALL accept user input and execute commands

4.5 WHEN the shell exits, THE System SHALL restart the shell (init should respawn)

---

### Requirement 5: Argument and Environment Passing

**User Story:** As a program, I want to receive command-line arguments and environment variables, so that I can configure my behavior based on user input.

#### Acceptance Criteria

5.1 WHEN exec() is called with argv, THE System SHALL copy argument strings to the new process's stack

5.2 WHEN arguments are copied, THE System SHALL create an argv array pointing to each argument string

5.3 WHEN the program starts, THE System SHALL pass argc (argument count) as the first parameter

5.4 WHEN the program starts, THE System SHALL pass argv (argument array) as the second parameter

5.5 WHEN envp is provided, THE System SHALL copy environment strings to the stack

5.6 WHEN environment is copied, THE System SHALL create an envp array pointing to each environment string

5.7 WHEN the program starts, THE System SHALL pass envp as the third parameter

5.8 WHEN no environment is provided, THE System SHALL pass NULL as envp

---

### Requirement 6: File Descriptor Handling

**User Story:** As a process, I want to control which file descriptors are inherited across exec(), so that I can manage resource cleanup properly.

#### Acceptance Criteria

6.1 WHEN exec() is called, THE System SHALL preserve all open file descriptors by default

6.2 WHEN a file descriptor has the O_CLOEXEC flag set, THE System SHALL close it during exec()

6.3 WHEN file descriptors are preserved, THE System SHALL maintain their file positions and flags

6.4 WHEN stdin/stdout/stderr (fds 0, 1, 2) are open, THE System SHALL preserve them for the new program

---

### Requirement 7: Error Handling

**User Story:** As a developer, I want clear error codes when exec() fails, so that I can diagnose and handle failures appropriately.

#### Acceptance Criteria

7.1 WHEN the specified file does not exist, THE System SHALL return ENOENT (-2)

7.2 WHEN the file is not executable, THE System SHALL return EACCES (-13)

7.3 WHEN the file is not a valid ELF binary, THE System SHALL return ENOEXEC (-8)

7.4 WHEN memory allocation fails, THE System SHALL return ENOMEM (-12)

7.5 WHEN an error occurs, THE System SHALL preserve the original process image

7.6 WHEN an error occurs, THE System SHALL log the error to the kernel log

---

### Requirement 8: Security and Validation

**User Story:** As the kernel, I want to validate exec() parameters, so that malicious or buggy programs cannot compromise system security.

#### Acceptance Criteria

8.1 WHEN exec() is called, THE System SHALL validate that the path pointer is in user space

8.2 WHEN argv is provided, THE System SHALL validate that all argument pointers are in user space

8.3 WHEN envp is provided, THE System SHALL validate that all environment pointers are in user space

8.4 WHEN copying strings, THE System SHALL enforce maximum string length limits

8.5 WHEN loading the ELF, THE System SHALL validate that program segments do not overlap kernel space

8.6 WHEN setting permissions, THE System SHALL enforce W^X (write XOR execute) policy

---

### Requirement 9: Shell Integration

**User Story:** As a user, I want the shell to work correctly after exec(), so that I can use all shell features.

#### Acceptance Criteria

9.1 WHEN mello-sh starts, THE Shell SHALL display a prompt (e.g., "mello$ ")

9.2 WHEN the user types a command, THE Shell SHALL read input from stdin

9.3 WHEN a command is entered, THE Shell SHALL parse and execute the command

9.4 WHEN executing commands, THE Shell SHALL use fork() and exec() to launch programs

9.5 WHEN a command completes, THE Shell SHALL display the prompt again

9.6 WHEN the shell receives SIGINT (Ctrl+C), THE Shell SHALL cancel the current command

9.7 WHEN the shell receives EOF (Ctrl+D), THE Shell SHALL exit gracefully

---

### Requirement 10: Binary Location and Loading

**User Story:** As the system, I want to locate shell binaries in a standard location, so that init can reliably find and launch the shell.

#### Acceptance Criteria

10.1 WHEN the system boots, THE System SHALL embed mello-sh binary in the kernel image

10.2 WHEN init starts, THE System SHALL extract mello-sh to the filesystem (e.g., /bin/sh)

10.3 WHEN init calls exec(), THE System SHALL load mello-sh from /bin/sh

10.4 WHEN the binary is not found, THE System SHALL fall back to a built-in shell or panic

---

## Success Criteria

The implementation is considered successful when:

1. ✅ Boot MelloOS → See "mello$ " prompt
2. ✅ Type "echo hello" → See "hello" output
3. ✅ Type "ls" → See directory listing
4. ✅ Type "ps" → See process list
5. ✅ Press Ctrl+C → Command cancels, prompt returns
6. ✅ Press Ctrl+D → Shell exits and respawns
7. ✅ exec() syscall works for any valid ELF binary
8. ✅ File descriptors are properly inherited/closed
9. ✅ Arguments and environment are correctly passed
10. ✅ Error handling works for invalid binaries

---

## Non-Functional Requirements

### Performance
- exec() SHALL complete in less than 10ms for typical binaries
- Shell prompt SHALL appear within 100ms of boot completion

### Reliability
- exec() SHALL NOT leak memory on success or failure
- Shell SHALL NOT crash on invalid input
- System SHALL remain stable if shell exits

### Usability
- Shell prompt SHALL be clearly visible
- Error messages SHALL be informative
- Command history SHALL work (if implemented)

---

## Dependencies

- Phase 6: Userland Foundation (fork, exec, wait syscalls)
- Phase 8: Filesystem & Storage (VFS, file operations)
- Existing: mello-sh, mello-term, mellobox binaries
- Existing: ELF loader infrastructure

---

## References

- POSIX exec() specification
- ELF format specification
- Linux exec() implementation
- MelloOS Phase 6 design documents

---

**Document Version:** 1.0  
**Status:** Draft  
**Priority:** HIGH - Required for usable system
