package vn.bizclaw.app.service

import android.content.Context
import android.util.Log
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import java.text.SimpleDateFormat
import java.util.*

/**
 * Unified Dashboard - Combined metrics from all BizClaw services.
 * 
 * Aggregates:
 * - Zalo chats & group messages
 * - Email summaries
 * - Social media posts
 * - Pending action items
 * - System alerts
 */
class UnifiedDashboard(private val context: Context) {

    companion object {
        private const val TAG = "UnifiedDashboard"
    }

    data class DashboardData(
        val timestamp: Long = System.currentTimeMillis(),
        val zaloMetrics: ZaloMetrics? = null,
        val emailMetrics: EmailMetrics? = null,
        val socialMetrics: SocialMetrics? = null,
        val pendingActions: List<ActionItem> = emptyList(),
        val alerts: List<Alert> = emptyList(),
        val summary: DashboardSummary? = null,
    )

    data class ZaloMetrics(
        val totalChats: Int = 0,
        val unreadChats: Int = 0,
        val autoRepliesSent: Int = 0,
        val escalatedToAgent: Int = 0,
        val avgResponseTime: Long = 0,
        val csatScore: Float = 0f, // Customer Satisfaction 1-5
        val newToday: Int = 0,
        val pendingReplies: Int = 0,
    )

    data class EmailMetrics(
        val totalEmails: Int = 0,
        val unreadEmails: Int = 0,
        val urgentEmails: Int = 0,
        val sentToday: Int = 0,
        val avgResponseTime: Long = 0,
        val topSender: String? = null,
        val pendingFollowUp: Int = 0,
    )

    data class SocialMetrics(
        val totalPosts: Int = 0,
        val scheduledPosts: Int = 0,
        val postsToday: Int = 0,
        val totalReach: Int = 0,
        val totalEngagement: Int = 0,
        val platformBreakdown: Map<String, PlatformStats> = emptyMap(),
    )

    data class PlatformStats(
        val posts: Int = 0,
        val reach: Int = 0,
        val engagement: Int = 0,
    )

    data class ActionItem(
        val id: String,
        val title: String,
        val description: String,
        val source: ActionSource,
        val priority: Priority,
        val assignee: String? = null,
        val deadline: Long? = null,
        val createdAt: Long,
        val status: ActionStatus = ActionStatus.PENDING,
    )

    enum class ActionSource {
        ZALO, EMAIL, SOCIAL, MEETING, AGENT
    }

    enum class Priority {
        URGENT, HIGH, NORMAL, LOW
    }

    enum class ActionStatus {
        PENDING, IN_PROGRESS, COMPLETED, CANCELLED
    }

    data class Alert(
        val id: String,
        val type: AlertType,
        val title: String,
        val message: String,
        val severity: AlertSeverity,
        val timestamp: Long,
        val actionRequired: String? = null,
        val isRead: Boolean = false,
    )

    enum class AlertType {
        NEW_MESSAGE, EMAIL_URGENT, SOCIAL_MENTION, ESCALATION, SYSTEM, DEADLINE
    }

    enum class AlertSeverity {
        CRITICAL, HIGH, MEDIUM, LOW
    }

    data class DashboardSummary(
        val totalUnread: Int = 0,
        val totalPending: Int = 0,
        val totalAlerts: Int = 0,
        val urgentCount: Int = 0,
        val healthScore: Float = 0f, // 0-100%
        val lastSync: Long = 0,
    )

    private val _dashboardData = MutableStateFlow(DashboardData())
    val dashboardData: StateFlow<DashboardData> = _dashboardData

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading

    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    // Services
    private var zaloAggregator: ZaloGroupAggregator? = null
    private var emailAggregator: EmailAggregator? = null
    private var postManager: UnifiedPostManager? = null
    private var smartAutoReply: SmartAutoReply? = null

    // Action items storage
    private val actionItems = mutableListOf<ActionItem>()

