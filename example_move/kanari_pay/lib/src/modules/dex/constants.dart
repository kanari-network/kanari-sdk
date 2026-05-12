// modules/dex/constants.dart
/// DEX V1 constants - Package addresses and function names

class DexConstants {
  const DexConstants._();

  // Package address (update after deployment)
  static const String packageAddress = '0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146';
  
  // Module name
  static const String dexModule = 'dex_v1';
  
  // Entry functions (transactions)
  static const String fnCreatePool = 'create_pool';
  static const String fnAddLiquidity = 'add_liquidity';
  static const String fnRemoveLiquidity = 'remove_liquidity';
  static const String fnSwapAForB = 'swap_a_for_b';
  static const String fnSwapBForA = 'swap_b_for_a';
  
  // View functions (queries)
  static const String fnGetPoolInfo = 'get_pool_info';
  static const String fnGetPoolId = 'get_pool_id';
  static const String fnGetReserveA = 'get_reserve_a';
  static const String fnGetReserveB = 'get_reserve_b';
  static const String fnGetLpSupply = 'get_lp_supply';
  static const String fnGetFeePercent = 'get_fee_percent';
  static const String fnGetSwapAForBOutput = 'get_swap_a_for_b_output';
  static const String fnGetSwapBForAOutput = 'get_swap_b_for_a_output';
  
  // Object types
  static const String poolType = 'Pool';
  static const String lpTokenTypePrefix = 'LP_TOKEN';
}
