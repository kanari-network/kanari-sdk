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

/**
 * Common types used throughout the library
 */

/**
 * Supported elliptic curve types
 */
export enum CurveType {
  K256 = 'K256', // secp256k1
  P256 = 'P256'  // secp256r1/prime256v1
}

/**
 * Wallet data structure
 */
export interface Wallet {
  address: string;
  privateKey: string;
  seedPhrase?: string;
  curveType: CurveType;
}

/**
 * Result of wallet generation/import operations
 */
export interface WalletResult {
  privateKey: string;
  publicAddress: string;
  seedPhrase: string;
  curveType: CurveType;
}
