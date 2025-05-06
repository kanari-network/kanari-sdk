# Kanari Move Blockchain Commands

## Using kari move publish

The `kari move publish` command deploys Move modules to the blockchain network. It handles compilation, transaction signing, and deployment of your Move code.

### Command Format

```bash
kari move publish [MODULE_PATH] [OPTIONS]
```

### Parameters

- `MODULE_PATH`: Path to the Move module directory to publish (defaults to current directory)
- `--gas-budget=N`: Gas units allocated for deployment (default: 3,000,000)
- `--skip-verify`: Skip module verification checks (not recommended for production)
- `--address=ADDRESS`: Blockchain address to deploy the module to (format: 0x...)
- `--password=PASSWORD`: Password for wallet to sign the deployment transaction

### Examples

```bash
# Deploy module from current directory
kari move publish

# Deploy module from specific path
kari move publish path/to/module

# Specify gas budget
kari move publish --gas-budget=5000000

# Deploy to specific address
kari move publish --address=0x123abc

# Skip verification and provide password
kari move publish --skip-verify --password=your_password
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

- `--module-id`: Module identifier in format `<address>::<module_name>` (e.g., `0x1234::counter`)
- `--function`: Function name to call within the module

### Optional Parameters

- `--args`: Comma-separated list of typed arguments in format `<type>:<value>`
- `--gas-budget`: Gas units allocated for function call (default: 1,000,000)
- `--address`: Blockchain address to call from (if not specified, uses wallet address or defaults to 0x1)

### Supported Argument Types

- `address:0x123` - Blockchain address
- `u8:123` - 8-bit unsigned integer
- `u64:1000` - 64-bit unsigned integer
- `u128:1000000` - 128-bit unsigned integer
- `bool:true` - Boolean value (true/false)
- `string:hello` - String value

### Examples

```bash
# Call a function with no arguments
kari move call --module-id=0x123::counter --function=initialize

# Call a function with typed arguments
kari move call --module-id=0x123::counter --function=increment --args=u64:1

# Call a function with multiple arguments
kari move call --module-id=0x123::token --function=transfer --args=address:0x456,u64:100

# Specify custom gas budget
kari move call --module-id=0x123::marketplace --function=list_item --gas-budget=2000000 --args=string:item_name,u64:1000,bool:true

# Call from a specific address
kari move call --module-id=0x123::token --function=mint --address=0x789 --args=u64:1000
```

### Output

The command provides detailed execution information:
- Function call details and arguments
- Gas usage and transaction ID
- Execution time and status
- Formatted result with any return values
- Detailed error information if the call fails