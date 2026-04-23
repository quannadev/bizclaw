package vn.bizclaw.app.service

import android.content.Context
import android.media.MediaRecorder
import android.os.Build
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.MultipartBody
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.asRequestBody
import okio.buffer
import okio.sink
import vn.bizclaw.app.engine.ProviderManager
import vn.bizclaw.app.engine.ProviderType
import java.io.File
import java.io.InputStream
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Speech-to-Text Service for meeting transcription.
 * 
 * Supports multiple providers:
 * 1. Google Speech-to-Text API (cloud)
 * 2. OpenAI Whisper API (cloud)
 * 3. Local Whisper via Ollama (on-device)
 * 
 * Usage:
 * ```
 * val stt = SpeechToText(context)
 * val transcript = stt.transcribe(audioFile)
 * ```
 */
class SpeechToText(private val context: Context) {
    
    companion object {
        private const val TAG = "SpeechToText"
    }

    private val client = OkHttpClient.Builder()
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(120, TimeUnit.SECONDS)
        .writeTimeout(60, TimeUnit.SECONDS)
        .build()

    /**
     * Transcription result with metadata.
     */
    data class TranscriptionResult(
        val text: String,
        val language: String = "vi",
        val duration: Float = 0f,
        val segments: List<Segment> = emptyList(),
        val provider: String,
        val success: Boolean = true,
        val error: String? = null,
    )

    data class Segment(
        val text: String,
        val startMs: Long,
        val endMs: Long,
        val speaker: String? = null,
    )

    /**
     * Transcribe audio file to text.
     * Automatically selects best available provider.
     */
    suspend fun transcribe(
        audioFile: File,
        language: String = "vi",
    ): TranscriptionResult = withContext(Dispatchers.IO) {
        Log.i(TAG, "Transcribing: ${audioFile.name} (${audioFile.length()} bytes)")

        // Try providers in order of preference
        val providerManager = ProviderManager(context)
        val providers = providerManager.loadProviders()

        // 1. Try Whisper API (OpenAI)
        val openAiProvider = providers.firstOrNull { 
            it.enabled && it.type == ProviderType.OPENAI && it.apiKey.isNotBlank() 
        }
        if (openAiProvider != null) {
            try {
                Log.d(TAG, "Trying OpenAI Whisper API...")
                val result = transcribeWithWhisper(audioFile, openAiProvider.apiKey)
                if (result.success) return@withContext result
            } catch (e: Exception) {
                Log.w(TAG, "Whisper API failed: ${e.message}")
            }
        }

        // 2. Try Google Speech-to-Text
        val googleProvider = providers.firstOrNull {
            it.enabled && it.type == ProviderType.GOOGLE && it.apiKey.isNotBlank()
        }
        if (googleProvider != null) {
            try {
                Log.d(TAG, "Trying Google Speech-to-Text...")
                val result = transcribeWithGoogle(audioFile, googleProvider.apiKey, language)
                if (result.success) return@withContext result
            } catch (e: Exception) {
                Log.w(TAG, "Google STT failed: ${e.message}")
            }
        }

        // 3. Try local Ollama with Whisper
        val ollamaProvider = providers.firstOrNull {
            it.enabled && it.type == ProviderType.OLLAMA && it.apiKey.isBlank()
        }
        if (ollamaProvider != null) {
            try {
                Log.d(TAG, "Trying local Ollama Whisper...")
                val result = transcribeWithOllama(audioFile, ollamaProvider.baseUrl)
                if (result.success) return@withContext result
            } catch (e: Exception) {
                Log.w(TAG, "Ollama Whisper failed: ${e.message}")
            }
        }

        // 4. Fallback: Mock transcription with LLM
        Log.w(TAG, "No STT provider available, using LLM fallback...")
        transcribeWithLLMFallback(audioFile, providers)
    }

