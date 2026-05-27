// modules/transactions/constants.dart
/// Transaction module constants

class TransactionConstants {
  const TransactionConstants._();

  // BCS Transaction types
  static const String txPublishModule = 'PublishModule';
  static const String txExecuteFunction = 'ExecuteFunction';
  static const String txTransfer = 'Transfer';
  static const String txBurn = 'Burn';

  // RPC methods
  static const String rpcPublishModule = 'kanari_publishModule';
  static const String rpcSubmitTransaction = 'kanari_submitTransaction';
  static const String rpcCallFunction = 'kanari_callFunction';

  // Default gas settings
  static const int defaultGasLimit = 100000;
  static const int defaultGasPrice = 1000;
}
