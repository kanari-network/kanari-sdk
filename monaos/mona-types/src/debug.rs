use std::fmt::Debug;

/// Module providing debug functionality
pub struct Debugger;

impl Debugger {
    /// Print a debug representation of a value
    pub fn print<T: Debug>(x: &T) {
        println!("{:?}", x);
    }

    /// Print the current stack trace
    pub fn print_stack_trace() {
        let backtrace = std::backtrace::Backtrace::capture();
        println!("{}", backtrace);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_print() {
        let value = 42;
        Debugger::print(&value); // Should print: 42
    }

    #[test]
    fn test_stack_trace() {
        Debugger::print_stack_trace(); // Should print current stack trace
    }
}