    /**
     * Transcribe with OpenAI Whisper API.
     */
    private suspend fun transcribeWithWhisper(
        audioFile: File,
        apiKey: String,
    ): TranscriptionResult = withContext(Dispatchers.IO) {
        val mediaType = "audio/m4a".toMediaType()
        
        val requestBody = MultipartBody.Builder()
            .setType(MultipartBody.FORM)
            .addFormDataPart(
                "file",
                audioFile.name,
                audioFile.asRequestBody(mediaType)
            )
            .addFormDataPart("model", "whisper-1")
            .addFormDataPart("language", "vi")
            .addFormDataPart("response_format", "verbose_json")
            .addFormDataPart("timestamp_granularities[]", "segment")
            .build()

        val request = Request.Builder()
            .url("https://api.openai.com/v1/audio/transcriptions")
            .post(requestBody)
            .addHeader("Authorization", "Bearer $apiKey")
            .build()

        val response = client.newCall(request).execute()
        val body = response.body?.string() ?: throw Exception("Empty response")

        if (!response.isSuccessful) {
            throw Exception("Whisper API error ${response.code}: $body")
        }

        // Parse JSON response
        val json = kotlinx.serialization.json.Json { ignoreUnknownKeys = true }
        val whisperResponse = json.decodeFromString<WhisperResponse>(body)

        TranscriptionResult(
            text = whisperResponse.text ?: "",
            language = whisperResponse.language ?: "vi",
            duration = whisperResponse.duration ?: 0f,
            segments = whisperResponse.segments?.map { seg ->
                Segment(
                    text = seg.text ?: "",
                    startMs = (seg.start ?: 0.0).toLong() * 1000,
                    endMs = (seg.end ?: 0.0).toLong() * 1000,
                )
            } ?: emptyList(),
            provider = "openai_whisper",
        )
    }

    /**
     * Transcribe with Google Speech-to-Text API.
     */
    private suspend fun transcribeWithGoogle(
        audioFile: File,
        apiKey: String,
        language: String,
    ): TranscriptionResult = withContext(Dispatchers.IO) {
        val audioContent = android.util.Base64.encodeToString(
            audioFile.readBytes(),
            android.util.Base64.NO_WRAP
        )

        val jsonBody = """
        {
            "config": {
                "encoding": "MP3",
                "sampleRateHertz": 16000,
                "languageCode": "${language}-VN",
                "enableWordTimeOffsets": true,
                "enableAutomaticPunctuation": true,
                "model": "latest_long"
            },
            "audio": {
                "content": "$audioContent"
            }
        }
        """.trimIndent()

        val requestBody = jsonBody.toRequestBody("application/json".toMediaType())

        val request = Request.Builder()
            .url("https://speech.googleapis.com/v1/speech:recognize?key=$apiKey")
            .post(requestBody)
            .build()

        val response = client.newCall(request).execute()
        val body = response.body?.string() ?: throw Exception("Empty response")

        if (!response.isSuccessful) {
            throw Exception("Google STT error ${response.code}: $body")
        }

        val json = kotlinx.serialization.json.Json { ignoreUnknownKeys = true }
        val googleResponse = json.decodeFromString<GoogleSTTResponse>(body)

        val results = googleResponse.results ?: emptyList()
        val transcription = results.mapNotNull { it.alternatives?.firstOrNull()?.transcript }.joinToString(" ")
        
        val segments = results.flatMap { result ->
            result.alternatives?.firstOrNull()?.words?.map { word ->
                Segment(
                    text = word.word ?: "",
                    startMs = parseGoogleTime(word.startTime),
                    endMs = parseGoogleTime(word.endTime),
                )
            } ?: emptyList()
        }

        TranscriptionResult(
            text = transcription,
            language = language,
            segments = segments,
            provider = "google_stt",
        )
    }

