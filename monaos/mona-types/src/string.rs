use crate::ascii;
use std::vec::Vec;

/// Error type for String operations
#[derive(Debug, PartialEq)]
pub enum StringError {
    InvalidUtf8,
    InvalidIndex,
}

/// A String type that guarantees valid UTF-8 encoding
#[derive(Debug, Clone)]
pub struct String {
    bytes: Vec<u8>,
}

impl String {
    /// Creates a new string from bytes. Returns error if not valid UTF-8
    pub fn utf8(bytes: Vec<u8>) -> Result<Self, StringError> {
        if std::str::from_utf8(&bytes).is_ok() {
            Ok(String { bytes })
        } else {
            Err(StringError::InvalidUtf8)
        }
    }

    /// Convert ASCII string to UTF8 string
    pub fn from_ascii(s: ascii::String) -> Self {
        String { bytes: s.into_bytes() }
    }

    /// Convert to ASCII string if possible
    pub fn to_ascii(&self) -> Result<ascii::String, ascii::AsciiError> {
        ascii::String::new(self.bytes.clone())
    }

    /// Try to create string from bytes
    pub fn try_utf8(bytes: Vec<u8>) -> Option<Self> {
        Self::utf8(bytes).ok()
    }

    /// Get reference to underlying bytes
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Check if string is empty
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Get length in bytes
    pub fn length(&self) -> usize {
        self.bytes.len()
    }

    /// Append another string
    pub fn append(&mut self, other: String) {
        self.bytes.extend(other.bytes);
    }

    /// Append UTF-8 bytes
    pub fn append_utf8(&mut self, bytes: Vec<u8>) -> Result<(), StringError> {
        let other = Self::utf8(bytes)?;
        self.append(other);
        Ok(())
    }

    /// Insert string at byte index
    pub fn insert(&mut self, at: usize, other: String) -> Result<(), StringError> {
        if !self.is_char_boundary(at) {
            return Err(StringError::InvalidIndex);
        }
        self.bytes.splice(at..at, other.bytes);
        Ok(())
    }

    /// Get substring
    pub fn sub_string(&self, i: usize, j: usize) -> Result<String, StringError> {
        if j > self.length() || i > j || !self.is_char_boundary(i) || !self.is_char_boundary(j) {
            return Err(StringError::InvalidIndex);
        }
        Ok(String { bytes: self.bytes[i..j].to_vec() })
    }

    /// Find first occurrence of substring
    pub fn index_of(&self, other: &String) -> usize {
        std::str::from_utf8(&self.bytes)
            .unwrap()
            .find(std::str::from_utf8(&other.bytes).unwrap())
            .unwrap_or(self.length())
    }

    /// Check if byte index is at character boundary
    fn is_char_boundary(&self, index: usize) -> bool {
        std::str::from_utf8(&self.bytes)
            .unwrap()
            .is_char_boundary(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utf8_creation() {
        let s = String::utf8(b"Hello".to_vec()).unwrap();
        assert_eq!(s.length(), 5);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_invalid_utf8() {
        let invalid = vec![0xFF, 0xFF];
        assert!(String::utf8(invalid).is_err());
    }

    #[test]
    fn test_substring() {
        let s = String::utf8("Hello World".as_bytes().to_vec()).unwrap();
        let sub = s.sub_string(0, 5).unwrap();
        assert_eq!(sub.bytes, b"Hello");
    }

    #[test]
    fn test_append() {
        let mut s = String::utf8(b"Hello".to_vec()).unwrap();
        let other = String::utf8(b" World".to_vec()).unwrap();
        s.append(other);
        assert_eq!(s.bytes, b"Hello World");
    }

    #[test]
    fn test_index_of() {
        let s = String::utf8(b"Hello World".to_vec()).unwrap();
        let sub = String::utf8(b"World".to_vec()).unwrap();
        assert_eq!(s.index_of(&sub), 6);
    }
}