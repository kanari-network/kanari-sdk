use crate::gas::TRANSACTION_GAS_COST;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Transaction {
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub gas_cost: f64, // Make gas_cost public
}

impl Transaction {
    #[allow(dead_code)]
    pub fn new(sender: String, receiver: String, amount: u64) -> Self {
        Self {
            sender,
            receiver,
            amount,
            gas_cost: TRANSACTION_GAS_COST,
        }
    }

    pub fn calculate_total_cost(&self) -> f64 {
        self.amount as f64 + self.gas_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_initialization() {
        let tx = Transaction::new(String::from("Alice"), String::from("Bob"), 10);
        assert_eq!(tx.sender, "Alice");
        assert_eq!(tx.receiver, "Bob");
        assert_eq!(tx.amount, 10);
        assert_eq!(tx.gas_cost, TRANSACTION_GAS_COST); // Check gas_cost
    }

    #[test]
    fn test_calculate_total_cost() {
        let tx = Transaction::new(String::from("Alice"), String::from("Bob"), 10);
        let expected_total_cost = 10.0 + TRANSACTION_GAS_COST;
        assert_eq!(tx.calculate_total_cost(), expected_total_cost);
    }
}