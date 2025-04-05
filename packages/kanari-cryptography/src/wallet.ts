/**
 * Copyright 2024 Kanari Network™. Community.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

/**
 * Wallet generation and management functions
 */

import { ec as EC } from 'elliptic';
import * as bip39 from 'bip39';
import { CurveType, WalletResult } from './types';
import { bufferToHex } from './utils';

// Initialize elliptic curve instances
const secp256k1 = new EC('secp256k1');
const p256 = new EC('p256');

/**
 * Generate a Kanari address
 * @param wordCount Number of words in the seed phrase (12 or 24)
 * @param curveType The elliptic curve to use
 * @returns The generated wallet information
 */
export function generateKanariAddress(
  wordCount: 12 | 24 = 12,
  curveType: CurveType = CurveType.K256
): WalletResult {
  switch (curveType) {
    case CurveType.K256:
      return generateK256Address(wordCount);
    case CurveType.P256:
      return generateP256Address(wordCount);
    default:
      throw new Error(`Unsupported curve type: ${curveType}`);
  }
}

/**
 * Generate a K256 (secp256k1) wallet
 * @param wordCount Number of words in seed phrase
 * @returns The generated wallet information
 */
export function generateK256Address(wordCount: 12 | 24 = 12): WalletResult {
  // Generate mnemonic
  const strength = wordCount === 24 ? 256 : 128;
  const seedPhrase = bip39.generateMnemonic(strength);
  
  // Generate seed from mnemonic
  const seed = bip39.mnemonicToSeedSync(seedPhrase);
  
  // Use first 32 bytes of seed as private key material
  const privateKeyBytes = seed.slice(0, 32);
  const privateKey = bufferToHex(privateKeyBytes);
  
  // Create keypair from private key
  const keypair = secp256k1.keyFromPrivate(privateKeyBytes);
  
  // Get public key without the 0x04 prefix (uncompressed format)
  const publicKeyBuffer = Buffer.from(keypair.getPublic('array'));
  const publicKey = publicKeyBuffer.slice(1).toString('hex').substring(0, 64);
  
  // Format the address with 0x prefix
  const publicAddress = `0x${publicKey}`;
  
  return {
    privateKey,
    publicAddress,
    seedPhrase,
    curveType: CurveType.K256
  };
}

/**
 * Generate a P256 (secp256r1) wallet
 * @param wordCount Number of words in seed phrase
 * @returns The generated wallet information
 */
export function generateP256Address(wordCount: 12 | 24 = 12): WalletResult {
  // Generate mnemonic
  const strength = wordCount === 24 ? 256 : 128;
  const seedPhrase = bip39.generateMnemonic(strength);
  
  // Generate seed from mnemonic
  const seed = bip39.mnemonicToSeedSync(seedPhrase);
  
  // Use first 32 bytes of seed as private key material
  const privateKeyBytes = seed.slice(0, 32);
  const privateKey = bufferToHex(privateKeyBytes);
  
  // Create keypair from private key
  const keypair = p256.keyFromPrivate(privateKeyBytes);
  
  // Get public key without the 0x04 prefix (uncompressed format)
  const publicKeyBuffer = Buffer.from(keypair.getPublic('array'));
  const publicKey = publicKeyBuffer.slice(1).toString('hex').substring(0, 64);
  
  // Format the address with 0x prefix
  const publicAddress = `0x${publicKey}`;
  
  return {
    privateKey,
    publicAddress,
    seedPhrase,
    curveType: CurveType.P256
  };
}

/**
 * Import a wallet from a private key
 * @param privateKey The private key in hex format
 * @param curveType The elliptic curve type
 * @returns The imported wallet information
 */
export function importFromPrivateKey(
  privateKey: string,
  curveType: CurveType
): WalletResult {
  switch (curveType) {
    case CurveType.K256:
      return importFromPrivateKeyK256(privateKey);
    case CurveType.P256:
      return importFromPrivateKeyP256(privateKey);
    default:
      throw new Error(`Unsupported curve type: ${curveType}`);
  }
}

/**
 * Import a K256 wallet from a private key
 * @param privateKey The private key in hex format
 * @returns The imported wallet information
 */
export function importFromPrivateKeyK256(privateKey: string): WalletResult {
  // Remove 0x prefix if present
  const cleanKey = privateKey.startsWith('0x') ? privateKey.slice(2) : privateKey;
  
  // Create keypair from private key
  const keypair = secp256k1.keyFromPrivate(cleanKey, 'hex');
  
  // Get public key without the 0x04 prefix (uncompressed format)
  const publicKeyBuffer = Buffer.from(keypair.getPublic('array'));
  const publicKey = publicKeyBuffer.slice(1).toString('hex').substring(0, 64);
  
  // Format the address with 0x prefix
  const publicAddress = `0x${publicKey}`;
  
  return {
    privateKey: cleanKey,
    publicAddress,
    seedPhrase: '', // No seed phrase when importing from private key
    curveType: CurveType.K256
  };
}

/**
 * Import a P256 wallet from a private key
 * @param privateKey The private key in hex format
 * @returns The imported wallet information
 */
export function importFromPrivateKeyP256(privateKey: string): WalletResult {
  // Remove 0x prefix if present
  const cleanKey = privateKey.startsWith('0x') ? privateKey.slice(2) : privateKey;
  
  // Create keypair from private key
  const keypair = p256.keyFromPrivate(cleanKey, 'hex');
  
  // Get public key without the 0x04 prefix (uncompressed format)
  const publicKeyBuffer = Buffer.from(keypair.getPublic('array'));
  const publicKey = publicKeyBuffer.slice(1).toString('hex').substring(0, 64);
  
  // Format the address with 0x prefix
  const publicAddress = `0x${publicKey}`;
  
  return {
    privateKey: cleanKey,
    publicAddress,
    seedPhrase: '', // No seed phrase when importing from private key
    curveType: CurveType.P256
  };
}

/**
 * Import a wallet from a seed phrase
 * @param seedPhrase The BIP39 seed phrase
 * @param curveType The elliptic curve to use
 * @returns The imported wallet information
 */
export function importFromSeedPhrase(
  seedPhrase: string,
  curveType: CurveType
): WalletResult {
  // Validate seed phrase
  if (!bip39.validateMnemonic(seedPhrase)) {
    throw new Error('Invalid seed phrase');
  }
  
  // Generate seed from mnemonic
  const seed = bip39.mnemonicToSeedSync(seedPhrase);
  
  // Use first 32 bytes as private key material
  const privateKeyBytes = seed.slice(0, 32);
  const privateKey = bufferToHex(privateKeyBytes);
  
  // Import using the private key function
  const wallet = importFromPrivateKey(privateKey, curveType);
  
  // Add the seed phrase to the result
  return {
    ...wallet,
    seedPhrase
  };
}
