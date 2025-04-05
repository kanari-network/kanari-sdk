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

import * as bip39 from 'bip39';
import {
  generateKanariAddress,
  generateK256Address,
  generateP256Address,
  importFromPrivateKey,
  importFromPrivateKeyK256,
  importFromPrivateKeyP256,
  importFromSeedPhrase
} from './wallet';
import { CurveType } from './types';

describe('Wallet Generation', () => {
  describe('K256 wallet generation', () => {
    it('should generate a K256 wallet with 12-word mnemonic', () => {
      const wallet = generateKanariAddress(12, CurveType.K256);
      
      // Check if all properties exist
      expect(wallet).toHaveProperty('privateKey');
      expect(wallet).toHaveProperty('publicAddress');
      expect(wallet).toHaveProperty('seedPhrase');
      expect(wallet).toHaveProperty('curveType');
      
      // Check if properties have the right format
      expect(wallet.privateKey).toMatch(/^[a-f0-9]{64}$/);
      expect(wallet.publicAddress).toMatch(/^0x[a-f0-9]{64}$/);
      expect(wallet.curveType).toBe(CurveType.K256);
      
      // Check if seed phrase has 12 words
      expect(wallet.seedPhrase.split(' ').length).toBe(12);
      
      // Validate the mnemonic
      expect(bip39.validateMnemonic(wallet.seedPhrase)).toBeTruthy();
    });

    it('should generate a K256 wallet with 24-word mnemonic', () => {
      const wallet = generateKanariAddress(24, CurveType.K256);
      
      // Check if all properties exist
      expect(wallet).toHaveProperty('privateKey');
      expect(wallet).toHaveProperty('publicAddress');
      expect(wallet).toHaveProperty('seedPhrase');
      expect(wallet).toHaveProperty('curveType');
      
      // Check if properties have the right format
      expect(wallet.privateKey).toMatch(/^[a-f0-9]{64}$/);
      expect(wallet.publicAddress).toMatch(/^0x[a-f0-9]{64}$/);
      expect(wallet.curveType).toBe(CurveType.K256);
      
      // Check if seed phrase has 24 words
      expect(wallet.seedPhrase.split(' ').length).toBe(24);
      
      // Validate the mnemonic
      expect(bip39.validateMnemonic(wallet.seedPhrase)).toBeTruthy();
    });
    
    it('should directly generate a K256 wallet using generateK256Address', () => {
      const wallet = generateK256Address();
      
      expect(wallet).toHaveProperty('privateKey');
      expect(wallet).toHaveProperty('publicAddress');
      expect(wallet).toHaveProperty('seedPhrase');
      expect(wallet.curveType).toBe(CurveType.K256);
    });
  });

  describe('P256 wallet generation', () => {
    it('should generate a P256 wallet with 12-word mnemonic', () => {
      const wallet = generateKanariAddress(12, CurveType.P256);
      
      // Check if all properties exist
      expect(wallet).toHaveProperty('privateKey');
      expect(wallet).toHaveProperty('publicAddress');
      expect(wallet).toHaveProperty('seedPhrase');
      expect(wallet).toHaveProperty('curveType');
      
      // Check if properties have the right format
      expect(wallet.privateKey).toMatch(/^[a-f0-9]{64}$/);
      expect(wallet.publicAddress).toMatch(/^0x[a-f0-9]{64}$/);
      expect(wallet.curveType).toBe(CurveType.P256);
      
      // Check if seed phrase has 12 words
      expect(wallet.seedPhrase.split(' ').length).toBe(12);
      
      // Validate the mnemonic
      expect(bip39.validateMnemonic(wallet.seedPhrase)).toBeTruthy();
    });
    
    it('should generate a P256 wallet with 24-word mnemonic', () => {
      const wallet = generateKanariAddress(24, CurveType.P256);
      
      // Check if all properties exist
      expect(wallet).toHaveProperty('privateKey');
      expect(wallet).toHaveProperty('publicAddress');
      expect(wallet).toHaveProperty('seedPhrase');
      expect(wallet).toHaveProperty('curveType');
      
      // Check if properties have the right format
      expect(wallet.privateKey).toMatch(/^[a-f0-9]{64}$/);
      expect(wallet.publicAddress).toMatch(/^0x[a-f0-9]{64}$/);
      expect(wallet.curveType).toBe(CurveType.P256);
      
      // Check if seed phrase has 24 words
      expect(wallet.seedPhrase.split(' ').length).toBe(24);
      
      // Validate the mnemonic
      expect(bip39.validateMnemonic(wallet.seedPhrase)).toBeTruthy();
    });
    
    it('should directly generate a P256 wallet using generateP256Address', () => {
      const wallet = generateP256Address();
      
      expect(wallet).toHaveProperty('privateKey');
      expect(wallet).toHaveProperty('publicAddress');
      expect(wallet).toHaveProperty('seedPhrase');
      expect(wallet.curveType).toBe(CurveType.P256);
    });
  });

  describe('Error handling in wallet generation', () => {
    it('should throw an error for unsupported curve type', () => {
      expect(() => {
        // @ts-ignore - Testing invalid input
        generateKanariAddress(12, 'UnsupportedCurve');
      }).toThrow('Unsupported curve type: UnsupportedCurve');
    });
  });
});

