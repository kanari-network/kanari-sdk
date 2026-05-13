// modules/modules.dart
/// Modules barrel file - Central export point for all modules
///
/// To use a module:
/// ```dart
/// import 'package:kanari_kit/src/modules/modules.dart';
///
/// // Access module classes
/// final tokenOps = TokenOperations(url, queries, client);
/// final nftQueries = NftQueries(url, baseQueries, client);
/// ```

// Core modules (implemented)
export 'transactions/transactions.dart';
export 'queries.dart';

// Feature modules (templates - ready for implementation)


// Future modules (uncomment when implemented)
// export 'governance/governance.dart';
// export 'staking/staking.dart';
// export 'identity/identity.dart';
// export 'oracle/oracle.dart';
