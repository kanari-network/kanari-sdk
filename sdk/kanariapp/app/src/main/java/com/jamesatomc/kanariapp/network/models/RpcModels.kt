package com.jamesatomc.kanariapp.network.models

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonElement

@Serializable
data class RpcResponse<T>(
    val jsonrpc: String = "2.0",
    val result: T? = null,
    val error: RpcError? = null,
    val id: JsonElement? = null
)

@Serializable
data class RpcError(
    val code: Int,
    val message: String,
    val data: JsonElement? = null
)

@Serializable
data class RpcRequest(
    val method: String,
    val params: JsonElement, // Changed from List<JsonElement> to match dynamic usage in Flutter
    val jsonrpc: String = "2.0",
    val id: Long = System.currentTimeMillis()
)

@Serializable
data class OwnedObjectsResponse(
    val objects: List<ObjectInfo> = emptyList(),
    val summary: OwnerObjectSummary? = null
)

@Serializable
data class OwnerObjectSummary(
    val owner: String,
    val total_objects: Int,
    val object_changes_hint: String
)