use std::vec::Vec;

/// Error codes for BitVector operations
#[derive(Debug)]
pub enum BitVectorError {
    IndexOutOfBounds,
    InvalidLength,
}

/// Maximum allowed bitvector size
const MAX_SIZE: usize = 1024;

/// A vector of bits with fixed length
#[derive(Clone, Debug)]
pub struct BitVector {
    length: usize,
    bit_field: Vec<bool>,
}

impl BitVector {
    /// Create a new BitVector of specified length
    pub fn new(length: usize) -> Result<Self, BitVectorError> {
        if length == 0 || length >= MAX_SIZE {
            return Err(BitVectorError::InvalidLength);
        }
        
        Ok(BitVector {
            length,
            bit_field: vec![false; length],
        })
    }

    /// Set the bit at the given index
    pub fn set(&mut self, bit_index: usize) -> Result<(), BitVectorError> {
        if bit_index >= self.bit_field.len() {
            return Err(BitVectorError::IndexOutOfBounds);
        }
        self.bit_field[bit_index] = true;
        Ok(())
    }

    /// Unset the bit at the given index
    pub fn unset(&mut self, bit_index: usize) -> Result<(), BitVectorError> {
        if bit_index >= self.bit_field.len() {
            return Err(BitVectorError::IndexOutOfBounds);
        }
        self.bit_field[bit_index] = false;
        Ok(())
    }

    /// Shift the bitvector left by the specified amount
    pub fn shift_left(&mut self, amount: usize) {
        if amount >= self.length {
            self.bit_field.fill(false);
        } else {
            for i in amount..self.length {
                let value = self.bit_field[i];
                self.bit_field[i - amount] = value;
            }
            for i in (self.length - amount)..self.length {
                self.bit_field[i] = false;
            }
        }
    }

    /// Check if the bit at the given index is set
    pub fn is_index_set(&self, bit_index: usize) -> Result<bool, BitVectorError> {
        if bit_index >= self.bit_field.len() {
            return Err(BitVectorError::IndexOutOfBounds);
        }
        Ok(self.bit_field[bit_index])
    }

    /// Get the length of the bitvector
    pub fn length(&self) -> usize {
        self.bit_field.len()
    }

    /// Get the length of the longest sequence of set bits starting at the given index
    pub fn longest_set_sequence_starting_at(&self, start_index: usize) -> Result<usize, BitVectorError> {
        if start_index >= self.length {
            return Err(BitVectorError::IndexOutOfBounds);
        }

        let mut index = start_index;
        while index < self.length {
            if !self.bit_field[index] {
                break;
            }
            index += 1;
        }

        Ok(index - start_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_bitvector() {
        let bv = BitVector::new(10).unwrap();
        assert_eq!(bv.length(), 10);
        assert!(bv.is_index_set(0).unwrap() == false);
    }

    #[test]
    fn test_set_and_unset() {
        let mut bv = BitVector::new(10).unwrap();
        bv.set(5).unwrap();
        assert!(bv.is_index_set(5).unwrap());
        bv.unset(5).unwrap();
        assert!(!bv.is_index_set(5).unwrap());
    }

    #[test]
    fn test_shift_left() {
        let mut bv = BitVector::new(10).unwrap();
        bv.set(5).unwrap();
        bv.shift_left(2);
        assert!(bv.is_index_set(3).unwrap());
        assert!(!bv.is_index_set(5).unwrap());
    }

    #[test]
    fn test_longest_sequence() {
        let mut bv = BitVector::new(10).unwrap();
        bv.set(1).unwrap();
        bv.set(2).unwrap();
        bv.set(3).unwrap();
        assert_eq!(bv.longest_set_sequence_starting_at(1).unwrap(), 3);
    }
}