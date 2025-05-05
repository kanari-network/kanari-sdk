# Using kari move publish

The `kari move publish` command is used to upload Move modules to the Mona VM blockchain.

## Command Format

### Parameters

- `MODULE_PATH`: Path to the Move module to be uploaded (default is the current directory).
- `--gas-budget=N`: Number of gas units to use (default is 3,000,000).
- `--skip-verify`: Skip the module verification step.
- `--address=ADDRESS`: Address to use for uploading the module (if not specified, the address from the wallet or the default 0x1 will be used).

### Examples

```bash
# Upload the module from the current directory
kari move publish

# Upload the module from a specified path
kari move publish /path/to/module

# Specify the number of gas units
kari move publish --gas-budget=5000000

# Upload to a specific address
kari move publish --address=0x123abc

# Skip module verification
kari move publish --skip-verify
```

# Using kari move call

The `kari move call` command is used to invoke functions in Move modules that have been uploaded to the blockchain.

## Command Format


### Parameters

- `--module-id`: Specifies the module identifier in the format `<ADDRESS>::<MODULE_NAME>` (required)
- `--function`: Name of the function to call within the module (required)
- `--args`: Arguments for the function in the format `<TYPE>:<VALUE>` separated by commas
- `--gas-budget`: Number of gas units to use for the function call (default is 1,000,000)
- `--address`: The sender address for the function call (if not specified, uses the wallet address or defaults to 0x1)

### Supported Argument Types

- `address:0x123` - Blockchain address
- `u8:123` - 8-bit unsigned integer
- `u64:1000` - 64-bit unsigned integer
- `u128:1000000` - 128-bit unsigned integer
- `bool:true` - Boolean value
- `string:hello` - String value

### Examples

```bash
# Call a function without arguments
kari move call --module-id=0x123::coin --function=initialize

# Call a function with arguments
kari move call --module-id=0x123::coin --function=transfer --args=address:0x456,u64:100

# Specify gas budget
kari move call --module-id=0x123::coin --function=mint --gas-budget=5000000

# Call from a specific sender address
kari move call --module-id=0x123::coin --function=burn --address=0x789 --args=u64:50

# Call with multiple arguments of different types
kari move call --module-id=0x123::marketplace --function=list_item --args=string:item_name,u64:1000,bool:true