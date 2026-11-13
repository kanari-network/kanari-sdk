package com.jamesatomc.kanariapp.network.models

private val defaultDeepbookDeployment = DeepbookDeployment(
    packageId = "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146",
    registryId = "0x1a705fb22b3207db4c25873a7db45e2eb0f4b2d11eea919f33dc51bbd662837e",
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

private val defaultEscrowDeployment = EscrowDeployment(
    packageId = "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146"
)

enum class KanariEnvironment(
    val url: String,
    val authUrl: String,
    val deepbook: DeepbookDeployment,
    val escrow: EscrowDeployment
) {
    local("http://10.0.2.2:6767/", "http://10.0.2.2:3000/", defaultDeepbookDeployment, defaultEscrowDeployment),
    dev("http://192.168.1.100:19001/", "http://10.0.2.2:3000/", defaultDeepbookDeployment, defaultEscrowDeployment);

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

data class EscrowDeployment(
    val packageId: String
)

enum class DeepbookTradeArgumentOrder {
    PRICE_THEN_QUANTITY,
    QUANTITY_THEN_PRICE
}
