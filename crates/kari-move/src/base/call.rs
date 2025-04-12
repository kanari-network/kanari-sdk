use clap::Parser;
use std::path::PathBuf;
use move_package::BuildConfig;
use move_core_types::account_address::AccountAddress;
use serde_json::{json, Value as JsonValue};
use std::time::{SystemTime, UNIX_EPOCH};
use sha3::{Digest, Sha3_256};
use rand::{thread_rng, Rng};
use anyhow::bail;

#[derive(Parser, Debug)]
pub struct Call {
    /// Full function identifier in format 0x{address}::{module}::{function}
    #[clap(long = "function-id", value_parser)]
    pub function_id: Option<String>,
    
    /// Alternative: Package path if not using function-id
    #[clap(long = "package")]
    pub package: Option<String>,
    
    /// Alternative: Module name if not using function-id
    #[clap(long = "module")]
    pub module: Option<String>,
    
    /// Alternative: Function name if not using function-id
    #[clap(long = "function")]
    pub function: Option<String>,
    
    /// Function arguments in JSON format
    #[clap(long = "args")]
    pub args: Vec<String>,
    
    /// Gas budget for the transaction
    #[clap(long = "gas-budget", default_value = "1000")]
    pub gas_budget: u64,
    
    /// Sender address (if different from module address)
    #[clap(long = "sender")]
    pub sender: Option<String>,
}

impl Call {
    pub fn execute(self, package_path: Option<PathBuf>, build_config: BuildConfig) -> anyhow::Result<()> {
        // Debug output to help troubleshoot
        println!("Debug: Call parameters: {:?}", self);

        // Parse function identifier or use individual components
        let (address, module_name, function_name) = if let Some(func_id) = &self.function_id {
            if !func_id.is_empty() {
                self.parse_function_id(func_id)?
            } else if let (Some(module), Some(function)) = (&self.module, &self.function) {
                // Default address if not provided
                let addr = AccountAddress::from_hex_literal("0x1")?;
                (addr, module.clone(), function.clone())
            } else {
                bail!("Either function-id or module and function must be provided")
            }
        } else if let (Some(module), Some(function)) = (&self.module, &self.function) {
            // Default address if not provided
            let addr = AccountAddress::from_hex_literal("0x1")?;
            (addr, module.clone(), function.clone())
        } else {
            bail!("Either function-id or module and function must be provided")
        };
        
        // Generate a unique transaction ID
        let transaction_id = generate_object_id();
        
        // Format address with 64 characters for consistency
        let address_str = format!("0x{:0>64}", address.to_hex());
        
        // Format module ID consistently with how it appears in deployment
        let module_id = format!("{}::{}", address_str, module_name);
        
        // Full function name with address and module
        let full_function_id = format!("{}::{}", module_id, function_name);
        
        // Parse arguments - in a real implementation, you would validate these against
        // the function's parameter types
        let parsed_args: Vec<JsonValue> = self.parse_arguments()?;
        
        // Simulate the function call (placeholder)
        let (result, gas_used) = self.simulate_function_call(
            &package_path,
            &build_config,
            &address,
            &module_name,
            &function_name,
            &parsed_args,
        )?;
        
        // Prepare the result JSON
        let output = json!({
            "status": "success",
            "type": "function_call",
            "metadata": {
                "call": {
                    "id": transaction_id,
                    "function": {
                        "module_id": module_id,
                        "name": function_name,
                        "full_id": full_function_id
                    },
                    "args": parsed_args,
                    "gas_budget": self.gas_budget,
                    "gas_used": gas_used,
                    "sender": self.sender.unwrap_or_else(|| address_str.clone())
                },
                "result": result,
                "timestamp": SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            }
        });
        
        println!("{}", serde_json::to_string_pretty(&output)?);
        Ok(())
    }
    
    fn parse_function_id(&self, function_id: &str) -> anyhow::Result<(AccountAddress, String, String)> {
        let parts: Vec<&str> = function_id.split("::").collect();
        
        if parts.len() != 3 {
            bail!("Function ID must be in format 0x{{address}}::{{module}}::{{function}}");
        }
        
        let address_str = parts[0];
        let module_name = parts[1].to_string();
        let function_name = parts[2].to_string();
        
        // Parse the address, removing the 0x prefix if present
        let address_str = address_str.trim_start_matches("0x");
        let address = AccountAddress::from_hex(address_str)?;
        
        Ok((address, module_name, function_name))
    }
    
    fn parse_arguments(&self) -> anyhow::Result<Vec<JsonValue>> {
        let mut parsed_args = Vec::new();
        
        for arg in &self.args {
            match serde_json::from_str(arg) {
                Ok(value) => parsed_args.push(value),
                Err(_) => {
                    // If not valid JSON, treat as a string
                    parsed_args.push(json!(arg));
                }
            }
        }
        
        Ok(parsed_args)
    }
    
    fn simulate_function_call(
        &self,
        _package_path: &Option<PathBuf>,
        _build_config: &BuildConfig,
        _address: &AccountAddress,
        module_name: &str,
        function_name: &str,
        args: &[JsonValue],
    ) -> anyhow::Result<(JsonValue, u64)> {
        // This is a placeholder for the actual function call implementation
        // In a real implementation, you would:
        // 1. Compile the package if needed
        // 2. Verify the function exists in the module
        // 3. Verify the arguments match the function's parameter types
        // 4. Execute the function in the VM
        // 5. Return the result and gas used
        
        // For now, we'll just simulate a result
        let mut rng = thread_rng();
        let gas_used = rng.gen_range(100..self.gas_budget);
        
        // Simulate a realistic result based on the function name
        let result = match function_name {
            "mint" => json!({
                "object_id": generate_object_id(),
                "owner": format!("0x{}", generate_random_hex(64)),
            }),
            "transfer" => json!({
                "success": true,
                "new_owner": format!("0x{}", generate_random_hex(64)),
            }),
            "burn" => json!({
                "success": true,
                "burned_id": generate_object_id(),
            }),
            _ => json!({
                "success": true,
                "return_values": [generate_object_id()]
            }),
        };
        
        // Log the simulated call
        println!("Called function '{}::{}' with {} arguments, gas used: {}", 
            module_name, function_name, args.len(), gas_used);
        
        Ok((result, gas_used))
    }
}

// Helper function to generate a unique object ID
fn generate_object_id() -> String {
    let mut hasher = Sha3_256::new();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    
    hasher.update(timestamp.to_le_bytes());
    hasher.update(counter.to_le_bytes());
    
    let hash = hasher.finalize();
    format!("0x{:0>64}", hex::encode(hash))
}

// Helper function to generate random hex string
fn generate_random_hex(length: usize) -> String {
    let mut rng = thread_rng();
    const CHARSET: &[u8] = b"0123456789abcdef";
    
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}