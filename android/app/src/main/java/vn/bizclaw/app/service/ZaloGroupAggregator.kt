package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.text.SimpleDateFormat
import java.util.*

/**
 * Zalo Group Aggregator - Monitor multiple Zalo groups and generate reports.
 * 
 * Features:
 * - Fetch messages from Zalo groups (via notification listener)
 * - Analyze hot topics and trends
 * - Detect unanswered questions
 * - Generate periodic reports
 * - Alert admins for urgent issues
 */
class ZaloGroupAggregator(private val context: Context) {

    companion object {
        private const val TAG = "ZaloGroupAggregator"
    }

    data class GroupMessage(
        val groupId: String,
        val groupName: String,
        val senderId: String,
        val senderName: String,
        val content: String,
        val timestamp: Long,
        val isMention: Boolean = false,
        val hasMedia: Boolean = false,
        val reactions: Int = 0,
    )

    data class GroupReport(
        val groupId: String,
        val groupName: String,
        val period: ReportPeriod,
        val startTime: Long,
        val endTime: Long,
        val totalMessages: Int,
        val activeMembers: Int,
        val peakHour: Int,
        val hotTopics: List<TopicMention>,
        val unansweredQuestions: List<UnansweredQuestion>,
        val topMembers: List<MemberActivity>,
        val mediaCount: Int,
        val mentionsCount: Int,
        val generatedAt: Long = System.currentTimeMillis(),
    )

    data class TopicMention(
        val topic: String,
        val count: Int,
        val examples: List<String>,
    )

    data class UnansweredQuestion(
        val content: String,
        val senderName: String,
        val timestamp: Long,
        val hoursAgo: Double,
        val isMention: Boolean,
    )

    data class MemberActivity(
        val memberName: String,
        val messageCount: Int,
        val percentage: Float,
    )

    enum class ReportPeriod {
        HOURLY, DAILY, WEEKLY
    }

    private val messageBuffer = mutableMapOf<String, MutableList<GroupMessage>>()
    private val questionPatterns = listOf(
        Regex("(?:có|co|muốn|mong|mấy|giá|bao nhiêu|làm sao|nào|ở đâu|hỏi|ask)", RegexOption.IGNORE_CASE),
        Regex("\\?", RegexOption.IGNORE_CASE),
        Regex("(?:chưa|chua)\\s+(?:ai|ai đó|aide|nobody)\\s+(?:trả lời|traloi|reply)", RegexOption.IGNORE_CASE),
    )

    /**
     * Add a message to the buffer for analysis.
     */
    fun addMessage(message: GroupMessage) {
        val groupMessages = messageBuffer.getOrPut(message.groupId) { mutableListOf() }
        groupMessages.add(message)
        
        // Keep only last 1000 messages per group
        if (groupMessages.size > 1000) {
            groupMessages.removeAt(0)
        }
    }

    /**
     * Get messages from a specific group.
     */
    fun getGroupMessages(groupId: String, since: Long? = null): List<GroupMessage> {
        val messages = messageBuffer[groupId] ?: return emptyList()
        return if (since != null) {
            messages.filter { it.timestamp >= since }
        } else {
            messages
        }
    }

    /**
     * Get all messages across all groups.
     */
    fun getAllMessages(since: Long? = null): Map<String, List<GroupMessage>> {
        return messageBuffer.mapValues { (_, messages) ->
            if (since != null) {
                messages.filter { it.timestamp >= since }
            } else {
                messages
            }
        }
    }

