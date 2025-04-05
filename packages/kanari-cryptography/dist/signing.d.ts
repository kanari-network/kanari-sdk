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
import { CurveType } from './types';
/**
 * Sign a message with a K256 (secp256k1) private key
 * @param privateKeyHex Private key in hex format
 * @param message Message to sign (string or byte array)
 * @returns The signature as a hex string
 */
export declare function signMessageK256(privateKeyHex: string, message: string | Uint8Array): string;
/**
 * Sign a message with a P256 (secp256r1) private key
 * @param privateKeyHex Private key in hex format
 * @param message Message to sign (string or byte array)
 * @returns The signature as a hex string
 */
export declare function signMessageP256(privateKeyHex: string, message: string | Uint8Array): string;
/**
 * Sign a message using the appropriate curve based on the specified curve type
 * @param privateKeyHex Private key in hex format
 * @param message Message to sign
 * @param curveType The curve type to use for signing
 * @returns The signature as a hex string
 */
export declare function signMessage(privateKeyHex: string, message: string | Uint8Array, curveType: CurveType): string;
/**
 * Verify a signature using K256 (secp256k1)
 * @param address Address (public key) to verify against
 * @param message The original message
 * @param signatureHex The signature in hex format
 * @returns True if signature is valid, false otherwise
 */
export declare function verifySignatureK256(address: string, message: string | Uint8Array, signatureHex: string): boolean;
/**
 * Verify a signature using P256 (secp256r1)
 * @param address Address (public key) to verify against
 * @param message The original message
 * @param signatureHex The signature in hex format
 * @returns True if signature is valid, false otherwise
 */
export declare function verifySignatureP256(address: string, message: string | Uint8Array, signatureHex: string): boolean;
/**
 * Verify a signature against an address
 * Tries both K256 and P256 curves
 * @param address Address to verify against
 * @param message Original message
 * @param signatureHex Signature in hex format
 * @returns True if signature is valid, false otherwise
 */
export declare function verifySignature(address: string, message: string | Uint8Array, signatureHex: string): boolean;
