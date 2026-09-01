package com.jamesatomc.kanariapp.network.models

enum class KanariEnvironment(
    val baseUrl: String,
    val authUrl: String
) {
    LOCAL("http://10.0.2.2:6767", "http://10.0.2.2:3000"), // Use 10.0.2.2 for Android emulator to localhost
    TESTNET("https://testnet.kanari.network", "https://auth.testnet.kanari.network"),
    MAINNET("https://mainnet.kanari.network", "https://auth.mainnet.kanari.network");

    val rpcUrl: String get() = "$baseUrl/rpc"
}
