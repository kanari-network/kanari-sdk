// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Input validation module for kanari-move-runtime-v1
//! This module provides safety checks for all external inputs to prevent DoS and exploits.

use anyhow::{Result, anyhow};
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::{ModuleId, TypeTag};

// ============================================================================
// Size Limits
// ============================================================================

/// Maximum module bytecode size (10 MB)
pub const MAX_MODULE_SIZE: usize = 10 * 1024 * 1024;

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

/// Maximum number of objects per transaction
pub const MAX_OBJECTS_PER_TX: usize = 1000;

/// Maximum transaction size
pub const MAX_TX_SIZE: usize = 10 * 1024 * 1024; // 10 MB

// ============================================================================
// Validation Functions
// ============================================================================

/// Validates module bytecode before publishing
pub fn validate_module_bytes(bytes: &[u8]) -> Result<()> {
    // Check empty
    if bytes.is_empty() {
        anyhow::bail!("Module bytes cannot be empty");
    }

    // Check size limit
    if bytes.len() > MAX_MODULE_SIZE {
        anyhow::bail!(
            "Module bytes too large: {} > {}",
            bytes.len(),
            MAX_MODULE_SIZE
        );
    }

    // Note: We skip bytecode parsing as CompiledModule::deserialize
    // is not available in the current dependencies.
    // The size check above provides basic protection.
    // For production, add proper bytecode verification.

    Ok(())
}

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

/// Validates sender address
pub fn validate_sender(_sender: AccountAddress) -> Result<()> {
    // In Kanari, zero address might be valid for system transactions
    // But for user transactions, we allow any address
    // This is mostly a placeholder for future validation
    Ok(())
}

/// Validates object ID
pub fn validate_object_id(object_id: &str) -> Result<()> {
    if object_id.is_empty() {
        anyhow::bail!("Object ID cannot be empty");
    }

    if object_id.len() > 128 {
        anyhow::bail!("Object ID too long: {} > 128", object_id.len());
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
    sender: Option<AccountAddress>,
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

    // Validate sender
    if let Some(sender) = sender {
        validate_sender(sender)?;
    }

    // Validate gas info if provided
    if let Some((limit, price)) = gas_info {
        validate_gas_info(limit, price)?;
    }

    Ok(())
}

/// Validates module publishing
pub fn validate_module_publish(module_bytes: &[u8], sender: AccountAddress) -> Result<()> {
    validate_module_bytes(module_bytes)?;
    validate_sender(sender)?;

    Ok(())
}

/// Validates module upgrade
pub fn validate_module_upgrade(
    module_id: &ModuleId,
    new_module_bytes: &[u8],
    sender: AccountAddress,
) -> Result<()> {
    validate_module_id(module_id)?;
    validate_module_bytes(new_module_bytes)?;
    validate_sender(sender)?;

    Ok(())
}

// ============================================================================
// Rate Limiting (Simple Implementation)
// ============================================================================

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Simple rate limiter using token bucket algorithm
pub struct RateLimiter {
    tokens: AtomicU64,
    max_tokens: u64,
    last_refill: std::sync::Mutex<Instant>,
    refill_interval: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_tokens: u64, refill_interval: Duration) -> Self {
        Self {
            tokens: AtomicU64::new(max_tokens),
            max_tokens,
            last_refill: std::sync::Mutex::new(Instant::now()),
            refill_interval,
        }
    }

    /// Try to acquire a token
    pub fn try_acquire(&self) -> bool {
        self.refill_if_needed();

        loop {
            let current = self.tokens.load(Ordering::Acquire);
            if current == 0 {
                return false;
            }
            if self
                .tokens
                .compare_exchange_weak(current, current - 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Acquire a token, waiting if necessary
    /// Returns error if timeout is exceeded (5 seconds)
    pub fn acquire(&self) -> Result<()> {
        use std::time::Instant;

        let start = Instant::now();
        let timeout = Duration::from_secs(5); // 5 second timeout

        while !self.try_acquire() {
            if start.elapsed() >= timeout {
                return Err(anyhow!("Rate limiter acquire timeout after 5 seconds"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        Ok(())
    }

    fn refill_if_needed(&self) {
        let now = Instant::now();
        // Safe: Handle poisoned mutex gracefully
        let mut last_refill = match self.last_refill.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // Log the poison event and recover
                log::warn!("Rate limiter mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };

        if now.duration_since(*last_refill) >= self.refill_interval {
            self.tokens.store(self.max_tokens, Ordering::Relaxed);
            *last_refill = now;
        }
    }
}

// ============================================================================
// Safe Wrapper Types
// ============================================================================

/// Validated module bytes
#[derive(Debug, Clone)]
#[must_use]
pub struct ValidatedModuleBytes {
    bytes: Vec<u8>,
}

impl ValidatedModuleBytes {
    /// Creates a new validated module bytes wrapper, rejecting invalid input.
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        validate_module_bytes(&bytes)?;
        Ok(Self { bytes })
    }

    /// Consumes the wrapper and returns the inner byte vector.
    pub fn into_inner(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns a borrowed slice of the validated bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }
}

/// Validated function name
#[derive(Debug, Clone)]
#[must_use]
pub struct ValidatedFunctionName {
    name: String,
}

impl ValidatedFunctionName {
    /// Creates a new validated function name, rejecting invalid input.
    pub fn new(name: String) -> Result<Self> {
        validate_function_name(&name)?;
        Ok(Self { name })
    }

    /// Returns the validated function name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

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
    fn test_validate_empty_module() {
        assert!(validate_module_bytes(&[]).is_err());
    }

    #[test]
    fn test_validate_large_module() {
        let large = vec![0u8; MAX_MODULE_SIZE + 1];
        assert!(validate_module_bytes(&large).is_err());
    }

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
    fn test_validated_module_bytes() {
        let valid = vec![0u8; 100];
        let result = ValidatedModuleBytes::new(valid);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validated_gas_info() {
        let result = ValidatedGasInfo::new(1000, 1);
        assert!(result.is_ok());
    }
}