    // Alerts storage
    private val alerts = mutableListOf<Alert>()

    /**
     * Initialize dashboard with required services.
     */
    fun initialize(
        zaloAggregator: ZaloGroupAggregator? = null,
        emailAggregator: EmailAggregator? = null,
        postManager: UnifiedPostManager? = null,
        smartAutoReply: SmartAutoReply? = null,
    ) {
        this.zaloAggregator = zaloAggregator
        this.emailAggregator = emailAggregator
        this.postManager = postManager
        this.smartAutoReply = smartAutoReply
    }

    /**
     * Refresh all dashboard data.
     */
    suspend fun refresh() {
        if (_isLoading.value) return
        _isLoading.value = true

        try {
            val zaloMetrics = fetchZaloMetrics()
            val emailMetrics = fetchEmailMetrics()
            val socialMetrics = fetchSocialMetrics()

            // Check for new alerts
            checkForAlerts(zaloMetrics, emailMetrics)

            // Update dashboard
            _dashboardData.value = DashboardData(
                timestamp = System.currentTimeMillis(),
                zaloMetrics = zaloMetrics,
                emailMetrics = emailMetrics,
                socialMetrics = socialMetrics,
                pendingActions = actionItems.toList(),
                alerts = alerts.toList(),
                summary = generateSummary(zaloMetrics, emailMetrics, socialMetrics),
            )
        } catch (e: Exception) {
            Log.e(TAG, "Refresh failed: ${e.message}")
        } finally {
            _isLoading.value = false
        }
    }

    /**
     * Start periodic refresh.
     */
    fun startPeriodicRefresh(intervalMs: Long = 5 * 60 * 1000L) {
        scope.launch {
            while (isActive) {
                refresh()
                delay(intervalMs)
            }
        }
    }

    /**
     * Stop periodic refresh.
     */
    fun stopPeriodicRefresh() {
        scope.cancel()
    }

    private suspend fun fetchZaloMetrics(): ZaloMetrics = withContext(Dispatchers.IO) {
        // Fetch from SmartAutoReply
        val autoReply = smartAutoReply
        val escalations = autoReply?.getPendingEscalations()?.size ?: 0

        ZaloMetrics(
            totalChats = 0, // Would come from notification listener
            unreadChats = 0,
            autoRepliesSent = 0,
            escalatedToAgent = escalations,
            avgResponseTime = 1500, // Mock: 1.5s average
            csatScore = 4.5f,
            newToday = 0,
            pendingReplies = escalations,
        )
    }

    private suspend fun fetchEmailMetrics(): EmailMetrics = withContext(Dispatchers.IO) {
        val aggregator = emailAggregator ?: return@withContext EmailMetrics()

        try {
            val emails = aggregator.fetchAllEmails(maxResults = 100)
            val summary = aggregator.generateSummary(emails)

            EmailMetrics(
                totalEmails = summary.totalEmails,
                unreadEmails = summary.unreadEmails,
                urgentEmails = summary.urgentEmails,
                sentToday = 0,
                avgResponseTime = 0,
                topSender = summary.bySender.entries.firstOrNull()?.key,
                pendingFollowUp = summary.unreadEmails / 5, // Estimate
            )
        } catch (e: Exception) {
            Log.e(TAG, "Email fetch error: ${e.message}")
            EmailMetrics()
        }
    }

    private suspend fun fetchSocialMetrics(): SocialMetrics = withContext(Dispatchers.IO) {
        val manager = postManager ?: return@withContext SocialMetrics()

        val configs = manager.getPlatformConfigs()
        val scheduled = manager.getScheduledPosts()

        val platformBreakdown = configs.associate { config ->
            config.platform.name to PlatformStats(
                posts = 0,
                reach = 0,
                engagement = 0,
            )
        }

        SocialMetrics(
            totalPosts = 0,
            scheduledPosts = scheduled.size,
            postsToday = 0,
            totalReach = 0,
            totalEngagement = 0,
            platformBreakdown = platformBreakdown,
        )
    }