    /**
     * Generate a report for a specific group.
     */
    fun generateGroupReport(
        groupId: String,
        period: ReportPeriod = ReportPeriod.DAILY,
    ): GroupReport? {
        val messages = messageBuffer[groupId] ?: return null
        if (messages.isEmpty()) return null

        val now = System.currentTimeMillis()
        val periodMs = when (period) {
            ReportPeriod.HOURLY -> 60 * 60 * 1000L
            ReportPeriod.DAILY -> 24 * 60 * 60 * 1000L
            ReportPeriod.WEEKLY -> 7 * 24 * 60 * 60 * 1000L
        }
        val startTime = now - periodMs

        val filteredMessages = messages.filter { it.timestamp >= startTime }
        if (filteredMessages.isEmpty()) return null

        val groupName = filteredMessages.first().groupName

        return GroupReport(
            groupId = groupId,
            groupName = groupName,
            period = period,
            startTime = startTime,
            endTime = now,
            totalMessages = filteredMessages.size,
            activeMembers = filteredMessages.map { it.senderId }.distinct().size,
            peakHour = calculatePeakHour(filteredMessages),
            hotTopics = extractHotTopics(filteredMessages),
            unansweredQuestions = extractUnansweredQuestions(filteredMessages, now),
            topMembers = extractTopMembers(filteredMessages),
            mediaCount = filteredMessages.count { it.hasMedia },
            mentionsCount = filteredMessages.count { it.isMention },
        )
    }

    /**
     * Generate report for all groups.
     */
    suspend fun generateAllGroupsReport(
        period: ReportPeriod = ReportPeriod.DAILY,
    ): List<GroupReport> = withContext(Dispatchers.Default) {
        messageBuffer.keys.mapNotNull { groupId ->
            generateGroupReport(groupId, period)
        }
    }

    /**
     * Calculate peak activity hour.
     */
    private fun calculatePeakHour(messages: List<GroupMessage>): Int {
        val hourCounts = messages.groupBy { msg ->
            val calendar = Calendar.getInstance()
            calendar.timeInMillis = msg.timestamp
            calendar.get(Calendar.HOUR_OF_DAY)
        }
        return hourCounts.maxByOrNull { it.value.size }?.key ?: 0
    }

    /**
     * Extract hot topics from messages.
     */
    private fun extractHotTopics(messages: List<GroupMessage>): List<TopicMention> {
        val wordCounts = mutableMapOf<String, Int>()
        val wordExamples = mutableMapOf<String, MutableList<String>>()

        // Common stop words to filter out
        val stopWords = setOf(
            "và", "của", "là", "có", "được", "trong", "cho", "với", "không", "theo",
            "to", "and", "the", "is", "a", "of", "in", "for", "with", "on",
            "nha", "nhé", "ạ", "ơi", "vậy", "mà", "rằng", "nếu",
        )

        messages.forEach { msg ->
            val words = msg.content
                .lowercase()
                .replace(Regex("[^a-zA-Zà-ỹ\\s]"), "")
                .split("\\s+".toRegex())
                .filter { it.length >= 3 && it !in stopWords }

            words.forEach { word ->
                wordCounts[word] = wordCounts.getOrDefault(word, 0) + 1
                wordExamples.getOrPut(word) { mutableListOf() }.apply {
                    if (size < 3) add(msg.content.take(100))
                }
            }
        }

        return wordCounts
            .filter { it.value >= 3 }
            .entries
            .sortedByDescending { it.value }
            .take(10)
            .map { (topic, count) ->
                TopicMention(
                    topic = topic,
                    count = count,
                    examples = wordExamples[topic] ?: emptyList(),
                )
            }
    }

    /**
     * Extract unanswered questions from messages.
     */
    private fun extractUnansweredQuestions(
        messages: List<GroupMessage>,
        now: Long,
    ): List<UnansweredQuestion> {
        val questions = messages.filter { msg ->
            questionPatterns.any { it.containsMatchIn(msg.content) }
        }

        // Group consecutive messages from different people
        val answeredQuestions = mutableSetOf<String>()
        val processed = mutableListOf<GroupMessage>()

        messages.sortedBy { it.timestamp }.forEach { msg ->
            val isReply = messages.any { other ->
                other.senderId != msg.senderId &&
                other.timestamp > msg.timestamp &&
                other.timestamp < msg.timestamp + 30 * 60 * 1000L // 30 min window
            }
            if (isReply) {
                answeredQuestions.add(msg.content.take(50))
            } else {
                processed.add(msg)
            }
        }

        val hourMs = 60 * 60 * 1000L
        return processed
            .filter { it.content.take(50) !in answeredQuestions }
            .filter { now - it.timestamp > 30 * 60 * 1000L } // At least 30 min old
            .sortedByDescending { it.timestamp }
            .take(10)
            .map { msg ->
                UnansweredQuestion(
                    content = msg.content,
                    senderName = msg.senderName,
                    timestamp = msg.timestamp,
                    hoursAgo = (now - msg.timestamp).toFloat() / hourMs,
                    isMention = msg.isMention,
                )
            }
    }

