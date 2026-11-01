package com.jamesatomc.kanariapp.network.models

private val defaultDeepbookDeployment = DeepbookDeployment(
    packageId = "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146",
    registryId = "0x950c7c86b5a425f2b2d7ce3e907fc5a64bf7b952dc516754b0bf7793db2d5b1b",
    nativeTokenType = "0x2::kanari::KANARI",
    // The published DeepBook package on these networks has the numeric
    // trade arguments in quantity/price order, even though newer source uses
    // price/quantity. Keep this tied to the deployed package ABI.
    tradeArgumentOrder = DeepbookTradeArgumentOrder.QUANTITY_THEN_PRICE,
    pairs = listOf(
        DeepbookPairConfig("0x2::kanari::KANARI", "0x2::usd::USD", 9, 6),
        DeepbookPairConfig("0x2::eth::ETH", "0x2::usd::USD", 18, 6)
    )
)

enum class KanariEnvironment(
    val url: String,
    val authUrl: String,
    val deepbook: DeepbookDeployment
) {
    local("http://10.0.2.2:6767/", "http://10.0.2.2:3000/", defaultDeepbookDeployment),
    dev("http://192.168.1.104:19001/", "http://10.0.2.2:3000/", defaultDeepbookDeployment);

    val rpcUrl: String get() = "${url.removeSuffix("/")}/rpc"
    val baseUrl: String get() = url
}

data class DeepbookPairConfig(
    val baseType: String,
    val quoteType: String,
    val baseDecimals: Int,
    val quoteDecimals: Int
)

data class DeepbookDeployment(
    val packageId: String,
    val registryId: String,
    val nativeTokenType: String,
    val tradeArgumentOrder: DeepbookTradeArgumentOrder,
    val pairs: List<DeepbookPairConfig>
)

enum class DeepbookTradeArgumentOrder {
    PRICE_THEN_QUANTITY,
    QUANTITY_THEN_PRICE
}
