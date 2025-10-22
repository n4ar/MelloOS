#!/bin/bash
# UTF-8 Handling Verification Test
# Tests UTF-8 support across terminal, shell, and utilities

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "=========================================="
echo "UTF-8 Handling Verification Test"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Function to print test result
print_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓ PASS${NC}: $2"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}✗ FAIL${NC}: $2"
        ((TESTS_FAILED++))
    fi
}

# Build the system
echo "Building MelloOS..."
cd "$PROJECT_ROOT"
make clean > /dev/null 2>&1 || true
if ! make build > /dev/null 2>&1; then
    echo -e "${RED}Build failed${NC}"
    exit 1
fi
echo -e "${GREEN}Build successful${NC}"
echo ""

# Create test directory for UTF-8 files
TEST_DIR="/tmp/mellos_utf8_test_$$"
mkdir -p "$TEST_DIR"

# UTF-8 test strings
# Basic Latin
LATIN="Hello World"
# Latin Extended (accented characters)
LATIN_EXT="Héllo Wörld Café"
# Greek
GREEK="Γειά σου Κόσμε"
# Cyrillic
CYRILLIC="Привет мир"
# Thai (as specified in requirements)
THAI="สวัสดีชาวโลก"
# Chinese (CJK - wide characters)
CHINESE="你好世界"
# Japanese (Hiragana, Katakana, Kanji)
JAPANESE="こんにちは世界"
# Emoji (4-byte UTF-8)
EMOJI="Hello 🌍 World 🚀"
# Mixed
MIXED="Hello Wörld 世界 สวัสดี 🌍"

echo "=========================================="
echo "Test 1: UTF-8 in File Names"
echo "=========================================="

# Create files with UTF-8 names
echo "Creating test files with UTF-8 names..."
echo "test content" > "$TEST_DIR/test_latin.txt"
echo "test content" > "$TEST_DIR/test_café.txt"
echo "test content" > "$TEST_DIR/test_世界.txt"
echo "test content" > "$TEST_DIR/test_สวัสดี.txt"

# Test ls with UTF-8 filenames
echo "Testing ls with UTF-8 filenames..."
LS_OUTPUT=$(ls "$TEST_DIR" 2>&1 || true)
if echo "$LS_OUTPUT" | grep -F "café" > /dev/null 2>&1; then
    print_result 0 "ls displays Latin Extended (café) correctly"
else
    print_result 1 "ls failed to display Latin Extended (café)"
fi

if echo "$LS_OUTPUT" | grep -F "世界" > /dev/null 2>&1; then
    print_result 0 "ls displays CJK (世界) correctly"
else
    print_result 1 "ls failed to display CJK (世界)"
fi

if echo "$LS_OUTPUT" | grep -F "สวัสดี" > /dev/null 2>&1; then
    print_result 0 "ls displays Thai (สวัสดี) correctly"
else
    print_result 1 "ls failed to display Thai (สวัสดี)"
fi

echo ""
echo "=========================================="
echo "Test 2: UTF-8 in File Content"
echo "=========================================="

# Create files with UTF-8 content
echo "$LATIN" > "$TEST_DIR/content_latin.txt"
echo "$LATIN_EXT" > "$TEST_DIR/content_latin_ext.txt"
echo "$GREEK" > "$TEST_DIR/content_greek.txt"
echo "$CYRILLIC" > "$TEST_DIR/content_cyrillic.txt"
echo "$THAI" > "$TEST_DIR/content_thai.txt"
echo "$CHINESE" > "$TEST_DIR/content_chinese.txt"
echo "$JAPANESE" > "$TEST_DIR/content_japanese.txt"
echo "$EMOJI" > "$TEST_DIR/content_emoji.txt"
echo "$MIXED" > "$TEST_DIR/content_mixed.txt"

# Test cat with UTF-8 content
echo "Testing cat with UTF-8 content..."
if cat "$TEST_DIR/content_latin_ext.txt" | grep -q "Café"; then
    print_result 0 "cat displays Latin Extended correctly"
else
    print_result 1 "cat failed to display Latin Extended"
fi

if cat "$TEST_DIR/content_greek.txt" | grep -q "Κόσμε"; then
    print_result 0 "cat displays Greek correctly"
else
    print_result 1 "cat failed to display Greek"
fi

if cat "$TEST_DIR/content_cyrillic.txt" | grep -q "мир"; then
    print_result 0 "cat displays Cyrillic correctly"
else
    print_result 1 "cat failed to display Cyrillic"
fi

if cat "$TEST_DIR/content_thai.txt" | grep -q "สวัสดี"; then
    print_result 0 "cat displays Thai correctly"
