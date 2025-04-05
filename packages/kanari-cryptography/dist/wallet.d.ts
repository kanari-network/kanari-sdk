/**
 * Wallet generation and management functions
 */
import { CurveType, WalletResult } from './types';
/**
 * Generate a Kanari address
 * @param wordCount Number of words in the seed phrase (12 or 24)
 * @param curveType The elliptic curve to use
 * @returns The generated wallet information
 */
export declare function generateKanariAddress(wordCount?: 12 | 24, curveType?: CurveType): WalletResult;
/**
 * Generate a K256 (secp256k1) wallet
 * @param wordCount Number of words in seed phrase
 * @returns The generated wallet information
 */
export declare function generateK256Address(wordCount?: 12 | 24): WalletResult;
/**
 * Generate a P256 (secp256r1) wallet
 * @param wordCount Number of words in seed phrase
 * @returns The generated wallet information
 */
export declare function generateP256Address(wordCount?: 12 | 24): WalletResult;
/**
 * Import a wallet from a private key
 * @param privateKey The private key in hex format
 * @param curveType The elliptic curve type
 * @returns The imported wallet information
 */
export declare function importFromPrivateKey(privateKey: string, curveType: CurveType): WalletResult;
/**
 * Import a K256 wallet from a private key
 * @param privateKey The private key in hex format
 * @returns The imported wallet information
 */
export declare function importFromPrivateKeyK256(privateKey: string): WalletResult;
/**
 * Import a P256 wallet from a private key
 * @param privateKey The private key in hex format
 * @returns The imported wallet information
 */
export declare function importFromPrivateKeyP256(privateKey: string): WalletResult;
/**
 * Import a wallet from a seed phrase
 * @param seedPhrase The BIP39 seed phrase
 * @param curveType The elliptic curve to use
 * @returns The imported wallet information
 */
export declare function importFromSeedPhrase(seedPhrase: string, curveType: CurveType): WalletResult;
