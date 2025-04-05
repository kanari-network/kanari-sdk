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

import { 
  generateKanariAddress, 
  importFromSeedPhrase,
  importFromPrivateKey,
  CurveType,
  signMessage,
  verifySignature
} from '../src';

/**
 * Create a K256 (secp256k1) wallet
 */
function createK256Wallet() {
  console.log('\n=== Create K256 (secp256k1) Wallet ===');
  
  // Create a new wallet with 12-word seed phrase
  const wallet = generateKanariAddress(12, CurveType.K256);
  
  console.log('📝 Seed Phrase (keep confidential):', wallet.seedPhrase);
  console.log('🔑 Private Key:', wallet.privateKey);
  console.log('📫 Public Address:', wallet.publicAddress);
  
  // Test message signing
  const message = 'Hello Kanari!';
  const signature = signMessage(wallet.privateKey, message, CurveType.K256);
  
  console.log('\n✍️  Message Signing Test:');
  console.log('Message:', message);
  console.log('Signature:', signature);
  
  // Test signature verification
  console.log('\n🔐 Testing signature verification (mock implementation):');
  const isSignatureValid = verifySignature(wallet.publicAddress, message, signature);
  console.log('Signature valid:', isSignatureValid ? '✅' : '❌');
  
  // Test with tampered message
  const tamperedMessage = message + ' (tampered)';
  console.log('\n⚠️ Note: Current verification is a mock implementation that returns true');
  console.log('A proper implementation would reject the tampered message.');
  const isTamperedValid = verifySignature(wallet.publicAddress, tamperedMessage, signature);
  console.log('Tampered message verification (should fail in production):', isTamperedValid ? '✅' : '❌');
  
  // Test importing wallet from seed phrase
  console.log('\n🔄 Testing import from seed phrase:');
  const importedFromSeed = importFromSeedPhrase(wallet.seedPhrase, CurveType.K256);
  console.log('Import successful:', importedFromSeed.publicAddress === wallet.publicAddress ? '✅' : '❌');
  
  return wallet;
}

/**
 * Create a P256 (secp256r1) wallet
 */
function createP256Wallet() {
  console.log('\n=== Create P256 (secp256r1) Wallet ===');
  
  // Create a new wallet with 24-word seed phrase
  const wallet = generateKanariAddress(24, CurveType.P256);
  
  console.log('📝 Seed Phrase (keep confidential):', wallet.seedPhrase);
  console.log('🔑 Private Key:', wallet.privateKey);
  console.log('📫 Public Address:', wallet.publicAddress);
  
  // Test message signing
  const message = 'Hello Kanari Network!';
  const signature = signMessage(wallet.privateKey, message, CurveType.P256);
  
  console.log('\n✍️  Message Signing Test:');
  console.log('Message:', message);
  console.log('Signature:', signature);
  
  // Test signature verification
  console.log('\n🔐 Testing signature verification:');
  const isSignatureValid = verifySignature(wallet.publicAddress, message, signature);
  console.log('Signature valid:', isSignatureValid ? '✅' : '❌');
  
  // Test with modified message
  const modifiedMessage = 'Modified ' + message;
  const isModifiedValid = verifySignature(wallet.publicAddress, modifiedMessage, signature);
  console.log('Modified message verification (should fail):', isModifiedValid ? '✅' : '❌');
  
  // Test importing wallet from private key
  console.log('\n🔄 Testing import from private key:');
  const importedFromKey = importFromPrivateKey(wallet.privateKey, CurveType.P256);
  console.log('Import successful:', importedFromKey.publicAddress === wallet.publicAddress ? '✅' : '❌');
  
  return wallet;
}

/**
 * Main function
 */
function main() {
  console.log('🚀 Starting Kanari Wallet Creation');
  
  const k256Wallet = createK256Wallet();
  const p256Wallet = createP256Wallet();
  
  console.log('\n\n📊 Wallet Creation Summary');
  console.log('K256 Public Address:', k256Wallet.publicAddress);
  console.log('P256 Public Address:', p256Wallet.publicAddress);
  
  console.log('\n⚠️  Warning: Keep your Private Key and Seed Phrase secure!');
  console.log('Do not share this information with others.');
}

// Run the program
main();
