/**
 * Utility functions for cryptographic operations
 */
import { CurveType } from './types';
/**
 * Hash a message using SHA3-256
 * @param message The message to hash
 * @returns The message hash as a Buffer
 */
export declare function hashMessage(message: Uint8Array | string): Buffer;
/**
 * Convert a hex string to a buffer
 * @param hex The hex string to convert
 * @returns The resulting Buffer
 */
export declare function hexToBuffer(hex: string): Buffer;
/**
 * Convert a buffer to a hex string
 * @param buffer The buffer to convert
 * @param with0x Whether to include 0x prefix
 * @returns The hex string
 */
export declare function bufferToHex(buffer: Buffer, with0x?: boolean): string;
/**
 * Detect the curve type for an address
 * This is a best-effort detection and may not always be accurate
 * @param address The address to check
 * @returns The detected curve type, or null if unable to determine
 */
export declare function detectCurveType(address: string): CurveType | null;
