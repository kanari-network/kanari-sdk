package com.jamesatomc.kanariapp.network

import com.jamesatomc.kanariapp.network.models.RpcRequest
import com.jamesatomc.kanariapp.network.models.RpcResponse
import kotlinx.serialization.json.JsonElement
import retrofit2.http.Body
import retrofit2.http.POST

interface EscrowService {
    @POST("rpc")
    suspend fun getDeals(@Body request: RpcRequest): RpcResponse<List<JsonElement>>
}
