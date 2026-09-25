// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::gas_coin::GasModule;
use kanari_crypto::hash_data_blake3;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use tracing::error;

use super::{GasPayment, NativeCall, ObjectInput, ObjectOwnerKind, ObjectRef, PublishedModule};

/// Transaction types in Kanari blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transaction {
    /// Publish a Move module
    PublishModule {
        sender: String,
        module_bytes: Vec<u8>,
        module_name: String,
        gas_payment: Option<GasPayment>,
        gas_limit: u64,
        gas_price: u64,
        nonce: u64,
    },
    /// Publish a Move package as one atomic transaction.
    PublishPackage {
        sender: String,
        modules: Vec<PublishedModule>,
        gas_payment: Option<GasPayment>,
        gas_limit: u64,
        gas_price: u64,
        nonce: u64,
    },
    /// Upgrade an existing Move module.
    UpgradeModule {
        sender: String,
        module_bytes: Vec<u8>,
        module_name: String,
        gas_payment: Option<GasPayment>,
        gas_limit: u64,
        gas_price: u64,
        nonce: u64,
    },
    /// Upgrade an existing Move package as one atomic transaction.
    UpgradePackage {
        sender: String,
        modules: Vec<PublishedModule>,
        gas_payment: Option<GasPayment>,
        gas_limit: u64,
        gas_price: u64,
        nonce: u64,
    },
    /// Execute a Move function
    ExecuteFunction {
        sender: String,
        module: String,
        function: String,
        type_args: Vec<String>,
        args: Vec<Vec<u8>>,
        object_inputs: Vec<ObjectInput>,
        gas_payment: Option<GasPayment>,
        gas_limit: u64,
        gas_price: u64,
        nonce: u64,
    },
}

