
<a name="@Kanari_System_Modules_0"></a>

# Kanari System Modules


This is the root document for the Kanari System module documentation. The Kanari System provides Move modules for blockchain operations including transfers, token management, and system utilities.


<a name="@Overview_1"></a>

## Overview


The Kanari System is built on top of the Move programming language, providing a secure and efficient platform for decentralized applications. This documentation covers all modules available in the Kanari System.


<a name="@Features_2"></a>

## Features


- **Transfer Module**: Production-ready transfer functionality with full validation
- **Type Safety**: Strong type checking and validation
- **Security**: Built-in error handling and validation
- **Performance**: Optimized for efficient execution


<a name="@Index_3"></a>

## Index


-  [`0x2::balance`](balance.md#0x2_balance)
-  [`0x2::coin`](coin.md#0x2_coin)
-  [`0x2::ecdsa_k1`](ecdsa_k1.md#0x2_ecdsa_k1)
-  [`0x2::ecdsa_r1`](ecdsa_r1.md#0x2_ecdsa_r1)
-  [`0x2::ed25519`](ed25519.md#0x2_ed25519)
-  [`0x2::kanari`](kanari.md#0x2_kanari)
-  [`0x2::object`](object.md#0x2_object)
-  [`0x2::transfer`](transfer.md#0x2_transfer)
-  [`0x2::tx_context`](tx_context.md#0x2_tx_context)
-  [`0x2::url`](url.md#0x2_url)



<a name="@Getting_Started_4"></a>

## Getting Started


To use Kanari System modules in your Move code, add the following to your <code>Move.toml</code>:

```toml
[dependencies]
KanariSystem = { local = "../kanari-system" }
```

Then import the modules you need:

```move
use kanari_system::transfer;
```


<a name="@Support_5"></a>

## Support


For issues and questions, please visit our GitHub repository.


[//]: # ("File containing references which can be used from documentation")

[Move Language]: https://github.com/move-language/move
[Kanari]: https://github.com/jamesatomc/kanari-cp
[Move Book]: https://move-language.github.io/move/
[Transfer Module]: transfer.md
