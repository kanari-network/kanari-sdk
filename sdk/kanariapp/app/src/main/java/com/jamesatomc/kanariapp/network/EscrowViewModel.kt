package com.jamesatomc.kanariapp.network

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.jamesatomc.kanariapp.network.models.KanariEnvironment
import com.jamesatomc.kanariapp.network.models.RpcRequest
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonArray

class EscrowViewModel : ViewModel() {
    private val _isLoading = MutableStateFlow(false)
    val isLoading = _isLoading.asStateFlow()

    private val _deals = MutableStateFlow<List<String>>(emptyList())
    val deals = _deals.asStateFlow()

    private val client = KanariClient(KanariEnvironment.TESTNET)

    fun loadDeals(address: String) {
        viewModelScope.launch {
            _isLoading.value = true
            try {
                val request = RpcRequest(
                    method = "kanari_getDeals",
                    params = buildJsonArray { add(JsonPrimitive(address)) }.toList()
                )
                val response = client.escrowService.getDeals(request)
                // Map JsonElement to local Deal model if needed
                _deals.value = emptyList() // response.result?.map { ... } ?: emptyList()
            } catch (e: Exception) {
                // Error handling
            }
            _isLoading.value = false
        }
    }
}
