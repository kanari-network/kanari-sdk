package com.jamesatomc.kanariapp.network

import kotlinx.serialization.json.JsonObjectBuilder
import kotlinx.serialization.json.put

internal const val DEFAULT_GAS_PRICE: ULong = 1uL
internal const val DEFAULT_GAS_LIMIT: ULong = 1_000uL

data class GasSettings(
    val limit: ULong = DEFAULT_GAS_LIMIT,
    val price: ULong = DEFAULT_GAS_PRICE,
)

internal val DEFAULT_GAS_SETTINGS = GasSettings()

// Native KANARI settles at the protocol unit price, not the USD gas-market quote.
internal val DEFAULT_GAS_RESERVE_MIST: ULong = DEFAULT_GAS_LIMIT * DEFAULT_GAS_PRICE
// USD uses 6 decimals and is priced at $1, so one gas unit maps to one
// micro-USD base unit under the default gas table.
internal val DEFAULT_GAS_RESERVE_USD: ULong = DEFAULT_GAS_LIMIT

internal fun JsonObjectBuilder.putGasSettings(gas: GasSettings = DEFAULT_GAS_SETTINGS) {
    put("gas_limit", gas.limit.toLong())
    put("gas_price", gas.price.toLong())
}