    private fun checkForAlerts(zalo: ZaloMetrics?, email: EmailMetrics?) {
        val now = System.currentTimeMillis()

        // Clear old alerts
        alerts.removeAll { now - it.timestamp > 24 * 60 * 60 * 1000L }

        // Add urgent email alert
        if ((email?.urgentEmails ?: 0) > 0) {
            val existing = alerts.find { 
                it.type == AlertType.EMAIL_URGENT && !it.isRead 
            }
            if (existing == null) {
                alerts.add(0, Alert(
                    id = "email_urgent_${now}",
                    type = AlertType.EMAIL_URGENT,
                    title = "📧 Email Urgent",
                    message = "Có ${email?.urgentEmails} email khẩn cần xử lý",
                    severity = AlertSeverity.HIGH,
                    timestamp = now,
                    actionRequired = "Xem email",
                ))
            }
        }

        // Add escalation alert
        if ((zalo?.pendingReplies ?: 0) > 0) {
            val existing = alerts.find { 
                it.type == AlertType.ESCALATION && !it.isRead 
            }
            if (existing == null) {
                alerts.add(0, Alert(
                    id = "escalation_${now}",
                    type = AlertType.ESCALATION,
                    title = "⚠️ Có khách hàng cần hỗ trợ",
                    message = "Có ${zalo?.pendingReplies} cuộc chat cần phản hồi",
                    severity = AlertSeverity.MEDIUM,
                    timestamp = now,
                    actionRequired = "Xem chat",
                ))
            }
        }
    }

    private fun generateSummary(
        zalo: ZaloMetrics?,
        email: EmailMetrics?,
        social: SocialMetrics?,
    ): DashboardSummary {
        val totalUnread = (zalo?.unreadChats ?: 0) + (email?.unreadEmails ?: 0)
        val totalPending = (zalo?.pendingReplies ?: 0) + (email?.pendingFollowUp ?: 0) + (social?.scheduledPosts ?: 0)
        val urgentCount = (zalo?.escalatedToAgent ?: 0) + (email?.urgentEmails ?: 0)

        val healthScore = calculateHealthScore(totalUnread, totalPending, urgentCount)

        return DashboardSummary(
            totalUnread = totalUnread,
            totalPending = totalPending,
            totalAlerts = alerts.count { !it.isRead },
            urgentCount = urgentCount,
            healthScore = healthScore,
            lastSync = System.currentTimeMillis(),
        )
    }

    private fun calculateHealthScore(unread: Int, pending: Int, urgent: Int): Float {
        var score = 100f
        
        // Deduct for unread
        score -= (unread * 0.5f).coerceAtMost(30f)
        
        // Deduct for pending
        score -= (pending * 0.3f).coerceAtMost(20f)
        
        // Deduct heavily for urgent
        score -= (urgent * 5f).coerceAtMost(50f)
        
        return score.coerceIn(0f, 100f)
    }

    // ─── Action Items ─────────────────────────────────────────────────

    fun addActionItem(item: ActionItem) {
        actionItems.add(0, item)
        _dashboardData.value = _dashboardData.value.copy(pendingActions = actionItems.toList())
    }

    fun updateActionStatus(id: String, status: ActionStatus) {
        val index = actionItems.indexOfFirst { it.id == id }
        if (index >= 0) {
            actionItems[index] = actionItems[index].copy(status = status)
            _dashboardData.value = _dashboardData.value.copy(pendingActions = actionItems.toList())
        }
    }

    fun completeAction(id: String) {
        updateActionStatus(id, ActionStatus.COMPLETED)
    }

    fun deleteAction(id: String) {
        actionItems.removeAll { it.id == id }
        _dashboardData.value = _dashboardData.value.copy(pendingActions = actionItems.toList())
    }

    // ─── Alerts ─────────────────────────────────────────────────────

