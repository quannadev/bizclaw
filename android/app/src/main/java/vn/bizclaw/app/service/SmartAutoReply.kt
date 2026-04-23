package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.withContext
import vn.bizclaw.app.engine.ProviderChat
import vn.bizclaw.app.engine.ProviderManager

/**
 * Smart Auto-Reply Engine - AI-powered automatic responses for chat platforms.
 * 
 * Features:
 * - Configurable auto-reply rules
 * - AI-generated responses using LLM
 * - Quick reply templates
 * - Smart routing to human agents
 * - Sentiment detection
 * - Response throttling
 */
class SmartAutoReply(private val context: Context) {

    companion object {
        private const val TAG = "SmartAutoReply"
    }

    data class AutoReplyConfig(
        val enabled: Boolean = true,
        val useAI: Boolean = true,
        val responseDelayMs: Long = 1000, // Delay before sending auto-reply
        val maxResponsesPerHour: Int = 20,
        val escalateKeywords: List<String> = listOf("agent", "nhân viên", "manager", "sếp", "refund", "khiếu nại"),
        val ignoreKeywords: List<String> = listOf("stop", "unsubscribe", "hủy", "stop"),
        val greetingTemplates: List<String> = listOf(
            "Xin chào! Cảm ơn bạn đã liên hệ. Mình đang online và sẽ hỗ trợ bạn ngay!",
            "Chào bạn! 👋 Mình là trợ lý AI của cửa hàng. Bạn cần giúp gì ạ?",
        ),
        val fallbackTemplates: List<String> = listOf(
            "Cảm ơn bạn! Tin nhắn của bạn đã được ghi nhận. Nhân viên sẽ phản hồi sớm nhất có thể.",
            "Mình đã nhận được tin nhắn của bạn rồi! Để tối đa 2h, nhân viên sẽ trả lời bạn nhé.",
        ),
    )

    data class ChatMessage(
        val id: String,
        val platform: ChatPlatform,
        val senderId: String,
        val senderName: String,
        val content: String,
        val timestamp: Long,
        val threadId: String? = null,
        val isGroup: Boolean = false,
    )

    enum class ChatPlatform {
        ZALO, FACEBOOK, MESSENGER, TELEGRAM, WHATSAPP
    }

    data class AutoReplyResult(
        val messageId: String,
        val response: String,
        val responseType: ResponseType,
        val sentAt: Long,
        val escalated: Boolean = false,
    )

    enum class ResponseType {
        AI_GENERATED, TEMPLATE, ESCALATE, IGNORED, RATE_LIMITED
    }

    data class SentimentResult(
        val sentiment: Sentiment,
        val confidence: Float,
        val keywords: List<String>,
    )

    enum class Sentiment {
        POSITIVE, NEUTRAL, NEGATIVE, URGENT
    }

    private val _config = MutableStateFlow(AutoReplyConfig())
    val config: StateFlow<AutoReplyConfig> = _config

    private val responseHistory = mutableMapOf<String, MutableList<Long>>() // senderId -> timestamps
    private val escalationQueue = mutableListOf<ChatMessage>()

    private val sentimentKeywords = mapOf(
        Sentiment.POSITIVE to listOf("cảm ơn", "tuyệt vời", "good", "excellent", "awesome", "hài lòng", "satisfy", "love", "yêu", "tốt"),
        Sentiment.NEGATIVE to listOf("tệ", "bad", "awful", "hậu", "không hài lòng", "disappointed", "frustrated", "angry", "giận", "worse"),
        Sentiment.URGENT to listOf("gấp", "urgent", "immediately", "ngay", "ngay lập tức", "chờ", "waiting", "can't wait", "help", "giúp", "problem", "vấn đề"),
    )

    /**
     * Update configuration.
     */
    fun updateConfig(config: AutoReplyConfig) {
        _config.value = config
    }

