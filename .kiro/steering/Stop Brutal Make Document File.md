---
inclusion: always
---

# Documentation Policy - MANDATORY

## Core Principle: Minimize Unnecessary Documentation

**DO NOT create documentation files unless explicitly requested by the user.**

### What This Means

❌ **NEVER create these without explicit request:**
- Summary markdown files after completing work
- "COMPLETION_SUMMARY.md" or similar recap documents
- "IMPLEMENTATION_NOTES.md" for routine changes
- Redundant documentation that duplicates existing docs
- Progress reports or status documents
- Change logs for minor updates

✅ **DO create/update documentation when:**
- User explicitly asks for documentation
- Adding new architecture or subsystem (update `docs/architecture/`)
- Implementing new features that need user-facing docs (update `docs/USER_GUIDE.md`)
- Fixing complex bugs that should be documented (update `docs/troubleshooting/`)
- Completing major phases (update `docs/` with architectural changes)
- Creating specs for new features (`.kiro/specs/`)

### Existing Documentation Structure

The project already has comprehensive documentation:

**Architecture:** `docs/architecture/` - System design and component documentation
**User Guides:** `docs/USER_GUIDE.md`, `docs/DEVELOPER_GUIDE.md`
**Troubleshooting:** `docs/troubleshooting/` - Known issues and solutions
**Specs:** `.kiro/specs/` - Feature requirements, design, and tasks
**Testing:** `tools/testing/` - Test documentation and results

### When Work is Complete

**Instead of creating a summary document:**

1. **Update existing documentation** if architectural changes were made
2. **Update spec tasks** (mark complete in `tasks.md`)
3. **Provide brief verbal summary** in response (2-3 sentences max)
4. **Update roadmap** if phase milestone reached

### Response Pattern After Completing Work

❌ **Bad (creates unnecessary file):**
```
I've completed the work. Let me create IMPLEMENTATION_SUMMARY.md 
to document what was done...
```

✅ **Good (concise verbal summary):**
```
Completed PTY implementation with signal handling and job control. 
All tests passing. Updated architecture docs with PTY subsystem details.
```

### Exception: Explicitly Requested Documentation

If user says:
- "Document this implementation"
- "Create a summary of the changes"
- "Write up the architecture"
- "Add this to the troubleshooting guide"

Then create or update the appropriate documentation.

### Integration with Other Policies

This policy complements:
- **response_style** - "Do not create new markdown files to summarize your work"
- **no-assumptions.md** - Ask before creating documentation
- **best-practice.md** - Follow literate programming in code comments instead

### Code Documentation vs. File Documentation

**Prefer code documentation:**
- Use doc comments (`///`) for public APIs
- Add inline comments for complex logic
- Write clear commit messages
- Update existing architectural docs

**Avoid file documentation:**
- Don't create standalone summary files
- Don't duplicate information already in code
- Don't create "notes" files for routine work

## Summary

**Remember:** The best documentation is often no documentation at all. Keep information in code comments, existing docs, and concise verbal responses. Only create new documentation files when explicitly requested or when adding significant new subsystems that require architectural documentation.