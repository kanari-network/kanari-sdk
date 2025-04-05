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
exports.hashMessage = hashMessage;
exports.hexToBuffer = hexToBuffer;
exports.bufferToHex = bufferToHex;
exports.detectCurveType = detectCurveType;
/**
 * Utility functions for cryptographic operations
 */
const sha3_1 = require("sha3");
const types_1 = require("./types");
/**
 * Hash a message using SHA3-256
 * @param message The message to hash
 * @returns The message hash as a Buffer
 */
function hashMessage(message) {
    const hasher = new sha3_1.SHA3(256);
    if (typeof message === 'string') {
        hasher.update(Buffer.from(message, 'utf8'));
    }
    else {
        hasher.update(Buffer.from(message));
    }
    return hasher.digest();
}
/**
 * Convert a hex string to a buffer
 * @param hex The hex string to convert
 * @returns The resulting Buffer
 */
function hexToBuffer(hex) {
    // Remove 0x prefix if present
    const cleanHex = hex.startsWith('0x') ? hex.slice(2) : hex;
    // Ensure even length
    const paddedHex = cleanHex.length % 2 === 0 ? cleanHex : `0${cleanHex}`;
    return Buffer.from(paddedHex, 'hex');
}
/**
 * Convert a buffer to a hex string
 * @param buffer The buffer to convert
 * @param with0x Whether to include 0x prefix
 * @returns The hex string
 */
function bufferToHex(buffer, with0x = false) {
    const hex = buffer.toString('hex');
    return with0x ? `0x${hex}` : hex;
}
/**
 * Detect the curve type for an address
 * This is a best-effort detection and may not always be accurate
 * @param address The address to check
 * @returns The detected curve type, or null if unable to determine
 */
function detectCurveType(address) {
    // In a web environment without direct curve point validation,
    // we would need to apply heuristics or maintain metadata
    // This is a simplified placeholder
    // For production usage, you might add curve point validation
    // or other methods to differentiate between curve types
    // Default to K256 for now
    return types_1.CurveType.K256;
}