    fun markAlertRead(id: String) {
        val index = alerts.indexOfFirst { it.id == id }
        if (index >= 0) {
            alerts[index] = alerts[index].copy(isRead = true)
            _dashboardData.value = _dashboardData.value.copy(alerts = alerts.toList())
        }
    }

    fun dismissAlert(id: String) {
        alerts.removeAll { it.id == id }
        _dashboardData.value = _dashboardData.value.copy(alerts = alerts.toList())
    }

    fun getUnreadAlertCount(): Int = alerts.count { !it.isRead }

    // ─── Formatting ─────────────────────────────────────────────────

    fun formatAsDailyReport(): String {
        val data = _dashboardData.value
        val dateFormat = SimpleDateFormat("dd/MM/yyyy HH:mm", Locale.getDefault())
        val now = System.currentTimeMillis()

        return buildString {
            appendLine("📊 BIZCLAW DAILY REPORT")
            appendLine("Generated: ${dateFormat.format(Date(now))}")
            appendLine()
            appendLine("═══ TỔNG QUAN ═══")
            appendLine("🏥 Health Score: ${data.summary?.healthScore?.toInt() ?: 0}%")
            appendLine("📬 Unread: ${data.summary?.totalUnread ?: 0}")
            appendLine("📋 Pending: ${data.summary?.totalPending ?: 0}")
            appendLine("🚨 Urgent: ${data.summary?.urgentCount ?: 0}")
            appendLine()

            data.zaloMetrics?.let { z ->
                appendLine("═══ 💬 ZALO ═══")
                appendLine("• Chats mới: ${z.newToday}")
                appendLine("• Auto-reply: ${z.autoRepliesSent}")
                appendLine("• Escalated: ${z.escalatedToAgent}")
                appendLine("• CSAT: ${z.csatScore}/5")
                appendLine()
            }

            data.emailMetrics?.let { e ->
                appendLine("═══ 📧 EMAIL ═══")
                appendLine("• Tổng: ${e.totalEmails}")
                appendLine("• Unread: ${e.unreadEmails}")
                appendLine("• Urgent: ${e.urgentEmails}")
                e.topSender?.let { appendLine("• Top sender: $it") }
                appendLine()
            }

            data.socialMetrics?.let { s ->
                appendLine("═══ 📱 SOCIAL ═══")
                appendLine("• Posts today: ${s.postsToday}")
                appendLine("• Scheduled: ${s.scheduledPosts}")
                appendLine("• Total reach: ${s.totalReach}")
                appendLine()
            }

            if (data.pendingActions.isNotEmpty()) {
                appendLine("═══ ✅ PENDING ACTIONS ═══")
                data.pendingActions.take(5).forEach { action ->
                    appendLine("• [${action.priority.name}] ${action.title}")
                }
                appendLine()
            }

            if (data.alerts.isNotEmpty()) {
                appendLine("═══ ⚠️ ALERTS ═══")
                data.alerts.filter { !it.isRead }.take(5).forEach { alert ->
                    appendLine("• ${alert.title}: ${alert.message}")
                }
            }
        }
    }

    fun formatAsWhatsApp(): String {
        val data = _dashboardData.value

        return buildString {
            appendLine("*📊 BizClaw Daily Report*")
            appendLine()
            appendLine("*Tổng quan:*")
            appendLine("🏥 Health: ${data.summary?.healthScore?.toInt() ?: 0}%")
            appendLine("📬 Unread: ${data.summary?.totalUnread ?: 0}")
            appendLine("📋 Pending: ${data.summary?.totalPending ?: 0}")
            appendLine("🚨 Urgent: ${data.summary?.urgentCount ?: 0}")
            appendLine()
            appendLine("*Zalo:* ${data.zaloMetrics?.newToday ?: 0} new")
            appendLine("*Email:* ${data.emailMetrics?.unreadEmails ?: 0} unread")
            appendLine("*Social:* ${data.socialMetrics?.postsToday ?: 0} posts")
        }
    }
}
