package vn.bizclaw.app.security

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Box Security Module
 * 
 * Features:
 * - Biometric lock (fingerprint, face)
 * - Chat history encryption
 * - Hard offline mode
 * - Secure key storage
 */
class BoxSecurity(private val context: Context) {
    
    private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    
    private var biometricEnabled = false
    private var encryptionEnabled = true
    private var hardOfflineMode = false
    
    // Encryption key alias
    private val keyAlias = "box_security_key"
    
    /**
     * Check if biometric is available
     */
    fun isBiometricAvailable(): BiometricStatus {
        val biometricManager = BiometricManager.from(context)
        
        return when (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)) {
            BiometricManager.BIOMETRIC_SUCCESS -> BiometricStatus.AVAILABLE
            BiometricManager.BIOMETRIC_ERROR_NO_HARDWARE -> BiometricStatus.NO_HARDWARE
            BiometricManager.BIOMETRIC_ERROR_HW_UNAVAILABLE -> BiometricStatus.UNAVAILABLE
            BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED -> BiometricStatus.NOT_ENROLLED
            else -> BiometricStatus.UNAVAILABLE
        }
    }
    
    /**
     * Enable biometric lock
     */
    suspend fun enableBiometric(activity: FragmentActivity): Result<Unit> = withContext(Dispatchers.Main) {
        try {
            val status = isBiometricAvailable()
            if (status != BiometricStatus.AVAILABLE) {
                return@withContext Result.failure(Exception("Biometric not available: $status"))
            }
            
            biometricEnabled = true
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * Authenticate with biometric
     */
    suspend fun authenticate(
        activity: FragmentActivity,
        reason: String = "Unlock Box AI"
    ): Result<BiometricResult> = withContext(Dispatchers.Main) {
        try {
            val executor = ContextCompat.getMainExecutor(context)
            
            val callback = object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    super.onAuthenticationSucceeded(result)
                }
                
                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                    super.onAuthenticationError(errorCode, errString)
                }
                
                override fun onAuthenticationFailed() {
                    super.onAuthenticationFailed()
                }
            }
            
            val biometricPrompt = BiometricPrompt(activity, executor, callback)
            
            val promptInfo = BiometricPrompt.PromptInfo.Builder()
                .setTitle("Box AI Security")
                .setSubtitle(reason)
                .setNegativeButtonText("Cancel")
                .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
                .build()
            
            biometricPrompt.authenticate(promptInfo)
            Result.success(BiometricResult.Success)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * Encrypt data
     */
    suspend fun encrypt(data: ByteArray): Result<EncryptedData> = withContext(Dispatchers.IO) {
        try {
            val key = getOrCreateKey()
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.ENCRYPT_MODE, key)
            
            val encrypted = cipher.doFinal(data)
            val iv = cipher.iv
            
            Result.success(EncryptedData(encrypted, iv))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * Decrypt data
     */
    suspend fun decrypt(encrypted: EncryptedData): Result<ByteArray> = withContext(Dispatchers.IO) {
        try {
            val key = getOrCreateKey()
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            val spec = GCMParameterSpec(128, encrypted.iv)
            cipher.init(Cipher.DECRYPT_MODE, key, spec)
            
            val decrypted = cipher.doFinal(encrypted.data)
            Result.success(decrypted)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * Encrypt string
     */
    suspend fun encryptString(text: String): Result<String> {
        return encrypt(text.toByteArray()).map { encrypted ->
            android.util.Base64.encodeToString(encrypted.data, android.util.Base64.NO_WRAP) + ":" + 
            android.util.Base64.encodeToString(encrypted.iv, android.util.Base64.NO_WRAP)
        }
    }
    
    /**
     * Decrypt string
     */
    suspend fun decryptString(encrypted: String): Result<String> {
        return try {
            val parts = encrypted.split(":")
            if (parts.size != 2) {
                return Result.failure(Exception("Invalid encrypted format"))
            }
            
            val data = android.util.Base64.decode(parts[0], android.util.Base64.NO_WRAP)
            val iv = android.util.Base64.decode(parts[1], android.util.Base64.NO_WRAP)
            
            decrypt(EncryptedData(data, iv)).map { String(it) }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    /**
     * Enable hard offline mode
     */
    fun enableHardOfflineMode(enable: Boolean) {
        hardOfflineMode = enable
    }
    
    /**
     * Check if hard offline mode is enabled
     */
    fun isHardOfflineMode() = hardOfflineMode
    
    /**
     * Get or create encryption key
     */
    private fun getOrCreateKey(): SecretKey {
        val existingKey = keyStore.getEntry(keyAlias, null) as? KeyStore.SecretKeyEntry
        if (existingKey != null) {
            return existingKey.secretKey
        }
        
        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            "AndroidKeyStore"
        )
        
        val keySpec = KeyGenParameterSpec.Builder(
            keyAlias,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setUserAuthenticationRequired(false)
            .build()
        
        keyGenerator.init(keySpec)
        return keyGenerator.generateKey()
    }
    
    /**
     * Encrypt chat history
     */
    suspend fun encryptChatHistory(messages: List<ChatMessage>): Result<String> {
        val json = Gson().toJson(messages)
        encryptString(json)
    }
    
    /**
     * Decrypt chat history
     */
    suspend fun decryptChatHistory(encrypted: String): Result<List<ChatMessage>> {
        decryptString(encrypted).mapCatching { json ->
            Gson().fromJson(json, Array<ChatMessage>::class.java).toList()
        }
    }
}

/**
 * Biometric status
 */
enum class BiometricStatus {
    AVAILABLE,
    NO_HARDWARE,
    UNAVAILABLE,
    NOT_ENROLLED
}

/**
 * Biometric result
 */
sealed class BiometricResult
object BiometricResult {
    object Success : BiometricResult()
    data class Error(val code: Int, val message: String) : BiometricResult()
}

/**
 * Encrypted data container
 */
data class EncryptedData(
    val data: ByteArray,
    val iv: ByteArray
)

/**
 * Chat message for encryption
 */
data class ChatMessage(
    val id: String,
    val role: String,
    val content: String,
    val timestamp: Long
)

/**
 * Simple JSON serializer
 */
class Gson {
    fun toJson(obj: Any): String {
        return when (obj) {
            is List<*> -> "[${obj.joinToString(",") { toJson(it as Any) }]"
            is ChatMessage -> """{"id":"${obj.id}","role":"${obj.role}","content":"${obj.content.escape()}","timestamp":${obj.timestamp}}"""
            else -> obj.toString()
        }
    }
    
    fun <T> fromJson(json: String, clazz: Class<T>): T {
        return when (clazz) {
            ChatMessage::class.java -> {
                val parts = json.trim('{', '}').split(",")
                val map = parts.associate { part ->
                    val kv = part.split(":")
                    kv[0].trim('"') to kv.getOrNull(1)?.trim('"') ?: ""
                }
                ChatMessage(
                    id = map["id"] ?: "",
                    role = map["role"] ?: "",
                    content = map["content"] ?: "",
                    timestamp = (map["timestamp"] ?: "0").toLongOrNull() ?: 0
                )
            }
            else -> throw NotImplementedError()
        } as T
    }
    
    private fun String.escape() = this
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
}