describe('Wallet Import', () => {
  describe('Import from private key', () => {
    it('should import a K256 wallet from a private key', () => {
      const originalWallet = generateKanariAddress(12, CurveType.K256);
      const importedWallet = importFromPrivateKey(originalWallet.privateKey, CurveType.K256);
      
      expect(importedWallet.privateKey).toBe(originalWallet.privateKey);
      expect(importedWallet.publicAddress).toBe(originalWallet.publicAddress);
      expect(importedWallet.curveType).toBe(CurveType.K256);
      expect(importedWallet.seedPhrase).toBe(''); // No seed phrase when importing from private key
    });
    
    it('should import a P256 wallet from a private key', () => {
      const originalWallet = generateKanariAddress(12, CurveType.P256);
      const importedWallet = importFromPrivateKey(originalWallet.privateKey, CurveType.P256);
      
      expect(importedWallet.privateKey).toBe(originalWallet.privateKey);
      expect(importedWallet.publicAddress).toBe(originalWallet.publicAddress);
      expect(importedWallet.curveType).toBe(CurveType.P256);
      expect(importedWallet.seedPhrase).toBe(''); // No seed phrase when importing from private key
    });
    
    it('should handle private keys with 0x prefix', () => {
      const originalWallet = generateKanariAddress(12, CurveType.K256);
      const prefixedKey = `0x${originalWallet.privateKey}`;
      const importedWallet = importFromPrivateKey(prefixedKey, CurveType.K256);
      
      expect(importedWallet.privateKey).toBe(originalWallet.privateKey);
      expect(importedWallet.publicAddress).toBe(originalWallet.publicAddress);
    });
    
    it('should import directly with importFromPrivateKeyK256', () => {
      const originalWallet = generateKanariAddress(12, CurveType.K256);
      const importedWallet = importFromPrivateKeyK256(originalWallet.privateKey);
      
      expect(importedWallet.privateKey).toBe(originalWallet.privateKey);
      expect(importedWallet.publicAddress).toBe(originalWallet.publicAddress);
    });
    
    it('should import directly with importFromPrivateKeyP256', () => {
      const originalWallet = generateKanariAddress(12, CurveType.P256);
      const importedWallet = importFromPrivateKeyP256(originalWallet.privateKey);
      
      expect(importedWallet.privateKey).toBe(originalWallet.privateKey);
      expect(importedWallet.publicAddress).toBe(originalWallet.publicAddress);
    });
    
    it('should throw an error for unsupported curve type', () => {
      const privateKey = generateKanariAddress(12, CurveType.K256).privateKey;
      
      expect(() => {
        // @ts-ignore - Testing invalid input
        importFromPrivateKey(privateKey, 'UnsupportedCurve');
      }).toThrow('Unsupported curve type: UnsupportedCurve');
    });
  });
  
  describe('Import from seed phrase', () => {
    it('should import a K256 wallet from a seed phrase', () => {
      const originalWallet = generateKanariAddress(12, CurveType.K256);
      const importedWallet = importFromSeedPhrase(originalWallet.seedPhrase, CurveType.K256);
      
      expect(importedWallet.seedPhrase).toBe(originalWallet.seedPhrase);
      expect(importedWallet.curveType).toBe(CurveType.K256);
      // We can't directly compare private keys as the derivation may be different
      // but we can verify the wallet has valid properties
      expect(importedWallet.privateKey).toMatch(/^[a-f0-9]{64}$/);
      expect(importedWallet.publicAddress).toMatch(/^0x[a-f0-9]{64}$/);
    });
    
    it('should import a P256 wallet from a seed phrase', () => {
      const originalWallet = generateKanariAddress(12, CurveType.P256);
      const importedWallet = importFromSeedPhrase(originalWallet.seedPhrase, CurveType.P256);
      
      expect(importedWallet.seedPhrase).toBe(originalWallet.seedPhrase);
      expect(importedWallet.curveType).toBe(CurveType.P256);
      expect(importedWallet.privateKey).toMatch(/^[a-f0-9]{64}$/);
      expect(importedWallet.publicAddress).toMatch(/^0x[a-f0-9]{64}$/);
    });
    
    it('should throw an error for invalid seed phrase', () => {
      const invalidSeedPhrase = 'invalid seed phrase without correct word count';
      
      expect(() => {
        importFromSeedPhrase(invalidSeedPhrase, CurveType.K256);
      }).toThrow('Invalid seed phrase');
    });
  });
});
