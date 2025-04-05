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

import { hashMessage, hexToBuffer, bufferToHex, detectCurveType } from './utils';
import { CurveType } from './types';

describe('Cryptography Utilities', () => {
  describe('hashMessage', () => {
    it('should hash a string message', () => {
      const message = 'Hello, Kanari!';
      const hash = hashMessage(message);
      
      expect(hash).toBeInstanceOf(Buffer);
      expect(hash.length).toBe(32); // SHA-256 produces 32 bytes
    });
    
    it('should hash a byte array message', () => {
      const message = new TextEncoder().encode('Hello, Kanari!');
      const hash = hashMessage(message);
      
      expect(hash).toBeInstanceOf(Buffer);
      expect(hash.length).toBe(32);
    });
    
    it('should produce deterministic hashes', () => {
      const message = 'Hello, Kanari!';
      const hash1 = hashMessage(message);
      const hash2 = hashMessage(message);
      
      expect(hash1.toString('hex')).toBe(hash2.toString('hex'));
    });
    
    it('should produce different hashes for different messages', () => {
      const message1 = 'Hello, Kanari!';
      const message2 = 'Hello, Kanari.';
      const hash1 = hashMessage(message1);
      const hash2 = hashMessage(message2);
      
      expect(hash1.toString('hex')).not.toBe(hash2.toString('hex'));
    });
  });
  
  describe('hexToBuffer', () => {
    it('should convert a hex string to buffer', () => {
      const hex = 'deadbeef';
      const buffer = hexToBuffer(hex);
      
      expect(buffer).toBeInstanceOf(Buffer);
      expect(buffer.length).toBe(4);
      expect(buffer[0]).toBe(0xde);
      expect(buffer[1]).toBe(0xad);
      expect(buffer[2]).toBe(0xbe);
      expect(buffer[3]).toBe(0xef);
    });
    
    it('should handle 0x prefix', () => {
      const hex = '0xdeadbeef';
      const buffer = hexToBuffer(hex);
      
      expect(buffer).toBeInstanceOf(Buffer);
      expect(buffer.length).toBe(4);
      expect(buffer[0]).toBe(0xde);
    });
    
    it('should pad odd-length hex strings', () => {
      const hex = 'deadbee'; // 7 characters (odd)
      const buffer = hexToBuffer(hex);
      
      expect(buffer).toBeInstanceOf(Buffer);
      expect(buffer.length).toBe(4);
      expect(buffer[0]).toBe(0x0d); // Should be padded to '0deadbee'
    });
  });
  
  describe('bufferToHex', () => {
    it('should convert buffer to hex string without 0x prefix by default', () => {
      const buffer = Buffer.from([0xde, 0xad, 0xbe, 0xef]);
      const hex = bufferToHex(buffer);
      
      expect(hex).toBe('deadbeef');
    });
    
    it('should include 0x prefix when requested', () => {
      const buffer = Buffer.from([0xde, 0xad, 0xbe, 0xef]);
      const hex = bufferToHex(buffer, true);
      
      expect(hex).toBe('0xdeadbeef');
    });
  });
  
  describe('detectCurveType', () => {
    it('should return a curve type', () => {
      const address = '0x1234567890123456789012345678901234567890';
      const curveType = detectCurveType(address);
      
      // The current implementation returns K256 by default
      expect(curveType).toBe(CurveType.K256);
    });
  });
});
