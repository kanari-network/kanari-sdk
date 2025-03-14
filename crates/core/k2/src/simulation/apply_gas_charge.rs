use log::error;

use crate::blockchain::BALANCES;

use super::{get_mutex_lock, SimulationError};

// Helper function to charge gas for failed transactions
pub fn apply_gas_charge(sender: &str, gas_charge: u64) -> Result<(), SimulationError> {
    let mut balances = get_mutex_lock(&BALANCES, "apply_gas_charge")?;

    if let Some(balance) = balances.get_mut(sender) {
        if *balance >= gas_charge {
            *balance -= gas_charge;
            Ok(())
        } else {
            let err = SimulationError::InsufficientFunds(gas_charge, *balance);
            error!("{}", err);
            Err(err)
        }
    } else {
        let err = SimulationError::AddressNotFound(sender.to_string());
        error!("{}", err);
        Err(err)
    }
}