impl Transaction {
    pub fn hash(&self) -> Vec<u8> {
        let serialized = match bcs::to_bytes(self) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to serialize Transaction for hashing: {}", e);
                Vec::new()
            }
        };
        hash_data_blake3(&serialized)
    }

    pub fn sender(&self) -> &str {
        match self {
            Transaction::PublishModule { sender, .. }
            | Transaction::PublishPackage { sender, .. }
            | Transaction::UpgradeModule { sender, .. }
            | Transaction::UpgradePackage { sender, .. } => sender,
            Transaction::ExecuteFunction { sender, .. } => sender,
        }
    }

    pub fn sender_address(&self) -> &str {
        self.sender()
    }

    pub fn nonce(&self) -> u64 {
        match self {
            Transaction::PublishModule { nonce, .. }
            | Transaction::PublishPackage { nonce, .. }
            | Transaction::UpgradeModule { nonce, .. }
            | Transaction::UpgradePackage { nonce, .. } => *nonce,
            Transaction::ExecuteFunction { nonce, .. } => *nonce,
        }
    }

    pub fn gas_limit(&self) -> u64 {
        match self {
            Transaction::PublishModule { gas_limit, .. }
            | Transaction::PublishPackage { gas_limit, .. }
            | Transaction::UpgradeModule { gas_limit, .. }
            | Transaction::UpgradePackage { gas_limit, .. } => *gas_limit,
            Transaction::ExecuteFunction { gas_limit, .. } => *gas_limit,
        }
    }

    pub fn gas_price(&self) -> u64 {
        match self {
            Transaction::PublishModule { gas_price, .. }
            | Transaction::PublishPackage { gas_price, .. }
            | Transaction::UpgradeModule { gas_price, .. }
            | Transaction::UpgradePackage { gas_price, .. } => *gas_price,
            Transaction::ExecuteFunction { gas_price, .. } => *gas_price,
        }
    }

    /// Get conflict keys for this transaction.
    /// Transactions with overlapping conflict keys must be executed sequentially.
    pub fn get_conflict_keys(&self) -> Vec<String> {
        let sender_norm = if let Ok(addr) = AccountAddress::from_hex_literal(self.sender()) {
            addr.to_hex_literal()
        } else {
            let s = self.sender();
            if !s.starts_with("0x") {
                format!("0x{}", s)
            } else {
                s.to_string()
            }
        };

        let mut keys = Vec::new();

        let object_access_keys = self.object_access_keys();
        if !object_access_keys.is_empty() {
            keys.extend(object_access_keys);
        } else {
            keys.push(format!("owner:{}", sender_norm));
        }

        match self {
            Transaction::ExecuteFunction { args, .. }
                if keys.len() == 1 && self.native_call().is_some() =>
            {
                for arg in args {
                    if arg.len() == 32
                        && let Ok(addr) = AccountAddress::from_bytes(arg)
                    {
                        keys.push(format!("object:{}", addr.to_hex_literal()));
                        continue;
                    }

                    // 2. Extract String from Argument (support both BCS String and Raw UTF-8)
                    let parsed_string = bcs::from_bytes::<String>(arg)
                        .or_else(|_| std::str::from_utf8(arg).map(|s| s.to_string()));

                    if let Ok(s) = parsed_string {
                        let s_trim = s.trim();

                        // Force add 0x prefix if missing
                        let hex_str = if !s_trim.starts_with("0x") {
                            format!("0x{}", s_trim)
                        } else {
                            s_trim.to_string()
                        };

                        // Use from_hex_literal to handle length and convert to Lowercase
                        if let Ok(addr) = AccountAddress::from_hex_literal(&hex_str) {
                            keys.push(format!("object:{}", addr.to_hex_literal()));
                        }
                    }
                }
            }
            Transaction::PublishModule { module_name, .. }
            | Transaction::UpgradeModule { module_name, .. } => {
                keys.push(format!("module:{}", module_name));
            }
            Transaction::PublishPackage { modules, .. }
            | Transaction::UpgradePackage { modules, .. } => {
                keys.extend(
                    modules
                        .iter()
                        .map(|module| format!("module:{}", module.module_name)),
                );
            }
            Transaction::ExecuteFunction { .. } => {}
        }
        keys.sort();
        keys.dedup();
        keys
    }

    pub fn object_inputs(&self) -> Vec<ObjectInput> {
        match self {
            Transaction::ExecuteFunction { object_inputs, .. } => object_inputs.clone(),
            Transaction::PublishModule { .. }
            | Transaction::PublishPackage { .. }
            | Transaction::UpgradeModule { .. }
            | Transaction::UpgradePackage { .. } => Vec::new(),
        }
    }

    pub fn gas_payment(&self) -> Option<GasPayment> {
        match self {
            Transaction::ExecuteFunction { gas_payment, .. } => gas_payment.clone(),
            Transaction::PublishModule { gas_payment, .. }
            | Transaction::PublishPackage { gas_payment, .. }
            | Transaction::UpgradeModule { gas_payment, .. }
            | Transaction::UpgradePackage { gas_payment, .. } => gas_payment.clone(),
        }
    }

    pub fn requires_strict_object_metadata(&self) -> bool {
        matches!(
            self,
            Transaction::ExecuteFunction { .. }
                | Transaction::PublishModule { .. }
                | Transaction::PublishPackage { .. }
                | Transaction::UpgradeModule { .. }
                | Transaction::UpgradePackage { .. }
        )
    }

    pub fn object_access_keys(&self) -> Vec<String> {
        let mut keys = self
            .object_inputs()
            .into_iter()
            // Conflict identity is the object itself. Access role must not create a
            // separate namespace, otherwise read/write and gas/input aliases look disjoint.
            .map(|input| format!("object:{}", input.object_ref.object_id))
            .collect::<Vec<_>>();

        if let Some(gas_payment) = self.gas_payment() {
            keys.extend(
                gas_payment
                    .payment_objects
                    .into_iter()
                    .map(|payment| format!("object:{}", payment.object_id)),
            );
        }

        keys.sort();
        keys.dedup();
        keys
    }

    pub fn primary_access_key(&self) -> String {
        if let Some(key) = self
            .object_inputs()
            .into_iter()
            .find(|input| input.mutable)
            .map(|input| format!("mut:object:{}", input.object_ref.object_id))
        {
            return key;
        }

        if let Some(key) = self.object_inputs().into_iter().next().map(|input| {
            let mutability = if input.mutable { "mut" } else { "ro" };
            format!("{mutability}:object:{}", input.object_ref.object_id)
        }) {
            return key;
        }

        if let Some(key) = self
            .gas_payment()
            .and_then(|gas_payment| gas_payment.payment_objects.into_iter().next())
            .map(|payment| format!("mut:gas:{}", payment.object_id))
        {
            return key;
        }

        format!("owner:{}", self.sender())
    }

    pub fn native_call(&self) -> Option<NativeCall> {
        let Transaction::ExecuteFunction {
            module,
            function,
            args,
            object_inputs,
            ..
        } = self
        else {
            return None;
        };

        let kanari_module = GasModule::module_path();
        if module != &kanari_module {
            return None;
        }

        match function.as_str() {
            function_name
                if function_name == GasModule::function_names().transfer && args.len() >= 3 =>
            {
                // The ABI carries the authoritative coin address in args[0]. Object
                // inputs are not ordered, so using the first input can select the
                // wrong coin when callers include additional inputs.
                let coin_object_id = AccountAddress::from_bytes(args.first()?.as_slice())
                    .ok()?
                    .to_hex_literal();
                // Keep the native fast path fail-closed: the ABI coin must be a
                // mutable object input. Other shapes go through full Move execution.
                if !object_inputs.iter().any(|input| {
                    input.mutable
                        && AccountAddress::from_hex_literal(&input.object_ref.object_id)
                            .map(|address| address.to_hex_literal() == coin_object_id)
                            .unwrap_or(false)
                }) {
                    return None;
                }
                let amount = bcs::from_bytes::<u64>(args.get(1)?).ok()?;
                let recipient = AccountAddress::from_bytes(args.get(2)?.as_slice())
                    .ok()?
                    .to_hex_literal();
                Some(NativeCall::Transfer {
                    coin_object_id,
                    recipient,
                    amount,
                })
            }
            function_name
                if function_name == GasModule::function_names().burn && !args.is_empty() =>
            {
                let amount = bcs::from_bytes::<u64>(args.last()?).ok()?;
                Some(NativeCall::Burn { amount })
            }
            _ => None,
        }
    }

    pub fn is_native_balance_call(&self) -> bool {
        self.native_call().is_some()
    }

    pub fn tx_type_label(&self) -> &'static str {
        if let Some(native_call) = self.native_call() {
            return native_call.tx_type_label();
        }

        match self {
            Transaction::PublishModule { .. } => "publish_module",
            Transaction::PublishPackage { .. } => "publish_package",
            Transaction::UpgradeModule { .. } => "upgrade_module",
            Transaction::UpgradePackage { .. } => "upgrade_package",
            Transaction::ExecuteFunction {
                module, function, ..
            } if module == &GasModule::module_path()
                && function == GasModule::function_names().transfer =>
            {
                "transfer"
            }
            Transaction::ExecuteFunction {
                module, function, ..
            } if module == &GasModule::module_path()
                && function == GasModule::function_names().burn =>
            {
                "burn"
            }
            Transaction::ExecuteFunction { .. } => "call",
        }
    }

    /// Create an object-input KANARI Move transfer transaction with default gas settings.
    pub fn new_transfer(
        from: String,
        coin_object_id: String,
        to: String,
        amount: u64,
        nonce: u64,
    ) -> Self {
        Self::new_transfer_with_gas(from, coin_object_id, to, amount, nonce, 100_000, 1000)
    }

    pub fn new_transfer_with_gas(
        from: String,
        coin_object_id: String,
        to: String,
        amount: u64,
        nonce: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Self {
        Self::new_transfer_with_object_ref_and_gas(
            from,
            ObjectRef::new(coin_object_id, None, None),
            to,
            amount,
            nonce,
            gas_limit,
            gas_price,
        )
    }

    pub fn new_transfer_with_object_ref(
        from: String,
        coin_object_ref: ObjectRef,
        to: String,
        amount: u64,
        nonce: u64,
    ) -> Self {
        Self::new_transfer_with_object_ref_and_gas(
            from,
            coin_object_ref,
            to,
            amount,
            nonce,
            100_000,
            1000,
        )
    }

    pub fn new_transfer_with_object_ref_and_gas(
        from: String,
        coin_object_ref: ObjectRef,
        to: String,
        amount: u64,
        nonce: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Self {
        let coin_object_addr = AccountAddress::from_hex_literal(&coin_object_ref.object_id)
            .unwrap_or(AccountAddress::ZERO);
        let recipient_addr = AccountAddress::from_hex_literal(&to).unwrap_or(AccountAddress::ZERO);

        Self::ExecuteFunction {
            sender: from.clone(),
            module: GasModule::module_path(),
            function: GasModule::function_names().transfer.to_string(),
            type_args: vec![],
            args: vec![
                coin_object_addr.to_vec(),
                bcs::to_bytes(&amount).unwrap_or_default(),
                recipient_addr.to_vec(),
            ],
            object_inputs: vec![ObjectInput {
                object_ref: coin_object_ref.clone(),
                owner: Some(ObjectOwnerKind::AddressOwner(from.clone())),
                mutable: true,
            }],
            gas_payment: Some(GasPayment {
                payment_objects: vec![coin_object_ref],
                owner: from,
                budget: gas_limit,
                price: gas_price,
            }),
            gas_limit,
            gas_price,
            nonce,
        }
    }

    /// Create a burn transaction with default gas settings
    pub fn new_burn(from: String, amount: u64, nonce: u64) -> Self {
        Self::new_burn_with_gas(from, amount, nonce, 100_000, 1000)
    }

    pub fn new_burn_with_gas(
        from: String,
        amount: u64,
        nonce: u64,
        gas_limit: u64,
        gas_price: u64,
    ) -> Self {
        Self::ExecuteFunction {
            sender: from.clone(),
            module: GasModule::module_path(),
            function: GasModule::function_names().burn.to_string(),
            type_args: vec![],
            args: vec![bcs::to_bytes(&amount).unwrap_or_default()],
            object_inputs: Vec::new(),
            gas_payment: Some(GasPayment {
                payment_objects: Vec::new(),
                owner: from,
                budget: gas_limit,
                price: gas_price,
            }),
            gas_limit,
            gas_price,
            nonce,
        }
    }
}
