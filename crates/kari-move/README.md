# Kanari Move Blockchain Commands

This guide explains how to use the Kanari Move blockchain commands to deploy and interact with smart contracts on the blockchain.

## Wallet Management

Before publishing or interacting with Move modules, you need to set up a wallet. Kanari SDK provides several methods to create and manage wallets using the `kari keytool` commands.

### Creating a New Wallet

```bash
# Generate a new wallet
kari keytool generate
```

This command will:
1. Ask for the mnemonic length (12 or 24 words)
2. Ask for the curve type (K-256, P-256, or Ed25519)
3. Generate and display the private key, public address, and seed phrase
4. Ask for a password to encrypt and save the wallet

### Importing an Existing Wallet

```bash
# Import from seed phrase
kari keytool import

# Import from private key
kari keytool privatekey
```

### Managing Multiple Wallets

```bash
# List available wallets
kari keytool list

# Select a wallet to use
kari keytool select
```

### Checking Wallet Balance

```bash
# Check wallet balance
kari keytool balance
```

## Using kari move publish

The `kari move publish` command deploys Move modules to the blockchain network. It handles compilation, transaction signing, and deployment of your Move code.

### Command Format

```bash
kari move publish [MODULE_PATH] [OPTIONS]
```

### Parameters

| Parameter | Format | Description |
|-----------|--------|-------------|
| `MODULE_PATH` | Path | Directory containing the Move package to publish (optional, defaults to current directory) |
| `--gas-budget` | Integer | Amount of gas units allocated for deployment (default: 3,000,000) |
| `--skip-verify` | Flag | Skip module verification steps (not recommended for production) |
| `--address` | Hex string | Blockchain address to deploy the module to (uses wallet address if not specified) |
| `--password` | String | Password for wallet to sign deployment transaction (will prompt if not provided) |

### Examples

```bash
# Publish module from current directory
kari move publish

# Publish module from a specific path
kari move publish path/to/module

# Publish with custom gas budget
kari move publish --gas-budget=5000000

# Publish to a specific address (instead of wallet address)
kari move publish --address=0x123abc456def

# Skip verification steps for faster deployment
kari move publish --skip-verify

# Provide password directly in command (not recommended for security)
kari move publish --password=your_password
```

### Output

The command provides detailed progress information:
- Package and sources verification
- Transaction signing status
- Compilation and deployment progress with timeout monitoring
- Detailed deployment results including:
  - Module names and IDs
  - Public functions available
  - Transaction details and gas usage

If deployment takes longer than 30 seconds, the command times out but the deployment may still complete in the background.

## Using kari move call

The `kari move call` command invokes functions in deployed Move modules. It formats arguments properly and displays execution results.

### Command Format

```bash
kari move call --module-id=<MODULE_ID> --function=<FUNCTION> [OPTIONS]
```

### Required Parameters

| Parameter | Format | Description |
|-----------|--------|-------------|
| `--module-id` | `<address>::<module_name>` | Address and name of the deployed module |
| `--function` | String | Name of the function to call |

### Optional Parameters

| Parameter | Format | Description |
|-----------|--------|-------------|
| `--args` | `<type>:<value>[,<type>:<value>...]` | Comma-separated list of typed arguments |
| `--gas-budget` | Integer | Amount of gas units allocated for function call (default: 1,000,000) |
| `--address` | Hex string | Blockchain address to call from (uses wallet address if not specified) |

### Supported Argument Types

| Type | Format | Example |
|------|--------|---------|
| `address` | Hex string | `address:0x123...` |
| `u8` | Integer | `u8:123` |
| `u64` | Integer | `u64:1000000` |
| `u128` | Integer | `u128:1000000000` |
| `bool` | Boolean | `bool:true` |
| `string` | String | `string:hello` |

### Examples

```bash
# Check token information (no arguments)
kari move call --module-id 0x28e442c54d872cea9415382e61559dde126da6d6aee8c70855bd6c8cbdeb40d8::token --function check_info

# Mint tokens (needs TreasuryCap object)
kari move call --module-id 0x28e442c54d872cea9415382e61559dde126da6d6aee8c70855bd6c8cbdeb40d8::token --function mint --args address:0x<treasury_cap>,u64:1000,address:0x<receiver>

# Burn tokens
kari move call --module-id 0x28e442c54d872cea9415382e61559dde126da6d6aee8c70855bd6c8cbdeb40d8::token --function burn --args address:0x<treasury_cap>,address:0x<coin>

# Add address to deny list
kari move call --module-id 0x28e442c54d872cea9415382e61559dde126da6d6aee8c70855bd6c8cbdeb40d8::token --function deny_list_add_admin --args address:0x<denylist>,address:0x<denycap>,address:0x<to_deny>

# Set custom gas budget 
kari move call --module-id 0x28e442c54d872cea9415382e61559dde126da6d6aee8c70855bd6c8cbdeb40d8::token --function mint --args address:0x123,u64:1000,address:0x456 --gas-budget=2000000

# Call from specific address
kari move call --module-id 0x28e442c54d872cea9415382e61559dde126da6d6aee8c70855bd6c8cbdeb40d8::token --function check_info --address=0x789
```

### Output

The command provides detailed execution information:
- Function call details and arguments
- Gas usage and transaction ID
- Execution time and status
- Formatted result with any return values
- Detailed error information if the call fails

## Best Practices

1. **Wallet Management**
   - Create a wallet with `kari keytool generate` before publishing
   - Keep your wallet password secure
   - Consider using environment variables for passwords in automated scripts

2. **Gas Management**
   - Start with the default gas budget and adjust as needed
   - Larger modules require more gas for deployment
   - Complex function calls may require higher gas budgets

3. **Error Handling**
   - If a module is not found, verify the module ID format
   - Function not found errors usually indicate the function is private or doesn't exist
   - Argument errors typically relate to incorrect type or number of arguments

4. **Module Deployment**
   - Always verify your Move code compiles locally before deploying
   - Test modules in a test environment before production deployment
   - Document your module's public functions for other users

## Common Workflows

### Token Creation and Management

1. Deploy token module:
   ```bash
   kari move publish token
   ```

2. Check token information:
   ```bash
   kari move call --module-id 0x<your_address>::token --function check_info
   ```

3. Mint tokens:
   ```bash
   kari move call --module-id 0x<your_address>::token --function mint --args address:0x<treasury_cap>,u64:1000,address:0x<receiver>
   ```

### Working with Capabilities

When your contract uses capability objects like `TreasuryCap`:

1. These capabilities are created during module deployment
2. They're sent to the deployer's address
3. You must use the address of the capability object, not your wallet address
4. Query your account to find capability objects

## Troubleshooting

- **Module Not Found**: Check address format, try with both 0x prefix and without
- **Function Not Found**: Verify function is public and spelled correctly
- **Argument Error**: Check arguments match the function signature
- **Timeout During Deployment**: Check if module was deployed despite the timeout
- **Invalid Signature**: Ensure your wallet password is correct

For more information, refer to the [Kanari Move documentation](https://docs.kanari.site).