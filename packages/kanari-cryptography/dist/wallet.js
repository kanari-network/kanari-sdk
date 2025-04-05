"use strict";
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
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.generateKanariAddress = generateKanariAddress;
exports.generateK256Address = generateK256Address;
exports.generateP256Address = generateP256Address;
exports.importFromPrivateKey = importFromPrivateKey;
exports.importFromPrivateKeyK256 = importFromPrivateKeyK256;
exports.importFromPrivateKeyP256 = importFromPrivateKeyP256;
exports.importFromSeedPhrase = importFromSeedPhrase;
/**
 * Wallet generation and management functions
 */
const elliptic_1 = require("elliptic");
const bip39 = __importStar(require("bip39"));
const types_1 = require("./types");
const utils_1 = require("./utils");
// Initialize elliptic curve instances
const secp256k1 = new elliptic_1.ec('secp256k1');
const p256 = new elliptic_1.ec('p256');
/**
 * Generate a Kanari address
 * @param wordCount Number of words in the seed phrase (12 or 24)
 * @param curveType The elliptic curve to use
 * @returns The generated wallet information
 */
function generateKanariAddress(wordCount = 12, curveType = types_1.CurveType.K256) {
    switch (curveType) {
        case types_1.CurveType.K256:
            return generateK256Address(wordCount);
        case types_1.CurveType.P256:
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
function generateK256Address(wordCount = 12) {
    // Generate mnemonic
    const strength = wordCount === 24 ? 256 : 128;
    const seedPhrase = bip39.generateMnemonic(strength);
    // Generate random keypair
    const keypair = secp256k1.genKeyPair();
    const privateKey = keypair.getPrivate('hex');
    // Get public key without the 0x04 prefix (uncompressed format)
    const publicKeyBuffer = Buffer.from(keypair.getPublic('array'));
    const publicKey = publicKeyBuffer.slice(1).toString('hex').substring(0, 64);
    // Format the address with 0x prefix
    const publicAddress = `0x${publicKey}`;
    return {
        privateKey,
        publicAddress,
        seedPhrase,
        curveType: types_1.CurveType.K256
    };
}
/**
 * Generate a P256 (secp256r1) wallet
 * @param wordCount Number of words in seed phrase
 * @returns The generated wallet information
 */
function generateP256Address(wordCount = 12) {
    // Generate mnemonic
    const strength = wordCount === 24 ? 256 : 128;
    const seedPhrase = bip39.generateMnemonic(strength);
    // Generate random keypair
    const keypair = p256.genKeyPair();
    const privateKey = keypair.getPrivate('hex');
    // Get public key without the 0x04 prefix (uncompressed format)
    const publicKeyBuffer = Buffer.from(keypair.getPublic('array'));
    const publicKey = publicKeyBuffer.slice(1).toString('hex').substring(0, 64);
    // Format the address with 0x prefix
    const publicAddress = `0x${publicKey}`;
    return {
        privateKey,
        publicAddress,
        seedPhrase,
        curveType: types_1.CurveType.P256
    };
}
/**
 * Import a wallet from a private key
 * @param privateKey The private key in hex format
 * @param curveType The elliptic curve type
 * @returns The imported wallet information
 */
function importFromPrivateKey(privateKey, curveType) {
    switch (curveType) {
        case types_1.CurveType.K256:
            return importFromPrivateKeyK256(privateKey);
        case types_1.CurveType.P256:
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
function importFromPrivateKeyK256(privateKey) {
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
        curveType: types_1.CurveType.K256
    };
}
/**
 * Import a P256 wallet from a private key
 * @param privateKey The private key in hex format
 * @returns The imported wallet information
 */
function importFromPrivateKeyP256(privateKey) {
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
        curveType: types_1.CurveType.P256
    };
}
/**
 * Import a wallet from a seed phrase
 * @param seedPhrase The BIP39 seed phrase
 * @param curveType The elliptic curve to use
 * @returns The imported wallet information
 */
function importFromSeedPhrase(seedPhrase, curveType) {
    // Validate seed phrase
    if (!bip39.validateMnemonic(seedPhrase)) {
        throw new Error('Invalid seed phrase');
    }
    // Generate seed from mnemonic
    const seed = bip39.mnemonicToSeedSync(seedPhrase);
    // Use first 32 bytes as private key material
    const privateKeyBytes = seed.slice(0, 32);
    const privateKey = (0, utils_1.bufferToHex)(privateKeyBytes);
    // Import using the private key function
    const wallet = importFromPrivateKey(privateKey, curveType);
    // Add the seed phrase to the result
    return Object.assign(Object.assign({}, wallet), { seedPhrase });
}