    /**
     * Transcribe with local Ollama Whisper.
     */
    private suspend fun transcribeWithOllama(
        audioFile: File,
        baseUrl: String,
    ): TranscriptionResult = withContext(Dispatchers.IO) {
        // Convert to WAV if needed (Ollama prefers WAV)
        val wavFile = convertToWav(audioFile)
        
        try {
            val audioBase64 = android.util.Base64.encodeToString(
                wavFile.readBytes(),
                android.util.Base64.NO_WRAP
            )

            val jsonBody = """
            {
                "model": "whisper",
                "input": "$audioBase64"
            }
            """.trimIndent()

            val requestBody = jsonBody.toRequestBody("application/json".toMediaType())

            val request = Request.Builder()
                .url("${baseUrl.trimEnd('/')}/api/generate")
                .post(requestBody)
                .build()

            val response = client.newCall(request).execute()
            val body = response.body?.string() ?: throw Exception("Empty response")

            if (!response.isSuccessful) {
                throw Exception("Ollama error ${response.code}: $body")
            }

            val json = kotlinx.serialization.json.Json { ignoreUnknownKeys = true }
            val ollamaResponse = json.decodeFromString<OllamaSTTResponse>(body)

            TranscriptionResult(
                text = ollamaResponse.response ?: "",
                provider = "ollama_whisper",
            )
        } finally {
            // Clean up temp file if different from original
            if (wavFile != audioFile) {
                wavFile.delete()
            }
        }
    }

    /**
     * Fallback: Use LLM to generate summary when no STT available.
     * This is NOT real transcription - just a placeholder.
     */
    private suspend fun transcribeWithLLMFallback(
        audioFile: File,
        providers: List<vn.bizclaw.app.engine.Provider>,
    ): TranscriptionResult = withContext(Dispatchers.IO) {
        Log.w(TAG, "⚠️ Using LLM fallback - this is NOT real transcription!")
        
        TranscriptionResult(
            text = "[LƯU Ý: Chưa có provider STT. Để transcription hoạt động, hãy thêm OpenAI API Key trong Settings.]",
            provider = "llm_fallback",
            success = false,
            error = "No STT provider configured",
        )
    }

    /**
     * Convert audio file to WAV format for Ollama.
     */
    private fun convertToWav(input: File): File {
        // For simplicity, we'll assume input is already WAV or m4a
        // In production, use FFmpeg or Android's MediaCodec for conversion
        return input
    }

    private fun parseGoogleTime(time: GoogleSTTResponse.TimeInfo?): Long {
        if (time == null) return 0L
        val seconds = time.seconds?.toLongOrNull() ?: 0L
        val nanos = time.nanos?.toLongOrNull() ?: 0L
        return seconds * 1000 + nanos / 1_000_000
    }
}

// ─── Response Models ─────────────────────────────────────────────────────────

@kotlinx.serialization.Serializable
data class WhisperResponse(
    val text: String? = null,
    val language: String? = null,
    val duration: Float? = null,
    val segments: List<WhisperSegment>? = null,
)

@kotlinx.serialization.Serializable
data class WhisperSegment(
    val id: Int? = null,
    val text: String? = null,
    val start: Double? = null,
    val end: Double? = null,
)

@kotlinx.serialization.Serializable
data class GoogleSTTResponse(
    val results: List<GoogleResult>? = null,
)

@kotlinx.serialization.Serializable
data class GoogleResult(
    val alternatives: List<GoogleAlternative>? = null,
)

@kotlinx.serialization.Serializable
data class GoogleAlternative(
    val transcript: String? = null,
    val words: List<GoogleWord>? = null,
)

@kotlinx.serialization.Serializable
data class GoogleWord(
    val word: String? = null,
    val startTime: TimeInfo? = null,
    val endTime: TimeInfo? = null,
)

@kotlinx.serialization.Serializable
data class TimeInfo(
    val seconds: String? = null,
    val nanos: String? = null,
)

@kotlinx.serialization.Serializable
data class OllamaSTTResponse(
    val response: String? = null,
    val done: Boolean = true,
)
