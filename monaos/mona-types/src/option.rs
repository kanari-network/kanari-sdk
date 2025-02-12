use std::vec::Vec;

/// Custom Option type that mirrors the Move implementation
#[derive(Debug, Clone)]
pub struct Option<T> {
    vec: Vec<T>,
}

/// Error type for Option operations
#[derive(Debug, PartialEq)]
pub enum OptionError {
    OptionIsSet,
    OptionNotSet,
}

impl<T> Option<T> {
    /// Create a None variant
    pub fn none() -> Self {
        Option { vec: Vec::new() }
    }

    /// Create a Some variant containing a value
    pub fn some(value: T) -> Self {
        Option { vec: vec![value] }
    }

    /// Check if the option is None
    pub fn is_none(&self) -> bool {
        self.vec.is_empty()
    }

    /// Check if the option is Some
    pub fn is_some(&self) -> bool {
        !self.vec.is_empty()
    }

    /// Check if contains a specific value
    pub fn contains(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.vec.contains(value)
    }

    /// Borrow the contained value
    pub fn borrow(&self) -> Result<&T, OptionError> {
        self.vec.first().ok_or(OptionError::OptionNotSet)
    }

    /// Borrow with a default value
    pub fn borrow_with_default<'a>(&'a self, default: &'a T) -> &T {
        self.vec.first().unwrap_or(default)
    }

    /// Get value with a default
    pub fn get_with_default(&self, default: T) -> T
    where
        T: Clone,
    {
        self.vec.first().cloned().unwrap_or(default)
    }

    /// Fill a None option with a value
    pub fn fill(&mut self, value: T) -> Result<(), OptionError> {
        if self.is_none() {
            self.vec.push(value);
            Ok(())
        } else {
            Err(OptionError::OptionIsSet)
        }
    }

    /// Extract the contained value
    pub fn extract(&mut self) -> Result<T, OptionError> {
        if self.is_some() {
            Ok(self.vec.pop().unwrap())
        } else {
            Err(OptionError::OptionNotSet)
        }
    }

    /// Borrow the contained value mutably
    pub fn borrow_mut(&mut self) -> Result<&mut T, OptionError> {
        self.vec.first_mut().ok_or(OptionError::OptionNotSet)
    }

    /// Swap the contained value with a new one
    pub fn swap(&mut self, value: T) -> Result<T, OptionError> {
        if self.is_some() {
            let old = self.vec.pop().unwrap();
            self.vec.push(value);
            Ok(old)
        } else {
            Err(OptionError::OptionNotSet)
        }
    }

    /// Swap or fill with a new value
    pub fn swap_or_fill(&mut self, value: T) -> std::option::Option<T> {
        let old = self.vec.pop();
        self.vec.push(value);
        old
    }

    /// Destroy and return value or default
    pub fn destroy_with_default(self, default: T) -> T {
        self.vec.into_iter().next().unwrap_or(default)
    }

    /// Destroy and return Some value
    pub fn destroy_some(self) -> Result<T, OptionError> {
        if self.is_some() {
            Ok(self.vec.into_iter().next().unwrap())
        } else {
            Err(OptionError::OptionNotSet)
        }
    }

    /// Destroy None variant
    pub fn destroy_none(self) -> Result<(), OptionError> {
        if self.is_none() {
            Ok(())
        } else {
            Err(OptionError::OptionIsSet)
        }
    }

    /// Convert to Vec
    pub fn to_vec(self) -> Vec<T> {
        self.vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        let mut opt = Option::none();
        assert!(opt.is_none());

        opt.fill(42).unwrap();
        assert!(opt.is_some());
        assert_eq!(*opt.borrow().unwrap(), 42);

        let extracted = opt.extract().unwrap();
        assert_eq!(extracted, 42);
        assert!(opt.is_none());
    }

    #[test]
    fn test_swap_operations() {
        let mut opt = Option::some(42);
        let swapped = opt.swap(24).unwrap();
        assert_eq!(swapped, 42);
        assert_eq!(*opt.borrow().unwrap(), 24);
    }

    #[test]
    fn test_default_operations() {
        let opt = Option::<i32>::none();
        assert_eq!(opt.get_with_default(10), 10);

        let opt = Option::some(42);
        assert_eq!(opt.get_with_default(10), 42);
    }

    #[test]
    fn test_swap_or_fill() {
        let mut opt = Option::none();
        let old = opt.swap_or_fill(42);
        assert!(old.is_none());
        assert_eq!(*opt.borrow().unwrap(), 42);

        let old = opt.swap_or_fill(24);
        assert_eq!(old, Some(42));
        assert_eq!(*opt.borrow().unwrap(), 24);
    }
}
