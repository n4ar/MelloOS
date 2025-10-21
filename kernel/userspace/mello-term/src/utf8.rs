//! UTF-8 parsing and character width calculation
//!
//! Handles multi-byte UTF-8 sequences and determines character display width.

/// UTF-8 parser state
pub struct Utf8Parser {
    /// Bytes accumulated so far
    bytes: [u8; 4],
    /// Number of bytes accumulated
    len: usize,
    /// Expected total bytes for current character
    expected: usize,
}

impl Utf8Parser {
    /// Create a new UTF-8 parser
    pub const fn new() -> Self {
        Self {
            bytes: [0; 4],
            len: 0,
            expected: 0,
        }
    }

    /// Parse a byte and return a character if complete
    pub fn parse(&mut self, byte: u8) -> Option<char> {
        if self.len == 0 {
            // Start of new character
            if byte & 0x80 == 0 {
                // Single-byte ASCII character
                return Some(byte as char);
            } else if byte & 0xE0 == 0xC0 {
                // 2-byte sequence
                self.expected = 2;
            } else if byte & 0xF0 == 0xE0 {
                // 3-byte sequence
                self.expected = 3;
            } else if byte & 0xF8 == 0xF0 {
                // 4-byte sequence
                self.expected = 4;
            } else {
                // Invalid start byte
                return None;
            }
            
            self.bytes[0] = byte;
            self.len = 1;
            None
        } else {
            // Continuation byte
            if byte & 0xC0 != 0x80 {
                // Invalid continuation byte - reset
                self.len = 0;
                self.expected = 0;
                return None;
            }
            
            self.bytes[self.len] = byte;
            self.len += 1;
            
            if self.len == self.expected {
                // Complete character
                let ch = match self.expected {
                    2 => {
                        let code = ((self.bytes[0] & 0x1F) as u32) << 6
                                 | ((self.bytes[1] & 0x3F) as u32);
                        char::from_u32(code)
                    }
                    3 => {
                        let code = ((self.bytes[0] & 0x0F) as u32) << 12
                                 | ((self.bytes[1] & 0x3F) as u32) << 6
                                 | ((self.bytes[2] & 0x3F) as u32);
                        char::from_u32(code)
                    }
                    4 => {
                        let code = ((self.bytes[0] & 0x07) as u32) << 18
                                 | ((self.bytes[1] & 0x3F) as u32) << 12
                                 | ((self.bytes[2] & 0x3F) as u32) << 6
                                 | ((self.bytes[3] & 0x3F) as u32);
                        char::from_u32(code)
                    }
                    _ => None,
                };
                
                // Reset for next character
                self.len = 0;
                self.expected = 0;
                
                ch
            } else {
                // Need more bytes
                None
            }
        }
    }

    /// Reset parser state
    pub fn reset(&mut self) {
        self.len = 0;
        self.expected = 0;
    }
}

/// Get the display width of a character (0, 1, or 2 columns)
///
/// This is a simplified wcwidth implementation that handles:
/// - Control characters: width 0
/// - ASCII printable: width 1
/// - CJK characters: width 2
/// - Other Unicode: width 1
pub fn char_width(ch: char) -> usize {
    let code = ch as u32;
    
    // Control characters have width 0
    if code < 0x20 || (code >= 0x7F && code < 0xA0) {
        return 0;
    }
    
    // CJK Unified Ideographs (width 2)
    if (0x4E00..=0x9FFF).contains(&code) {
        return 2;
    }
    
    // CJK Compatibility Ideographs (width 2)
    if (0xF900..=0xFAFF).contains(&code) {
        return 2;
    }
    
    // Hangul Syllables (width 2)
    if (0xAC00..=0xD7AF).contains(&code) {
        return 2;
    }
    
    // Hiragana and Katakana (width 2)
    if (0x3040..=0x30FF).contains(&code) {
        return 2;
    }
    
    // Fullwidth forms (width 2)
    if (0xFF00..=0xFFEF).contains(&code) {
        return 2;
    }
    
    // Combining characters (width 0)
    if (0x0300..=0x036F).contains(&code) {
        return 0;
    }
    
    // Default to width 1
    1
}

/// Check if a character is a combining character
pub fn is_combining(ch: char) -> bool {
    let code = ch as u32;
    
    // Combining Diacritical Marks
    if (0x0300..=0x036F).contains(&code) {
        return true;
    }
    
    // Combining Diacritical Marks Extended
    if (0x1AB0..=0x1AFF).contains(&code) {
        return true;
    }
    
    // Combining Diacritical Marks Supplement
    if (0x1DC0..=0x1DFF).contains(&code) {
        return true;
    }
    
    false
}
