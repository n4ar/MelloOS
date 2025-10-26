#!/usr/bin/env bash
# Categorize and prioritize TODOs in MelloOS
#
# This script categorizes TODOs by:
# - Module/subsystem
# - Priority (based on keywords)
# - Type (implementation, optimization, documentation, etc.)
#
# Usage: ./tools/categorize_todos.sh

set -e

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

OUTPUT_FILE="TECHNICAL_DEBT_CATEGORIZED.md"

echo "Categorizing TODOs in MelloOS..."
echo "Output: $OUTPUT_FILE"
echo ""

# Clear output file
> "$OUTPUT_FILE"

cat >> "$OUTPUT_FILE" << 'EOF'
# MelloOS Technical Debt - Categorized

Generated: $(date)

## Summary

This document categorizes all TODO, FIXME, and HACK markers found in the MelloOS codebase.

---

EOF

# Count totals
TOTAL_TODO=$(grep -r -i --include="*.rs" --include="*.md" --include="*.toml" \
    --exclude-dir=target --exclude-dir=.git --exclude-dir=build \
    '\(//\|#\|/\*\|\*\).*\bTODO\b' \
    kernel/ tests/ 2>/dev/null | wc -l | tr -d ' ')

TOTAL_FIXME=$(grep -r -i --include="*.rs" --include="*.md" --include="*.toml" \
    --exclude-dir=target --exclude-dir=.git --exclude-dir=build \
    '\(//\|#\|/\*\|\*\).*\bFIXME\b' \
    kernel/ tests/ 2>/dev/null | wc -l | tr -d ' ')

TOTAL_HACK=$(grep -r -i --include="*.rs" --include="*.md" --include="*.toml" \
    --exclude-dir=target --exclude-dir=.git --exclude-dir=build \
    '\(//\|#\|/\*\|\*\).*\bHACK\b' \
    kernel/ tests/ 2>/dev/null | wc -l | tr -d ' ')

cat >> "$OUTPUT_FILE" << EOF
## Totals

- **TODO**: $TOTAL_TODO
- **FIXME**: $TOTAL_FIXME  
- **HACK**: $TOTAL_HACK
- **TOTAL**: $((TOTAL_TODO + TOTAL_FIXME + TOTAL_HACK))

---

EOF

# Categorize by subsystem
echo "Categorizing by subsystem..."

cat >> "$OUTPUT_FILE" << 'EOF'
## By Subsystem

EOF

# Memory Management
MM_COUNT=$(grep -r -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/mm/ 2>/dev/null | wc -l | tr -d ' ')

cat >> "$OUTPUT_FILE" << EOF
### Memory Management (kernel/src/mm/) - $MM_COUNT items

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/mm/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# Filesystem
FS_COUNT=$(grep -r -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/fs/ 2>/dev/null | wc -l | tr -d ' ')

cat >> "$OUTPUT_FILE" << EOF
### Filesystem (kernel/src/fs/) - $FS_COUNT items

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/fs/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# User/Process Management
USER_COUNT=$(grep -r -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/user/ 2>/dev/null | wc -l | tr -d ' ')

cat >> "$OUTPUT_FILE" << EOF
### User/Process Management (kernel/src/user/) - $USER_COUNT items

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/user/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# Syscalls
SYSCALL_COUNT=$(grep -r -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/arch/x86_64/syscall/ kernel/src/sys/ 2>/dev/null | wc -l | tr -d ' ')

cat >> "$OUTPUT_FILE" << EOF
### Syscalls (kernel/src/sys/ & kernel/src/arch/x86_64/syscall/) - $SYSCALL_COUNT items

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/arch/x86_64/syscall/ kernel/src/sys/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# Drivers
DRIVER_COUNT=$(grep -r -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/drivers/ 2>/dev/null | wc -l | tr -d ' ')

cat >> "$OUTPUT_FILE" << EOF
### Drivers (kernel/src/drivers/) - $DRIVER_COUNT items

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/drivers/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# Signals
SIGNAL_COUNT=$(grep -r -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/signal/ 2>/dev/null | wc -l | tr -d ' ')

