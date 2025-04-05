/**
 * Common types used throughout the library
 */
/**
 * Supported elliptic curve types
 */
export declare enum CurveType {
    K256 = "K256",// secp256k1
    P256 = "P256"
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