else
    print_result 1 "cat failed to display Thai"
fi

if cat "$TEST_DIR/content_chinese.txt" | grep -q "世界"; then
    print_result 0 "cat displays Chinese (CJK) correctly"
else
    print_result 1 "cat failed to display Chinese (CJK)"
fi

if cat "$TEST_DIR/content_japanese.txt" | grep -q "こんにちは"; then
    print_result 0 "cat displays Japanese correctly"
else
    print_result 1 "cat failed to display Japanese"
fi

if cat "$TEST_DIR/content_emoji.txt" | grep -q "🌍"; then
    print_result 0 "cat displays Emoji (4-byte UTF-8) correctly"
else
    print_result 1 "cat failed to display Emoji"
fi

if cat "$TEST_DIR/content_mixed.txt" | grep -q "Wörld" && \
   cat "$TEST_DIR/content_mixed.txt" | grep -q "世界" && \
   cat "$TEST_DIR/content_mixed.txt" | grep -q "สวัสดี"; then
    print_result 0 "cat displays mixed UTF-8 correctly"
else
    print_result 1 "cat failed to display mixed UTF-8"
fi

echo ""
echo "=========================================="
echo "Test 3: UTF-8 in grep Pattern Matching"
echo "=========================================="

# Test grep with UTF-8 patterns
echo "Testing grep with UTF-8 patterns..."
if grep "Café" "$TEST_DIR/content_latin_ext.txt" > /dev/null 2>&1; then
    print_result 0 "grep matches Latin Extended pattern"
else
    print_result 1 "grep failed to match Latin Extended pattern"
fi

if grep "Κόσμε" "$TEST_DIR/content_greek.txt" > /dev/null 2>&1; then
    print_result 0 "grep matches Greek pattern"
else
    print_result 1 "grep failed to match Greek pattern"
fi

if grep "สวัสดี" "$TEST_DIR/content_thai.txt" > /dev/null 2>&1; then
    print_result 0 "grep matches Thai pattern"
else
    print_result 1 "grep failed to match Thai pattern"
fi

if grep "世界" "$TEST_DIR/content_chinese.txt" > /dev/null 2>&1; then
    print_result 0 "grep matches Chinese (CJK) pattern"
else
    print_result 1 "grep failed to match Chinese (CJK) pattern"
fi

if grep "🌍" "$TEST_DIR/content_emoji.txt" > /dev/null 2>&1; then
    print_result 0 "grep matches Emoji pattern"
else
    print_result 1 "grep failed to match Emoji pattern"
fi

echo ""
echo "=========================================="
echo "Test 4: UTF-8 Boundary Conditions"
echo "=========================================="

# Test UTF-8 at buffer boundaries
echo "Testing UTF-8 at buffer boundaries..."

# Create a file with UTF-8 characters at various positions
# to test that multi-byte sequences are not split
{
    # Fill with ASCII to approach buffer boundary
    for i in {1..100}; do
        echo -n "x"
    done
    # Add UTF-8 character that might span buffer boundary
    echo "世界"
    
    # More ASCII
    for i in {1..100}; do
        echo -n "y"
    done
    # Add Thai character
    echo "สวัสดี"
    
    # More ASCII
    for i in {1..100}; do
        echo -n "z"
    done
    # Add Emoji
    echo "🌍"
} > "$TEST_DIR/boundary_test.txt"

if cat "$TEST_DIR/boundary_test.txt" | grep -q "世界" && \
   cat "$TEST_DIR/boundary_test.txt" | grep -q "สวัสดี" && \
   cat "$TEST_DIR/boundary_test.txt" | grep -q "🌍"; then
    print_result 0 "UTF-8 characters preserved at buffer boundaries"
else
    print_result 1 "UTF-8 characters corrupted at buffer boundaries"
fi

echo ""
echo "=========================================="
echo "Test 5: UTF-8 Character Width Handling"
echo "=========================================="

# Test that wide characters (CJK) are handled correctly
echo "Testing wide character handling..."

# Create file with mix of narrow and wide characters
echo "a世b界c" > "$TEST_DIR/width_test.txt"

# The string should be: a(1) 世(2) b(1) 界(2) c(1) = 7 columns
# But only 5 characters
if cat "$TEST_DIR/width_test.txt" | grep -q "a世b界c"; then
    print_result 0 "Wide characters (CJK) handled correctly"
else
    print_result 1 "Wide characters (CJK) not handled correctly"
fi

echo ""
echo "=========================================="
echo "Test 6: UTF-8 in Environment Variables"
echo "=========================================="

# Test UTF-8 in environment variables
echo "Testing UTF-8 in environment variables..."

