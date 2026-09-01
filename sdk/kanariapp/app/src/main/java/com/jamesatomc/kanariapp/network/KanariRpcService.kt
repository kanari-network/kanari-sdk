package com.jamesatomc.kanariapp.network

import com.jamesatomc.kanariapp.network.models.*
import kotlinx.serialization.json.JsonElement
import retrofit2.http.Body
import retrofit2.http.POST

interface KanariRpcService {
    @POST("rpc")
    suspend fun call(@Body request: RpcRequest): RpcResponse<JsonElement>

    // Specific helpers for common types
    @POST("rpc")
    suspend fun getAccount(@Body request: RpcRequest): RpcResponse<AccountInfo>

    @POST("rpc")
    suspend fun getObjects(@Body request: RpcRequest): RpcResponse<List<ObjectInfo>>

    @POST("rpc")
    suspend fun getTransaction(@Body request: RpcRequest): RpcResponse<TransactionDetails>

    @POST("rpc")
    suspend fun executeTransaction(@Body request: RpcRequest): RpcResponse<TransactionResult>

    @POST("rpc")
    suspend fun getAllBalances(@Body request: RpcRequest): RpcResponse<List<TokenBalance>>
}
