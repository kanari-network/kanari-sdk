package io.kanari.wallet.utils

import java.io.File
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString

object ConfigManager {
    /**
     * Get Kari configuration directory
     */
    fun getKariDir(): File {
        val homeDir = System.getProperty("user.home")
        return File(homeDir, ".kari").also { 
            if (!it.exists()) it.mkdirs() 
        }
    }
    
    /**
     * Load configuration from file
     */
    fun loadConfig(): MutableMap<String, String>? {
        val configFile = File(getKariDir(), "config.json")
        if (!configFile.exists()) {
            return mutableMapOf()
        }
        
        return try {
            val configJson = configFile.readText()
            val configObject = Json.decodeFromString<Map<String, String>>(configJson)
            configObject.toMutableMap()
        } catch (e: Exception) {
            mutableMapOf()
        }
    }
    
    /**
     * Save configuration to file
     */
    fun saveConfig(config: Map<String, String>): Boolean {
        val configFile = File(getKariDir(), "config.json")
        return try {
            val configJson = Json.encodeToString(config)
            configFile.writeText(configJson)
            true
        } catch (e: Exception) {
            false
        }
    }
}
