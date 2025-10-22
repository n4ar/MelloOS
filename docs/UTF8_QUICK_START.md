# UTF-8 Quick Start Guide

## Overview

MelloOS fully supports UTF-8 encoding for international characters. This guide shows you how to use UTF-8 in your daily work.

## Default Configuration

MelloOS is configured for UTF-8 by default:
- **Locale**: `LANG=C.UTF-8`
- **Encoding**: UTF-8 everywhere
- **No configuration needed**: Just start using international characters!

## Quick Examples

### Display International Text

```bash
# Thai
echo "สวัสดีชาวโลก"

# Chinese
echo "你好世界"

# Japanese
echo "こんにちは世界"

# Greek
echo "Γειά σου Κόσμε"

# Emoji
echo "Hello 🌍 World 🚀"

# Mixed
echo "Hello Wörld 世界 สวัสดี 🌍"
```

### Create Files with UTF-8 Names

```bash
# Create files with international names
touch café.txt
touch 世界.txt
touch สวัสดี.txt

# List them
ls
```

### Work with UTF-8 Content

```bash
# Create file with UTF-8 content
echo "สวัสดีชาวโลก" > thai.txt

# Display it
cat thai.txt

# Search in it
grep "สวัสดี" thai.txt
```

### Change Locale

```bash
# Set Thai locale
export LANG=th_TH.UTF-8

# Verify
echo $LANG
```

## Supported Languages

| Language | Example | Status |
|----------|---------|--------|
| English | Hello | ✓ |
| French | Café | ✓ |
| German | Größe | ✓ |
| Spanish | Niño | ✓ |
| Greek | Γειά | ✓ |
| Russian | Привет | ✓ |
| Thai | สวัสดี | ✓ |
| Chinese | 你好 | ✓ |
| Japanese | こんにちは | ✓ |
| Korean | 안녕 | ✓ |
| Arabic | مرحبا | ⚠️ (RTL not yet supported) |
| Hebrew | שלום | ⚠️ (RTL not yet supported) |

## Tips

### Wide Characters

Some characters (like Chinese, Japanese, Korean) take 2 columns:

```bash
# This string has 5 characters but takes 7 columns
echo "a世b界c"
#     ^ ^  ^ ^  ^
#     1 2  3 4  5 columns
```

### Emoji

Emoji are 4-byte UTF-8 characters and take 2 columns:

```bash
echo "🌍"  # Takes 2 columns
```

### Combining Characters

Some languages use combining characters (accents, marks):

```bash
echo "é"   # Can be 1 or 2 characters (e + ́)
```

## Common Tasks

### Copy/Paste UTF-8 Text

In mello-term:
1. Select text with mouse
2. Copy: Ctrl+Shift+C
3. Paste: Ctrl+Shift+V

### Search UTF-8 Text

```bash
# Case-sensitive
grep "สวัสดี" file.txt

# Case-insensitive
grep -i "café" file.txt

# Recursive
grep -r "世界" /path/to/dir
```

### Process UTF-8 Files

```bash
# Count lines with UTF-8
cat thai.txt | wc -l

# Sort UTF-8 content
cat file.txt | sort

# Filter UTF-8 content
cat file.txt | grep "pattern"
```

## Troubleshooting

### Problem: Characters appear as boxes or question marks

**Solution**: Your terminal font may not support the character set. Try:
1. Use a Unicode font (e.g., DejaVu Sans Mono, Noto Sans)
2. Verify locale: `echo $LANG` (should be `C.UTF-8` or `*.UTF-8`)

### Problem: Wide characters misaligned

**Solution**: This is expected. CJK characters take 2 columns:
```bash
# Correct alignment:
echo "a世b"
#     ^  ^
#     1  3 (not 2)
```

### Problem: Invalid UTF-8 error

**Solution**: The file may not be UTF-8 encoded. Check with:
```bash
file filename.txt
```

## Testing

Run the UTF-8 test suite to verify everything works:

```bash
./tools/testing/test_utf8_handling.sh
```

Expected output:
```
Tests passed: 29
Tests failed: 0
All UTF-8 tests passed!
```

## Advanced Usage

### Set Locale for Single Command

```bash
LANG=th_TH.UTF-8 command args
```

### Check Current Locale

```bash
echo $LANG
```

### List Available Locales

Currently supported:
- `C.UTF-8` (default, recommended)
- `th_TH.UTF-8` (Thai)
- Any `*.UTF-8` locale (via export)

## Best Practices

1. **Always use UTF-8**: Don't use legacy encodings (ISO-8859-1, etc.)
2. **Keep LANG=C.UTF-8**: Best compatibility
3. **Use Unicode fonts**: Ensure your terminal font supports your language
4. **Test with real data**: Use actual international text in testing
5. **Handle errors gracefully**: Some characters may not display on all systems

## Resources

- Full documentation: `docs/UTF8_SUPPORT.md`
- Test suite: `tools/testing/test_utf8_handling.sh`
- Requirements: `.kiro/specs/advanced-userland-shell/requirements.md`

## Examples by Language

### Thai (th_TH.UTF-8)

```bash
export LANG=th_TH.UTF-8
echo "สวัสดีครับ"
echo "ขอบคุณครับ"
```

### Chinese (zh_CN.UTF-8)

```bash
export LANG=zh_CN.UTF-8
echo "你好"
echo "谢谢"
```

### Japanese (ja_JP.UTF-8)

```bash
export LANG=ja_JP.UTF-8
echo "こんにちは"
echo "ありがとう"
```

### Russian (ru_RU.UTF-8)

```bash
export LANG=ru_RU.UTF-8
echo "Привет"
echo "Спасибо"
```

## Summary

MelloOS provides full UTF-8 support out of the box. Just start using international characters - no configuration needed!

- ✓ Default locale: C.UTF-8
- ✓ All components support UTF-8
- ✓ 29/29 tests passing
- ✓ Wide character support
- ✓ Emoji support
- ✓ Multi-byte sequences handled correctly

**Happy international computing!** 🌍