cat >> "$OUTPUT_FILE" << EOF
### Signals (kernel/src/signal/) - $SIGNAL_COUNT items

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/signal/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# Devices (PTY, etc.)
DEV_COUNT=$(grep -r -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/dev/ 2>/dev/null | wc -l | tr -d ' ')

cat >> "$OUTPUT_FILE" << EOF
### Devices (kernel/src/dev/) - $DEV_COUNT items

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/src/dev/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# Userspace programs
USERSPACE_COUNT=$(grep -r -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/userspace/ 2>/dev/null | wc -l | tr -d ' ')

cat >> "$OUTPUT_FILE" << EOF
### Userspace Programs (kernel/userspace/) - $USERSPACE_COUNT items

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    kernel/userspace/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# Tests
TEST_COUNT=$(grep -r -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    tests/ 2>/dev/null | wc -l | tr -d ' ')

cat >> "$OUTPUT_FILE" << EOF
### Tests (tests/) - $TEST_COUNT items

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b' \
    tests/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# Priority categorization
cat >> "$OUTPUT_FILE" << 'EOF'
---

## By Priority

### 🔴 High Priority (Security, Correctness, Core Functionality)

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b.*\(security\|unsafe\|panic\|crash\|bug\|critical\|important\)' \
    kernel/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None found" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

cat >> "$OUTPUT_FILE" << 'EOF'
### 🟡 Medium Priority (Features, Optimizations)

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b.*\(implement\|optimization\|performance\|feature\)' \
    kernel/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None found" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

cat >> "$OUTPUT_FILE" << 'EOF'
### 🟢 Low Priority (Documentation, Logging, Nice-to-have)

EOF

grep -rn -i --include="*.rs" \
    --exclude-dir=target --exclude-dir=.git \
    '\(//\|#\|/\*\|\*\).*\b\(TODO\|FIXME\|HACK\)\b.*\(log\|document\|comment\|cleanup\)' \
    kernel/ 2>/dev/null | \
    sed 's/^/- /' >> "$OUTPUT_FILE" || echo "None found" >> "$OUTPUT_FILE"

echo "" >> "$OUTPUT_FILE"

# Recommendations
cat >> "$OUTPUT_FILE" << 'EOF'
---

## Recommendations

### Immediate Actions (High Priority)
1. Review all security-related TODOs
2. Fix any unsafe code without proper documentation
3. Implement critical missing functionality

### Short-term (Medium Priority)
1. Complete core feature implementations
2. Add missing error handling
3. Implement performance optimizations

### Long-term (Low Priority)
1. Improve documentation
2. Add comprehensive logging
3. Code cleanup and refactoring

### Process Improvements
1. Create GitHub issues for each TODO
2. Add TODOs to .kiro/specs/*/tasks.md
3. Set up automated TODO tracking in CI
4. Regular technical debt review sessions

---

## Next Steps

1. Review this categorized list
2. Prioritize items based on current phase (see roadmap.md)
3. Create tasks in appropriate spec files
4. Assign owners and deadlines
5. Track progress and remove completed TODOs

EOF

echo ""
echo -e "${GREEN}✓ Categorization complete!${NC}"
echo -e "${CYAN}Report saved to: $OUTPUT_FILE${NC}"
echo ""
echo -e "${YELLOW}Summary:${NC}"
echo -e "  Memory Management: $MM_COUNT"
echo -e "  Filesystem: $FS_COUNT"
echo -e "  User/Process: $USER_COUNT"
echo -e "  Syscalls: $SYSCALL_COUNT"
echo -e "  Drivers: $DRIVER_COUNT"
echo -e "  Signals: $SIGNAL_COUNT"
echo -e "  Devices: $DEV_COUNT"
echo -e "  Userspace: $USERSPACE_COUNT"
echo -e "  Tests: $TEST_COUNT"
echo ""
echo -e "${RED}Total: $((TOTAL_TODO + TOTAL_FIXME + TOTAL_HACK)) items${NC}"
echo ""

exit 0
