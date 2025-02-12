use std::vec::Vec;

/// Error type for ASCII operations
#[derive(Debug, PartialEq)]
pub enum AsciiError {
    InvalidAsciiCharacter,
}

/// An ASCII string that ensures all characters are valid ASCII
#[derive(Clone, Debug, PartialEq)]
pub struct String {
    bytes: Vec<u8>,
}

/// A single ASCII character
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Char {
    byte: u8,
}

impl String {
    /// Create a new ASCII string from bytes. Returns error if any byte is invalid ASCII.
    pub fn new(bytes: Vec<u8>) -> Result<Self, AsciiError> {
        if bytes.iter().all(|&b| Self::is_valid_char(b)) {
            Ok(String { bytes })
        } else {
            Err(AsciiError::InvalidAsciiCharacter)
        }
    }

    /// Try to create an ASCII string, returning None if invalid
    pub fn try_new(bytes: Vec<u8>) -> Option<Self> {
        Self::new(bytes).ok()
    }

    /// Check if all characters are printable
    pub fn all_characters_printable(&self) -> bool {
        self.bytes.iter().all(|&b| Self::is_printable_char(b))
    }

    /// Add a character to the end of the string
    pub fn push_char(&mut self, char: Char) {
        self.bytes.push(char.byte);
    }

    /// Remove and return the last character
    pub fn pop_char(&mut self) -> Option<Char> {
        self.bytes.pop().map(|byte| Char { byte })
    }

    /// Get the length of the string
    pub fn length(&self) -> usize {
        self.bytes.len()
    }

    /// Get a reference to the underlying bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Convert into the underlying bytes
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns true if byte is valid ASCII
    pub fn is_valid_char(b: u8) -> bool {
        b <= 0x7F
    }

    /// Returns true if byte is printable ASCII
    pub fn is_printable_char(byte: u8) -> bool {
        byte >= 0x20 && byte <= 0x7E
    }
}

impl Char {
    /// Create a new ASCII character
    pub fn new(byte: u8) -> Result<Self, AsciiError> {
        if String::is_valid_char(byte) {
            Ok(Char { byte })
        } else {
            Err(AsciiError::InvalidAsciiCharacter)
        }
    }

    /// Get the underlying byte
    pub fn byte(self) -> u8 {
        self.byte
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_string() {
        let bytes = b"Hello World!".to_vec();
        let string = String::new(bytes).unwrap();
        assert_eq!(string.length(), 12);
        assert!(string.all_characters_printable());
    }

    #[test]
    fn test_invalid_string() {
        let bytes = vec![0x80];  // Invalid ASCII
        assert!(String::new(bytes).is_err());
    }

    #[test]
    fn test_char() {
        assert!(Char::new(b'A').is_ok());
        assert!(Char::new(0x80).is_err());
    }

    #[test]
    fn test_push_pop() {
        let mut string = String::new(b"Hello".to_vec()).unwrap();
        string.push_char(Char::new(b'!').unwrap());
        assert_eq!(string.length(), 6);
        
        let popped = string.pop_char().unwrap();
        assert_eq!(popped.byte(), b'!');
    }
}