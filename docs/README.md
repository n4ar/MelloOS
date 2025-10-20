# MelloOS Documentation

Comprehensive documentation for MelloOS kernel development, architecture, and usage.

## 📚 Documentation Structure

```
docs/
├── architecture/       # System architecture and design documents
├── development/        # Development guides and API documentation
├── troubleshooting/    # Debugging guides and issue resolution
└── README.md          # This file
```

## 🏗️ Architecture Documentation (`architecture/`)

Core system design and implementation details:

- **[architecture.md](architecture/architecture.md)**: Complete system architecture overview
- **[smp.md](architecture/smp.md)**: SMP (Multi-core) implementation details
- **[task-scheduler.md](architecture/task-scheduler.md)**: Task scheduler design and algorithms
- **[memory-management-logging.md](architecture/memory-management-logging.md)**: Memory management subsystem

## 🛠️ Development Documentation (`development/`)

Guides for developers working on MelloOS:

- **[api-guide.md](development/api-guide.md)**: API usage examples and best practices
- **[testing.md](development/testing.md)**: Testing procedures and verification methods

## 🐛 Troubleshooting Documentation (`troubleshooting/`)

Debugging guides and issue resolution:

- **[troubleshooting.md](troubleshooting/troubleshooting.md)**: General troubleshooting guide
- **[DEBUG-SMP-TRIPLE-FAULT.md](troubleshooting/DEBUG-SMP-TRIPLE-FAULT.md)**: SMP triple fault debugging
- **[smp-boot-debug.md](troubleshooting/smp-boot-debug.md)**: SMP boot process debugging
- **[smp-safety.md](troubleshooting/smp-safety.md)**: SMP safety and synchronization
- **[smp-triple-fault-fix.md](troubleshooting/smp-triple-fault-fix.md)**: Triple fault fixes

## 📖 Quick Start Reading Order

For new developers, we recommend reading in this order:

### 1. Understanding the System
1. [System Architecture](architecture/architecture.md) - Start here for overall understanding
2. [Task Scheduler](architecture/task-scheduler.md) - Core scheduling concepts
3. [Memory Management](architecture/memory-management-logging.md) - Memory subsystem

### 2. Multi-Core Development
1. [SMP Implementation](architecture/smp.md) - Multi-core architecture
2. [SMP Safety](troubleshooting/smp-safety.md) - Synchronization best practices

### 3. Development & Testing
1. [API Guide](development/api-guide.md) - How to use kernel APIs
2. [Testing Guide](development/testing.md) - Testing procedures
3. [Troubleshooting](troubleshooting/troubleshooting.md) - Common issues

## 🎯 Documentation by Topic

### Memory Management
- [Architecture Overview](architecture/architecture.md#memory-management-architecture)
- [Detailed Implementation](architecture/memory-management-logging.md)

### Task Scheduling
- [Architecture Overview](architecture/architecture.md#task-scheduler-architecture)
- [Detailed Implementation](architecture/task-scheduler.md)

### Multi-Core (SMP)
- [SMP Architecture](architecture/architecture.md#smp-symmetric-multi-processing-architecture)
- [Complete SMP Guide](architecture/smp.md)
- [SMP Safety Guidelines](troubleshooting/smp-safety.md)

### System Calls & IPC
- [Architecture Overview](architecture/architecture.md#system-call-interface)
- [API Usage Guide](development/api-guide.md)

### Testing & Debugging
- [Testing Procedures](development/testing.md)
- [General Troubleshooting](troubleshooting/troubleshooting.md)
- [SMP-Specific Debugging](troubleshooting/smp-boot-debug.md)

## 🔧 Contributing to Documentation

When adding new documentation:

1. **Choose the right category**:
   - `architecture/` - System design and implementation
   - `development/` - Developer guides and APIs
   - `troubleshooting/` - Debugging and issue resolution

2. **Follow naming conventions**:
   - Use kebab-case for filenames
   - Include `.md` extension
   - Use descriptive names

3. **Update this README**:
   - Add your document to the appropriate section
   - Include a brief description
   - Update the quick start guide if needed

4. **Cross-reference related docs**:
   - Link to related documentation
   - Update existing docs with links to new content

## 📋 Documentation Standards

- Use Markdown format (`.md`)
- Include code examples where applicable
- Add diagrams using ASCII art or Mermaid
- Keep sections focused and well-organized
- Include table of contents for long documents
- Use consistent heading styles
- Add cross-references to related documentation

## 🔍 Finding Information

Use these strategies to find what you need:

1. **Start with this README** - Overview of all documentation
2. **Check architecture docs** - For system design questions
3. **Look in development docs** - For API and development questions
4. **Search troubleshooting docs** - For debugging issues
5. **Use grep/search** - Search across all documentation files

Example searches:
```bash
# Find all mentions of SMP
grep -r "SMP" docs/

# Find scheduler-related documentation
grep -r "scheduler" docs/

# Find API examples
grep -r "API" docs/development/
```