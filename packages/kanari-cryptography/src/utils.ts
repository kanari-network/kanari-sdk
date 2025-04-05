/**
 * Utility functions for cryptographic operations
 */

import { SHA3 } from 'sha3';
import { CurveType } from './types';

/**
 * Hash a message using SHA3-256
 * @param message The message to hash
 * @returns The message hash as a Buffer
 */
export function hashMessage(message: Uint8Array | string): Buffer {
  const hasher = new SHA3(256);
  
  if (typeof message === 'string') {
    hasher.update(Buffer.from(message, 'utf8'));
  } else {
    hasher.update(Buffer.from(message));
  }
  
  return hasher.digest();
}

/**
 * Convert a hex string to a buffer
 * @param hex The hex string to convert
 * @returns The resulting Buffer
 */
export function hexToBuffer(hex: string): Buffer {
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
export function bufferToHex(buffer: Buffer, with0x: boolean = false): string {
  const hex = buffer.toString('hex');
  return with0x ? `0x${hex}` : hex;
}

/**
 * Detect the curve type for an address
 * This is a best-effort detection and may not always be accurate
 * @param address The address to check
 * @returns The detected curve type, or null if unable to determine
 */
export function detectCurveType(address: string): CurveType | null {
  // In a web environment without direct curve point validation,
  // we would need to apply heuristics or maintain metadata
  // This is a simplified placeholder
  
  // For production usage, you might add curve point validation
  // or other methods to differentiate between curve types
  
  // Default to K256 for now
  return CurveType.K256;
}
