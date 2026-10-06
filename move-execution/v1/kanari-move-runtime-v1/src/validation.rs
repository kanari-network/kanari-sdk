// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Input validation module for kanari-move-runtime-v1
//! This module provides safety checks for all external inputs to prevent DoS and exploits.

use anyhow::Result;
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::{ModuleId, TypeTag};

// ============================================================================
// Size Limits
// ============================================================================

/// Maximum number of type arguments per transaction
pub const MAX_TYPE_ARGS: usize = 100;

/// Maximum number of arguments per transaction
pub const MAX_ARGS: usize = 100;

/// Maximum size of a single argument (1 MB)
pub const MAX_ARG_SIZE: usize = 1024 * 1024;

/// Maximum length of function names
pub const MAX_FUNCTION_NAME_LENGTH: usize = 256;

/// Maximum length of module names
pub const MAX_MODULE_NAME_LENGTH: usize = 256;

// ============================================================================
// Validation Functions
// ============================================================================

/// Validates function name
pub fn validate_function_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Function name cannot be empty");
    }

    if name.len() > MAX_FUNCTION_NAME_LENGTH {
        anyhow::bail!(
            "Function name too long: {} > {}",
            name.len(),
            MAX_FUNCTION_NAME_LENGTH
        );
    }

    // Check for null bytes
    if name.contains('\0') {
        anyhow::bail!("Function name cannot contain null bytes");
    }

    Ok(())
}

/// Validates module ID
pub fn validate_module_id(module_id: &ModuleId) -> Result<()> {
    // Validate address
    // Address is always valid in Move, but we check it's not zero for publishing
    if *module_id.address() == AccountAddress::ZERO {
        anyhow::bail!("Cannot publish to zero address");
    }

    // Validate module name
    let module_name = module_id.name().as_str();
    if module_name.is_empty() {
        anyhow::bail!("Module name cannot be empty");
    }

    if module_name.len() > MAX_MODULE_NAME_LENGTH {
        anyhow::bail!(
            "Module name too long: {} > {}",
            module_name.len(),
            MAX_MODULE_NAME_LENGTH
        );
    }

    Ok(())
}

/// Validates type tags
pub fn validate_type_tags(type_tags: &[TypeTag]) -> Result<()> {
    if type_tags.len() > MAX_TYPE_ARGS {
        anyhow::bail!(
            "Too many type arguments: {} > {}",
            type_tags.len(),
            MAX_TYPE_ARGS
        );
    }

    for tag in type_tags {
        // Validate each type tag's string representation
        let type_str = tag.to_string();
        if type_str.len() > MAX_MODULE_NAME_LENGTH * 2 {
            anyhow::bail!("Type tag string representation too long");
        }
    }

    Ok(())
}

/// Validates transaction arguments
pub fn validate_args(args: &[Vec<u8>]) -> Result<()> {
    if args.len() > MAX_ARGS {
        anyhow::bail!("Too many arguments: {} > {}", args.len(), MAX_ARGS);
    }

    for (i, arg) in args.iter().enumerate() {
        if arg.len() > MAX_ARG_SIZE {
            anyhow::bail!("Argument {} too large: {} > {}", i, arg.len(), MAX_ARG_SIZE);
        }
    }

    Ok(())
}

/// Validates gas parameters
pub fn validate_gas_info(gas_limit: u64, gas_price: u64) -> Result<()> {
    // Check for overflow in total cost calculation
    // gas_limit * gas_price should not overflow u64
    if gas_limit > 0 && gas_price > u64::MAX / gas_limit {
        anyhow::bail!(
            "Gas calculation would overflow: limit={}, price={}",
            gas_limit,
            gas_price
        );
    }

    // Reasonable limits
    const MAX_GAS_LIMIT: u64 = 10_000_000_000; // 10B gas
    const MAX_GAS_PRICE: u64 = 10_000_000_000; // 10B price

    if gas_limit > MAX_GAS_LIMIT {
        anyhow::bail!("Gas limit too high: {} > {}", gas_limit, MAX_GAS_LIMIT);
    }

    if gas_price > MAX_GAS_PRICE {
        anyhow::bail!("Gas price too high: {} > {}", gas_price, MAX_GAS_PRICE);
    }

    Ok(())
}

// ============================================================================
// Transaction Validation
// ============================================================================

/// Validates all aspects of a transaction before execution
pub fn validate_transaction(
    module_id: Option<&ModuleId>,
    function_name: &str,
    type_args: &[TypeTag],
    args: &[Vec<u8>],
    _sender: Option<AccountAddress>,
    gas_info: Option<(u64, u64)>,
) -> Result<()> {
    // Validate function name
    validate_function_name(function_name)?;

    // Validate module ID if provided
    if let Some(module_id) = module_id {
        validate_module_id(module_id)?;
    }

    // Validate type arguments
    validate_type_tags(type_args)?;

    // Validate arguments
    validate_args(args)?;

    // Validate gas info if provided
    if let Some((limit, price)) = gas_info {
        validate_gas_info(limit, price)?;
    }

    Ok(())
}

// ============================================================================
// Safe Wrapper Types
// ============================================================================

/// Validated gas info
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct ValidatedGasInfo {
    limit: u64,
    price: u64,
}

impl ValidatedGasInfo {
    /// Creates a new validated gas info, rejecting overflow or excessive values.
    pub fn new(limit: u64, price: u64) -> Result<Self> {
        validate_gas_info(limit, price)?;
        Ok(Self { limit, price })
    }

    /// Consumes the wrapper and returns the (limit, price) tuple.
    pub fn into_tuple(self) -> (u64, u64) {
        (self.limit, self.price)
    }

    /// Returns the gas limit.
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Returns the gas price.
    pub fn price(&self) -> u64 {
        self.price
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_function_name() {
        assert!(validate_function_name("").is_err());
    }

    #[test]
    fn test_validate_long_function_name() {
        let long = "a".repeat(MAX_FUNCTION_NAME_LENGTH + 1);
        assert!(validate_function_name(&long).is_err());
    }

    #[test]
    fn test_validate_gas_overflow() {
        assert!(validate_gas_info(u64::MAX, 2).is_err());
    }

    #[test]
    fn test_validate_too_many_type_args() {
        let too_many: Vec<TypeTag> = vec![TypeTag::Bool; MAX_TYPE_ARGS + 1];
        assert!(validate_type_tags(&too_many).is_err());
    }

    #[test]
    fn test_validate_too_many_args() {
        let too_many: Vec<Vec<u8>> = vec![vec![1u8]; MAX_ARGS + 1];
        assert!(validate_args(&too_many).is_err());
    }

    #[test]
    fn test_validated_gas_info() {
        let result = ValidatedGasInfo::new(1000, 1);
        assert!(result.is_ok());
    }
}
