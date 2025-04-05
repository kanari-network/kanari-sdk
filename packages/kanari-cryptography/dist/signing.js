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
Object.defineProperty(exports, "__esModule", { value: true });
exports.signMessageK256 = signMessageK256;
exports.signMessageP256 = signMessageP256;
exports.signMessage = signMessage;
exports.verifySignatureK256 = verifySignatureK256;
exports.verifySignatureP256 = verifySignatureP256;
exports.verifySignature = verifySignature;
/**
 * Message signing and verification functions
 */
const elliptic_1 = require("elliptic");
const types_1 = require("./types");
const utils_1 = require("./utils");
// Initialize elliptic curve instances
const secp256k1 = new elliptic_1.ec('secp256k1');
const p256 = new elliptic_1.ec('p256');
/**
 * Sign a message with a K256 (secp256k1) private key
 * @param privateKeyHex Private key in hex format
 * @param message Message to sign (string or byte array)
 * @returns The signature as a hex string
 */
function signMessageK256(privateKeyHex, message) {
    // Hash the message with SHA3-256
    const messageHash = (0, utils_1.hashMessage)(message);
    // Clean up private key (remove 0x if present)
    const cleanKey = privateKeyHex.startsWith('0x') ? privateKeyHex.slice(2) : privateKeyHex;
    // Create key from private key
    const key = secp256k1.keyFromPrivate(cleanKey, 'hex');
    // Sign the message hash
    // The signature is generated deterministically according to RFC 6979
    const signature = key.sign(messageHash);
    // Convert to DER format
    const derSignature = signature.toDER('hex');
    return derSignature;
}
/**
 * Sign a message with a P256 (secp256r1) private key
 * @param privateKeyHex Private key in hex format
 * @param message Message to sign (string or byte array)
 * @returns The signature as a hex string
 */
function signMessageP256(privateKeyHex, message) {
    // Hash the message with SHA3-256
    const messageHash = (0, utils_1.hashMessage)(message);
    // Clean up private key (remove 0x if present)
    const cleanKey = privateKeyHex.startsWith('0x') ? privateKeyHex.slice(2) : privateKeyHex;
    // Create key from private key
    const key = p256.keyFromPrivate(cleanKey, 'hex');
    // Sign the message hash
    // The signature is generated deterministically according to RFC 6979
    const signature = key.sign(messageHash);
    // Convert to DER format
    const derSignature = signature.toDER('hex');
    return derSignature;
}
/**
 * Sign a message using the appropriate curve based on the specified curve type
 * @param privateKeyHex Private key in hex format
 * @param message Message to sign
 * @param curveType The curve type to use for signing
 * @returns The signature as a hex string
 */
function signMessage(privateKeyHex, message, curveType) {
    switch (curveType) {
        case types_1.CurveType.K256:
            return signMessageK256(privateKeyHex, message);
        case types_1.CurveType.P256:
            return signMessageP256(privateKeyHex, message);
        default:
            throw new Error(`Unsupported curve type: ${curveType}`);
    }
}
/**
 * Verify a signature using K256 (secp256k1)
 * @param address Address (public key) to verify against
 * @param message The original message
 * @param signatureHex The signature in hex format
 * @returns True if signature is valid, false otherwise
 */
function verifySignatureK256(address, message, signatureHex) {
    try {
        // Hash the message with SHA3-256
        const messageHash = (0, utils_1.hashMessage)(message);
        // Remove 0x prefix from address if present
        const addressHex = address.startsWith('0x') ? address.slice(2) : address;
        // Convert signature from hex to buffer if needed
        const signatureBytes = typeof signatureHex === 'string'
            ? (0, utils_1.hexToBuffer)(signatureHex)
            : Buffer.from(signatureHex);
        // Create key pair from the given public key coordinates
        // We need to properly format the public key for the elliptic library
        // By convention, uncompressed public keys start with 0x04 followed by x and y coordinates
        const fullPublicKeyHex = `04${addressHex}`;
        try {
            const key = secp256k1.keyFromPublic(fullPublicKeyHex, 'hex');
            // Verify the signature
            return key.verify(messageHash, signatureBytes);
        }
        catch (error) {
            // If we can't decode the public key directly, try another approach
            // Get the public key from signature and message and compare
            const key = secp256k1.keyFromPublic(fullPublicKeyHex, 'hex');
            return key.verify(messageHash, signatureHex);
        }
    }
    catch (error) {
        console.error('K256 verification error:', error);
        return false;
    }
}
/**
 * Verify a signature using P256 (secp256r1)
 * @param address Address (public key) to verify against
 * @param message The original message
 * @param signatureHex The signature in hex format
 * @returns True if signature is valid, false otherwise
 */
function verifySignatureP256(address, message, signatureHex) {
    try {
        // Hash the message with SHA3-256
        const messageHash = (0, utils_1.hashMessage)(message);
        // Remove 0x prefix from address if present
        const addressHex = address.startsWith('0x') ? address.slice(2) : address;
        // Convert signature from hex to buffer if needed
        const signatureBytes = typeof signatureHex === 'string'
            ? (0, utils_1.hexToBuffer)(signatureHex)
            : Buffer.from(signatureHex);
        // Create key pair from the given public key coordinates
        // We need to properly format the public key for the elliptic library
        // By convention, uncompressed public keys start with 0x04 followed by x and y coordinates
        const fullPublicKeyHex = `04${addressHex}`;
        try {
            const key = p256.keyFromPublic(fullPublicKeyHex, 'hex');
            // Verify the signature
            return key.verify(messageHash, signatureBytes);
        }
        catch (error) {
            // If we can't decode the public key directly, try another approach
            // Get the public key from signature and message and compare
            const key = p256.keyFromPublic(fullPublicKeyHex, 'hex');
            return key.verify(messageHash, signatureHex);
        }
    }
    catch (error) {
        console.error('P256 verification error:', error);
        return false;
    }
}
/**
 * Verify a signature against an address
 * Tries both K256 and P256 curves
 * @param address Address to verify against
 * @param message Original message
 * @param signatureHex Signature in hex format
 * @returns True if signature is valid, false otherwise
 */
function verifySignature(address, message, signatureHex) {
    // Try both curves and return true if either succeeds
    const k256Result = verifySignatureK256(address, message, signatureHex);
    if (k256Result) {
        return true;
    }
    const p256Result = verifySignatureP256(address, message, signatureHex);
    return p256Result;
}