    /**
     * Check if should auto-reply.
     */
    fun shouldAutoReply(message: ChatMessage): Boolean {
        val cfg = _config.value
        if (!cfg.enabled) return false

        // Check ignore keywords
        val lowerContent = message.content.lowercase()
        if (cfg.ignoreKeywords.any { lowerContent.contains(it) }) {
            Log.d(TAG, "Ignored: contains '${cfg.ignoreKeywords.find { lowerContent.contains(it) }}'")
            return false
        }

        // Check rate limit
        val now = System.currentTimeMillis()
        val hourAgo = now - 60 * 60 * 1000
        val recentResponses = responseHistory[message.senderId]?.count { it > hourAgo } ?: 0
        if (recentResponses >= cfg.maxResponsesPerHour) {
            Log.d(TAG, "Rate limited: ${message.senderId} has $recentResponses recent responses")
            return false
        }

        return true
    }

    /**
     * Generate auto-reply for a message.
     */
    suspend fun generateReply(message: ChatMessage): AutoReplyResult = withContext(Dispatchers.IO) {
        val cfg = _config.value
        val now = System.currentTimeMillis()

        // Detect sentiment
        val sentiment = detectSentiment(message.content)

        // Check escalation keywords
        if (shouldEscalate(message, sentiment)) {
            escalationQueue.add(message)
            return@withContext AutoReplyResult(
                messageId = message.id,
                response = " Tin nhắn của bạn đã được chuyển đến nhân viên. Vui lòng chờ trong giây lát!",
                responseType = ResponseType.ESCALATE,
                sentAt = now,
                escalated = true,
            )
        }

        // Generate response based on config
        val response = if (cfg.useAI) {
            generateAIResponse(message, sentiment)
        } else {
            selectTemplateResponse(message)
        }

        // Record response
        responseHistory.getOrPut(message.senderId) { mutableListOf() }.add(now)

        AutoReplyResult(
            messageId = message.id,
            response = response,
            responseType = if (cfg.useAI) ResponseType.AI_GENERATED else ResponseType.TEMPLATE,
            sentAt = now,
            escalated = false,
        )
    }

    /**
     * Generate AI-powered response.
     */
    private suspend fun generateAIResponse(message: ChatMessage, sentiment: SentimentResult): String {
        return try {
            val providerManager = ProviderManager(context)
            val providers = providerManager.loadProviders()
            val provider = providers.firstOrNull { it.enabled }
                ?: return selectFallbackResponse(sentiment)

            val systemPrompt = buildSystemPrompt()
            val sentimentPrefix = when (sentiment.sentiment) {
                Sentiment.URGENT -> "⚠️ Khách hàng có vẻ gấp. Hãy phản hồi nhanh và thể hiện sự quan tâm.\n"
                Sentiment.NEGATIVE -> "⚠️ Khách hàng có vẻ không hài lòng. Hãy hỏi han và giải quyết thỏa đáng.\n"
                else -> ""
            }

            val userPrompt = """
${sentimentPrefix}
Khách hàng: ${message.senderName}
Tin nhắn: ${message.content}

Hãy trả lời ngắn gọn, thân thiện bằng tiếng Việt.
Nếu câu hỏi cần thông tin cụ thể, hãy cho biết mình sẽ kiểm tra và phản hồi sau.
Keep response under 100 words.
            """.trimIndent()

            ProviderChat.appContext = context
            ProviderChat.chat(provider, "", userPrompt, systemPrompt)
        } catch (e: Exception) {
            Log.e(TAG, "AI response failed: ${e.message}")
            selectFallbackResponse(sentiment)
        }
    }

    /**
     * Build system prompt for AI.
     */
    private fun buildSystemPrompt(): String {
        return """
Bạn là trợ lý chat thông minh cho doanh nghiệp SME.
- Trả lời ngắn gọn, thân thiện, chuyên nghiệp
- Sử dụng emoji phù hợp
- Nếu không biết, hãy nói sẽ kiểm tra và phản hồi
- Không bịa thông tin sản phẩm/giá nếu không chắc chắn
- Hướng dẫn khách đến website hoặc hotline nếu cần
        """.trimIndent()
    }

