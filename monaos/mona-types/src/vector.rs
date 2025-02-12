#[derive(Debug, Clone)]
pub struct Vector<T> {
    elements: Vec<T>,
}

#[derive(Debug)]
pub enum VectorError {
    IndexOutOfBounds,
}

impl<T> Vector<T> {
    /// Create an empty vector
    pub fn empty() -> Self {
        Vector { elements: Vec::new() }
    }

    /// Get the length of the vector
    pub fn length(&self) -> usize {
        self.elements.len()
    }

    /// Borrow element at index
    pub fn borrow(&self, i: usize) -> Result<&T, VectorError> {
        self.elements.get(i).ok_or(VectorError::IndexOutOfBounds)
    }

    /// Push element to back of vector
    pub fn push_back(&mut self, e: T) {
        self.elements.push(e);
    }

    /// Borrow mutable element at index
    pub fn borrow_mut(&mut self, i: usize) -> Result<&mut T, VectorError> {
        self.elements.get_mut(i).ok_or(VectorError::IndexOutOfBounds)
    }

    /// Pop element from back of vector
    pub fn pop_back(&mut self) -> Result<T, VectorError> {
        self.elements.pop().ok_or(VectorError::IndexOutOfBounds)
    }

    /// Destroy empty vector
    pub fn destroy_empty(self) -> Result<(), VectorError> {
        if self.elements.is_empty() {
            Ok(())
        } else {
            Err(VectorError::IndexOutOfBounds)
        }
    }

    /// Swap elements at indices
    pub fn swap(&mut self, i: usize, j: usize) -> Result<(), VectorError> {
        if i >= self.length() || j >= self.length() {
            return Err(VectorError::IndexOutOfBounds);
        }
        self.elements.swap(i, j);
        Ok(())
    }

    /// Create singleton vector
    pub fn singleton(e: T) -> Self {
        let mut v = Self::empty();
        v.push_back(e);
        v
    }

    /// Reverse vector in place
    pub fn reverse(&mut self) {
        self.elements.reverse();
    }

    /// Append other vector
    pub fn append(&mut self, mut other: Vector<T>) {
        self.elements.append(&mut other.elements);
    }

    /// Check if vector is empty
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Check if vector contains element
    pub fn contains(&self, e: &T) -> bool 
    where T: PartialEq {
        self.elements.contains(e)
    }

    /// Get index of element
    pub fn index_of(&self, e: &T) -> (bool, usize) 
    where T: PartialEq {
        match self.elements.iter().position(|x| x == e) {
            Some(i) => (true, i),
            None => (false, 0),
        }
    }

    /// Remove element at index
    pub fn remove(&mut self, i: usize) -> Result<T, VectorError> {
        if i >= self.length() {
            return Err(VectorError::IndexOutOfBounds);
        }
        Ok(self.elements.remove(i))
    }

    /// Insert element at index
    pub fn insert(&mut self, e: T, i: usize) -> Result<(), VectorError> {
        if i > self.length() {
            return Err(VectorError::IndexOutOfBounds);
        }
        self.elements.insert(i, e);
        Ok(())
    }

    /// Swap remove element at index
    pub fn swap_remove(&mut self, i: usize) -> Result<T, VectorError> {
        if self.is_empty() || i >= self.length() {
            return Err(VectorError::IndexOutOfBounds);
        }
        Ok(self.elements.swap_remove(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut v = Vector::empty();
        v.push_back(1);
        v.push_back(2);
        v.push_back(3);

        assert_eq!(*v.borrow(0).unwrap(), 1);
        assert_eq!(v.length(), 3);
        assert!(!v.is_empty());
    }

    #[test]
    fn test_insert_remove() {
        let mut v = Vector::singleton(1);
        v.insert(2, 1).unwrap();
        assert_eq!(*v.borrow(1).unwrap(), 2);

        let removed = v.remove(0).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(v.length(), 1);
    }

    #[test]
    fn test_swap_remove() {
        let mut v = Vector::empty();
        v.push_back(1);
        v.push_back(2);
        v.push_back(3);

        let removed = v.swap_remove(1).unwrap();
        assert_eq!(removed, 2);
        assert_eq!(v.length(), 2);
        assert_eq!(*v.borrow(1).unwrap(), 3);
    }
}