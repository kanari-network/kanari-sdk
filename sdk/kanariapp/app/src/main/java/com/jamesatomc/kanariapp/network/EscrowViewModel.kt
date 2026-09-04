package com.jamesatomc.kanariapp.network

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.jamesatomc.kanariapp.network.models.EscrowDeal
import com.jamesatomc.kanariapp.network.models.KanariEnvironment
import com.jamesatomc.kanariapp.network.models.RpcRequest
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.serialization.json.*

class EscrowViewModel : ViewModel() {
    private val _isLoading = MutableStateFlow(false)
    val isLoading = _isLoading.asStateFlow()

    private val _deals = MutableStateFlow<List<EscrowDeal>>(emptyList())
    val deals = _deals.asStateFlow()
    
    private val _error = MutableStateFlow<String?>(null)
    val error = _error.asStateFlow()

    private val client = KanariClient(KanariEnvironment.dev)

    fun loadDeals(address: String) {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            try {
                val request = RpcRequest(
                    method = "kanari_getDeals",
                    params = buildJsonArray { add(JsonPrimitive(address)) }
                )
                val response = client.escrowService.getDeals(request)
                // In a real implementation, we would parse response.result
                // For now, we simulate an empty list or mock data
                _deals.value = emptyList()
            } catch (e: Exception) {
                _error.value = e.message ?: "Failed to load deals"
            }
            _isLoading.value = false
        }
    }

    fun createDeal(
        walletAddress: String,
        sellerAddress: String,
        amount: Long,
        tokenType: String,
        description: String
    ) {
        viewModelScope.launch {
            _isLoading.value = true
            try {
                val result = client.createEscrowDeal(
                    walletAddress, 
                    "deal_${System.currentTimeMillis()}", 
                    sellerAddress, 
                    amount, 
                    tokenType, 
                    description
                )
                if (result?.status == "success") {
                    loadDeals(walletAddress)
                } else {
                    _error.value = result?.errorMessage ?: "Failed to create deal"
                }
            } catch (e: Exception) {
                _error.value = e.message
            }
            _isLoading.value = false
        }
    }

    fun confirmDelivery(walletAddress: String, deal: EscrowDeal) {
        viewModelScope.launch {
            _isLoading.value = true
            try {
                val result = client.confirmEscrowDelivery(
                    walletAddress,
                    deal.objectId,
                    deal.coinType,
                    deal.proofId ?: ""
                )
                if (result?.status == "success") {
                    loadDeals(walletAddress)
                }
            } catch (e: Exception) {
                _error.value = e.message
            }
            _isLoading.value = false
        }
    }
}