    /**
     * Select template response based on message content.
     */
    private fun selectTemplateResponse(message: ChatMessage): String {
        val cfg = _config.value
        val lowerContent = message.content.lowercase()

        return when {
            // Greeting detection
            lowerContent.contains("chào") || lowerContent.contains("hello") || lowerContent.contains("hi") || lowerContent.contains("hey") ->
                cfg.greetingTemplates.random()

            // Question patterns
            lowerContent.contains("giá") || lowerContent.contains("price") || lowerContent.contains("bao nhiêu") ->
                "Cảm ơn bạn đã quan tâm! Giá sản phẩm sẽ tùy thuộc vào model bạn chọn. Bạn có thể inbox trực tiếp để được báo giá chi tiết nhé!"

            lowerContent.contains("ship") || lowerContent.contains("giao hàng") || lowerContent.contains("delivery") ->
                "Chào bạn! Hiện tại shop có hỗ trợ giao hàng toàn quốc. Thời gian giao thường 2-5 ngày tùy khu vực. Bạn ở đâu để mình kiểm tra phí ship nhé!"

            // Fallback
            else -> cfg.fallbackTemplates.random()
        }
    }

    /**
     * Select fallback response based on sentiment.
     */
    private fun selectFallbackResponse(sentiment: SentimentResult): String {
        return when (sentiment.sentiment) {
            Sentiment.URGENT -> "Xin lỗi vì sự bất tiện này! Mình đã ghi nhận và sẽ phản hồi bạn trong ít phút. Bạn có thể cho biết thêm chi tiết không ạ?"
            Sentiment.NEGATIVE -> "Mình rất tiếc nếu bạn chưa hài lòng. Làm ơn cho mình biết thêm chi tiết để mình có thể hỗ trợ tốt hơn nhé!"
            else -> "Cảm ơn bạn đã nhắn tin! Mình đang ghi nhận và sẽ phản hồi sớm nhất có thể."
        }
    }

    /**
     * Detect sentiment from message.
     */
    fun detectSentiment(content: String): SentimentResult {
        val lower = content.lowercase()
        val foundKeywords = mutableListOf<String>()

        var maxScore = 0
        var sentiment = Sentiment.NEUTRAL

        sentimentKeywords.forEach { (sent, keywords) ->
            val matches = keywords.count { lower.contains(it) }
            if (matches > 0) {
                foundKeywords.addAll(keywords.filter { lower.contains(it) })
                if (matches > maxScore) {
                    maxScore = matches
                    sentiment = sent
                }
            }
        }

        val confidence = (maxScore * 0.3f).coerceIn(0f, 1f)

        return SentimentResult(
            sentiment = sentiment,
            confidence = confidence,
            keywords = foundKeywords.distinct().take(5),
        )
    }

    /**
     * Check if should escalate to human agent.
     */
    private fun shouldEscalate(message: ChatMessage, sentiment: SentimentResult): Boolean {
        val cfg = _config.value
        val lowerContent = message.content.lowercase()

        // Always escalate negative sentiment
        if (sentiment.sentiment == Sentiment.URGENT) return true

        // Check escalation keywords
        if (cfg.escalateKeywords.any { lowerContent.contains(it) }) return true

        // Check escalation queue size
        if (escalationQueue.size > 10) return false

        return false
    }

    /**
     * Get pending escalations.
     */
    fun getPendingEscalations(): List<ChatMessage> {
        return escalationQueue.toList()
    }

    /**
     * Clear escalation queue.
     */
    fun clearEscalations() {
        escalationQueue.clear()
    }

    /**
     * Mark escalation as handled.
     */
    fun markEscalationHandled(messageId: String) {
        escalationQueue.removeAll { it.id == messageId }
    }
}