    /**
     * Extract top active members.
     */
    private fun extractTopMembers(messages: List<GroupMessage>): List<MemberActivity> {
        val memberCounts = messages.groupBy { it.senderId }
        val totalMessages = messages.size

        return memberCounts
            .map { (_, msgs) ->
                val firstMsg = msgs.first()
                MemberActivity(
                    memberName = firstMsg.senderName,
                    messageCount = msgs.size,
                    percentage = if (totalMessages > 0) {
                        (msgs.size.toFloat() / totalMessages) * 100
                    } else 0f,
                )
            }
            .sortedByDescending { it.messageCount }
            .take(10)
    }

    /**
     * Format report as text for display.
     */
    fun formatReportAsText(report: GroupReport): String {
        val dateFormat = SimpleDateFormat("HH:mm dd/MM", Locale.getDefault())

        return buildString {
            appendLine("📊 BÁO CÁO NHÓM: ${report.groupName}")
            appendLine("📅 ${report.period.name} (${dateFormat.format(Date(report.startTime))} - ${dateFormat.format(Date(report.endTime))})")
            appendLine()
            appendLine("═══ TỔNG QUAN ═══")
            appendLine("💬 Tin nhắn: ${report.totalMessages}")
            appendLine("👥 Thành viên active: ${report.activeMembers}")
            appendLine("⏰ Giờ peak: ${report.peakHour}:00")
            appendLine("📎 Media: ${report.mediaCount}")
            appendLine("📢 Mentions: ${report.mentionsCount}")
            appendLine()

            if (report.hotTopics.isNotEmpty()) {
                appendLine("═══ 🔥 CHỦ ĐỀ HOT ═══")
                report.hotTopics.take(5).forEachIndexed { idx, topic ->
                    appendLine("${idx + 1}. \"${topic.topic}\" - ${topic.count} lần")
                }
                appendLine()
            }

            if (report.unansweredQuestions.isNotEmpty()) {
                appendLine("═══ ❓ CÂU HỎI CHƯA TRẢ LỜI ═══")
                report.unansweredQuestions.take(5).forEach { q ->
                    val timeStr = if (q.hoursAgo < 1) {
                        "${(q.hoursAgo * 60).toInt()} phút trước"
                    } else {
                        "${q.hoursAgo.toInt()} giờ trước"
                    }
                    val prefix = if (q.isMention) "@admin" else ""
                    appendLine("• $prefix${q.content.take(60)}${if (q.content.length > 60) "..." else ""}")
                    appendLine("  ${q.senderName} • $timeStr")
                }
                appendLine()
            }

            if (report.topMembers.isNotEmpty()) {
                appendLine("═══ 🏆 TOP THÀNH VIÊN ═══")
                report.topMembers.take(5).forEachIndexed { idx, member ->
                    appendLine("${idx + 1}. ${member.memberName} - ${member.messageCount} tin (%.1f%%)".format(member.percentage))
                }
            }
        }
    }

    /**
     * Clear buffer for a specific group.
     */
    fun clearGroupBuffer(groupId: String) {
        messageBuffer[groupId]?.clear()
    }

    /**
     * Clear all buffers.
     */
    fun clearAll() {
        messageBuffer.values.forEach { it.clear() }
    }
}
