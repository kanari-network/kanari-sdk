# Kanari Cryptography

A TypeScript library for performing cryptographic operations for Kanari wallets in web environments.

## Features

- Generate wallets using K256 (secp256k1) or P256 (secp256r1) curves
- Import wallets from private keys or seed phrases
- Sign messages with private keys
- Verify message signatures
- Utility functions for working with cryptographic primitives

## Installation

```bash
npm install kanari-cryptography
```

## Usage

### Wallet Generation

```typescript
import { generateKanariAddress, CurveType } from 'kanari-cryptography';

// Generate a K256 (secp256k1) wallet with 12-word mnemonic
const k256Wallet = generateKanariAddress(12, CurveType.K256);

// Generate a P256 (secp256r1) wallet with 24-word mnemonic
const p256Wallet = generateKanariAddress(24, CurveType.P256);

console.log(k256Wallet);
/*
{
  privateKey: 'c8df9f0e1c038e0ef5e6eb94a8e261872ed10ce58a9a9f37ffd9f858348c0e8e',
  publicAddress: '0x79f35a8c75a3f6c4779f9715a333476b3df1a56ffd6cc4f20f12c4b47e37e0c5',
  seedPhrase: 'quantum sugar leader avocado sock rabbit bring great voice cricket camp sibling',
  curveType: 'K256'
}
*/
```

### Message Signing

```typescript
import { signMessage, CurveType } from 'kanari-cryptography';

// Sign a message with a K256 private key
const privateKey = 'c8df9f0e1c038e0ef5e6eb94a8e261872ed10ce58a9a9f37ffd9f858348c0e8e';
const message = 'Hello, Kanari!';
const signature = signMessage(privateKey, message, CurveType.K256);

console.log(signature);
// '3045022100c28316f4351f797246ed77c0146a4fb2b0d2d0e087d53e4b0534ee8067f043fc02203cd0d818c7e7cf226eff0bdfe7f0d2c53eb4c15be0ab9c661f12b3d1dbd38a61'
```

### Signature Verification

```typescript
import { verifySignature } from 'kanari-cryptography';

// Verify a signature
const address = '0x79f35a8c75a3f6c4779f9715a333476b3df1a56ffd6cc4f20f12c4b47e37e0c5';
const message = 'Hello, Kanari!';
const signature = '3045022100c28316f4351f797246ed77c0146a4fb2b0d2d0e087d53e4b0534ee8067f043fc02203cd0d818c7e7cf226eff0bdfe7f0d2c53eb4c15be0ab9c661f12b3d1dbd38a61';

const isValid = verifySignature(address, message, signature);
console.log(isValid); // true or false
```

### Importing Wallets

```typescript
import { importFromPrivateKey, importFromSeedPhrase, CurveType } from 'kanari-cryptography';

// Import from private key
const privateKey = 'c8df9f0e1c038e0ef5e6eb94a8e261872ed10ce58a9a9f37ffd9f858348c0e8e';
const wallet = importFromPrivateKey(privateKey, CurveType.K256);

// Import from seed phrase
const seedPhrase = 'quantum sugar leader avocado sock rabbit bring great voice cricket camp sibling';
const walletFromSeed = importFromSeedPhrase(seedPhrase, CurveType.K256);
```

## Security Notes

- Never expose private keys or seed phrases in client-side code
- Consider using a secure storage solution for sensitive data
- Always validate inputs before processing
