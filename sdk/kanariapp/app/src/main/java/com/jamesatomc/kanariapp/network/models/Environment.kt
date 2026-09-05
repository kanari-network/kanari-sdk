package com.jamesatomc.kanariapp.network.models

enum class KanariEnvironment(
    val url: String,
    val authUrl: String
) {
    local("http://10.0.2.2:6767/", "http://10.0.2.2:3000/"), 
    dev("http://192.168.1.102:19001/", "http://10.0.2.2:3000/");

    val rpcUrl: String get() = "${url.removeSuffix("/")}/rpc"
    val baseUrl: String get() = url
}