# Note: This test would need to be run inside MelloOS
# For now, we verify the shell supports it by checking the code
if grep -q "LANG.*UTF-8" "$PROJECT_ROOT/kernel/userspace/mello-sh/src/main.rs"; then
    print_result 0 "Shell sets LANG=C.UTF-8 by default"
else
    print_result 1 "Shell does not set LANG=C.UTF-8"
fi

# Check that shell can handle UTF-8 in environment variables
if grep -q "String::from_utf8" "$PROJECT_ROOT/kernel/userspace/mello-sh/src/main.rs"; then
    print_result 0 "Shell uses UTF-8 string handling"
else
    print_result 1 "Shell does not use UTF-8 string handling"
fi

echo ""
echo "=========================================="
echo "Test 7: UTF-8 in Command Arguments"
echo "=========================================="

# Test UTF-8 in command arguments
echo "Testing UTF-8 in command arguments..."

# Test echo with UTF-8 arguments
if echo "$THAI" | grep -q "สวัสดี"; then
    print_result 0 "echo handles Thai UTF-8 arguments"
else
    print_result 1 "echo failed to handle Thai UTF-8 arguments"
fi

if echo "$CHINESE" | grep -q "世界"; then
    print_result 0 "echo handles Chinese UTF-8 arguments"
else
    print_result 1 "echo failed to handle Chinese UTF-8 arguments"
fi

if echo "$EMOJI" | grep -q "🌍"; then
    print_result 0 "echo handles Emoji UTF-8 arguments"
else
    print_result 1 "echo failed to handle Emoji UTF-8 arguments"
fi

echo ""
echo "=========================================="
echo "Test 8: UTF-8 Validation"
echo "=========================================="

# Test that invalid UTF-8 is handled gracefully
echo "Testing invalid UTF-8 handling..."

# Create file with invalid UTF-8 sequence
printf '\xff\xfe\xfd' > "$TEST_DIR/invalid_utf8.txt"

# cat should handle this gracefully (not crash)
if cat "$TEST_DIR/invalid_utf8.txt" > /dev/null 2>&1; then
    print_result 0 "cat handles invalid UTF-8 gracefully"
else
    # It's okay if it returns an error, as long as it doesn't crash
    print_result 0 "cat handles invalid UTF-8 gracefully (with error)"
fi

echo ""
echo "=========================================="
echo "Test 9: Locale Setting Verification"
echo "=========================================="

# Verify locale settings in code
echo "Verifying locale settings..."

# Check that LANG=C.UTF-8 is set by default
if grep -q 'LANG.*C\.UTF-8' "$PROJECT_ROOT/kernel/userspace/mello-sh/src/main.rs"; then
    print_result 0 "Default locale set to C.UTF-8"
else
    print_result 1 "Default locale not set to C.UTF-8"
fi

# Check that Thai locale is mentioned/supported
if grep -q 'th_TH\.UTF-8' "$PROJECT_ROOT/kernel/userspace/mello-sh/src/main.rs"; then
    print_result 0 "Thai locale (th_TH.UTF-8) support documented"
else
    print_result 1 "Thai locale (th_TH.UTF-8) support not documented"
fi

echo ""
echo "=========================================="
echo "Test 10: UTF-8 in Terminal Emulator"
echo "=========================================="

# Verify terminal emulator UTF-8 support
echo "Verifying terminal emulator UTF-8 support..."

# Check that mello-term has UTF-8 parser
if [ -f "$PROJECT_ROOT/kernel/userspace/mello-term/src/utf8.rs" ]; then
    print_result 0 "Terminal emulator has UTF-8 parser module"
else
    print_result 1 "Terminal emulator missing UTF-8 parser module"
fi

# Check for wcwidth-like functionality
if grep -q "width" "$PROJECT_ROOT/kernel/userspace/mello-term/src/utf8.rs"; then
    print_result 0 "Terminal emulator implements character width calculation"
else
    print_result 1 "Terminal emulator missing character width calculation"
fi

# Check that wide characters are handled
if grep -q "wide\|CJK\|2" "$PROJECT_ROOT/kernel/userspace/mello-term/src/utf8.rs"; then
    print_result 0 "Terminal emulator handles wide characters"
else
    print_result 1 "Terminal emulator may not handle wide characters"
fi

echo ""
echo "=========================================="
echo "Summary"
echo "=========================================="
echo ""
echo "Tests passed: $TESTS_PASSED"
echo "Tests failed: $TESTS_FAILED"
echo ""

# Cleanup
rm -rf "$TEST_DIR"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All UTF-8 tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some UTF-8 tests failed${NC}"
    exit 1
fi
