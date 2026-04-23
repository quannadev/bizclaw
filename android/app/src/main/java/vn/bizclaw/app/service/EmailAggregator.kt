package vn.bizclaw.app.service

import android.content.Context
import android.util.Base64
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import java.text.SimpleDateFormat
import java.util.*
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec

/**
 * Email Aggregator - Multi-account email fetching and digest generation.
 * 
 * Supports:
 * - Gmail via OAuth2
 * - Outlook via OAuth2
 * - Custom IMAP/SMTP servers
 * - Yahoo Mail
 */
class EmailAggregator(private val context: Context) {

    companion object {
        private const val TAG = "EmailAggregator"
        private const val GMAIL_SCOPE = "https://www.googleapis.com/auth/gmail.readonly"
        private const val OUTLOOK_SCOPE = "https://graph.microsoft.com/Mail.Read"
    }

    data class EmailAccount(
        val id: String,
        val email: String,
        val provider: EmailProvider,
        val accessToken: String? = null,
        val refreshToken: String? = null,
        val lastSync: Long = 0,
    )

    enum class EmailProvider {
        GMAIL, OUTLOOK, YAHOO, IMAP, YAHOO_IMAP
    }

    data class EmailMessage(
        val id: String,
        val threadId: String?,
        val subject: String,
        val from: String,
        val fromName: String?,
        val to: List<String>,
        val cc: List<String> = emptyList(),
        val body: String,
        val bodyHtml: String? = null,
        val date: Long,
        val isRead: Boolean,
        val isStarred: Boolean,
        val labels: List<String> = emptyList(),
        val hasAttachments: Boolean,
        val priority: EmailPriority = EmailPriority.NORMAL,
        val accountId: String,
    )

    enum class EmailPriority {
        URGENT, HIGH, NORMAL, LOW
    }

    data class EmailSummary(
        val totalEmails: Int,
        val unreadEmails: Int,
        val urgentEmails: Int,
        val bySender: Map<String, Int>,
        val byLabel: Map<String, Int>,
        val threads: List<EmailThread>,
        val generatedAt: Long = System.currentTimeMillis(),
    )

    data class EmailThread(
        val threadId: String,
        val subject: String,
        val messages: List<EmailMessage>,
        val participantCount: Int,
        val lastMessageDate: Long,
        val isRead: Boolean,
    )

    data class DigestTemplate(
        val title: String = "📧 Email Digest",
        val includeUrgent: Boolean = true,
        val includeBySender: Boolean = true,
        val includeThreads: Boolean = true,
        val maxItems: Int = 10,
    )

    private val client = OkHttpClient.Builder()
        .connectTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(60, java.util.concurrent.TimeUnit.SECONDS)
        .build()

    private val accounts = mutableMapOf<String, EmailAccount>()

    /**
     * Add an email account.
     */
    fun addAccount(account: EmailAccount) {
        accounts[account.id] = account
    }

    /**
     * Remove an account.
     */
    fun removeAccount(accountId: String) {
        accounts.remove(accountId)
    }

    /**
     * Get all accounts.
     */
    fun getAccounts(): List<EmailAccount> = accounts.values.toList()

    /**
     * Fetch emails from all accounts.
     */
    suspend fun fetchAllEmails(
        since: Long? = null,
        maxResults: Int = 50,
    ): List<EmailMessage> = withContext(Dispatchers.IO) {
        accounts.values.mapNotNull { account ->
            try {
                fetchEmails(account, since, maxResults)
            } catch (e: Exception) {
                Log.e(TAG, "Failed to fetch from ${account.email}: ${e.message}")
                null
            }
        }.flatten()
    }

    /**
     * Fetch emails from a specific account.
     */
    suspend fun fetchEmails(
        account: EmailAccount,
        since: Long? = null,
        maxResults: Int = 50,
    ): List<EmailMessage> = withContext(Dispatchers.IO) {
        when (account.provider) {
            EmailProvider.GMAIL -> fetchGmailEmails(account, since, maxResults)
            EmailProvider.OUTLOOK -> fetchOutlookEmails(account, since, maxResults)
            EmailProvider.IMAP, EmailProvider.YAHOO_IMAP -> fetchImapEmails(account, since, maxResults)
            else -> emptyList()
        }
    }

    /**
     * Fetch from Gmail API.
     */
    private suspend fun fetchGmailEmails(
        account: EmailAccount,
        since: Long?,
        maxResults: Int,
    ): List<EmailMessage> = withContext(Dispatchers.IO) {
        val token = account.accessToken ?: return@withContext emptyList()
        
        var url = "https://gmail.googleapis.com/gmail/v1/users/me/messages?maxResults=$maxResults"
        if (since != null) {
            val afterDate = SimpleDateFormat("yyyy/MM/dd", Locale.US).format(Date(since))
            url += "&after=$afterDate"
        }

        val request = Request.Builder()
            .url(url)
            .addHeader("Authorization", "Bearer $token")
            .get()
            .build()

        val response = client.newCall(request).execute()
        if (!response.isSuccessful) return@withContext emptyList()

        val body = response.body?.string() ?: return@withContext emptyList()
        val json = kotlinx.serialization.json.Json { ignoreUnknownKeys = true }
        
        try {
            val listResponse = json.decodeFromString<GmailListResponse>(body)
            listResponse.messages?.mapNotNull { msgRef ->
                getGmailMessageDetail(token, msgRef.id, account.id)
            } ?: emptyList()
        } catch (e: Exception) {
            Log.e(TAG, "Gmail parse error: ${e.message}")
            emptyList()
        }
    }

    /**
     * Get single Gmail message details.
     */
    private suspend fun getGmailMessageDetail(
        token: String,
        messageId: String,
        accountId: String,
    ): EmailMessage? = withContext(Dispatchers.IO) {
        val request = Request.Builder()
            .url("https://gmail.googleapis.com/gmail/v1/users/me/messages/$messageId?format=full")
            .addHeader("Authorization", "Bearer $token")
            .get()
            .build()

        val response = client.newCall(request).execute()
        if (!response.isSuccessful) return@withContext null

        val body = response.body?.string() ?: return@withContext null
        parseGmailMessage(body, accountId)
    }

    private fun parseGmailMessage(jsonStr: String, accountId: String): EmailMessage? {
        return try {
            val json = kotlinx.serialization.json.Json { ignoreUnknownKeys = true }
            val msg = json.decodeFromString<GmailMessage>(jsonStr)
            
            val headers = msg.payload?.headers?.associate { it.name to it.value } ?: emptyMap()
            
            EmailMessage(
                id = msg.id ?: return null,
                threadId = msg.threadId,
                subject = headers["Subject"] ?: "(No Subject)",
                from = headers["From"] ?: "",
                fromName = headers["From"]?.let { extractNameFromEmail(it) },
                to = headers["To"]?.split(",")?.map { it.trim() } ?: emptyList(),
                cc = headers["Cc"]?.split(",")?.map { it.trim() } ?: emptyList(),
                body = decodeBody(msg.payload),
                bodyHtml = msg.payload?.body?.data?.let {
                    String(Base64.getUrlDecoder().decode(it))
                },
                date = msg.internalDate?.toLongOrNull() ?: System.currentTimeMillis(),
                isRead = msg.labelIds?.contains("UNREAD") == false,
                isStarred = msg.labelIds?.contains("STARRED") == true,
                labels = msg.labelIds ?: emptyList(),
                hasAttachments = msg.payload?.hasAttachments == true,
                priority = detectPriority(headers),
                accountId = accountId,
            )
        } catch (e: Exception) {
            Log.e(TAG, "Parse Gmail message error: ${e.message}")
            null
        }
    }

    /**
     * Fetch from Outlook/Microsoft Graph API.
     */
    private suspend fun fetchOutlookEmails(
        account: EmailAccount,
        since: Long?,
        maxResults: Int,
    ): List<EmailMessage> = withContext(Dispatchers.IO) {
        val token = account.accessToken ?: return@withContext emptyList()

        var url = "https://graph.microsoft.com/v1.0/me/messages?\$top=$maxResults&\$orderby=receivedDateTime desc"
        if (since != null) {
            val afterDate = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", Locale.US).format(Date(since))
            url += "&\$filter=receivedDateTime ge $afterDate"
        }

        val request = Request.Builder()
            .url(url)
            .addHeader("Authorization", "Bearer $token")
            .get()
            .build()

        val response = client.newCall(request).execute()
        if (!response.isSuccessful) return@withContext emptyList()

        val body = response.body?.string() ?: return@withContext emptyList()
        val json = kotlinx.serialization.json.Json { ignoreUnknownKeys = true }
        
        try {
            val graphResponse = json.decodeFromString<OutlookResponse>(body)
            graphResponse.value?.map { msg ->
                EmailMessage(
                    id = msg.id ?: return@map null,
                    threadId = msg.conversationId,
                    subject = msg.subject ?: "(No Subject)",
                    from = msg.from?.emailAddress?.address ?: "",
                    fromName = msg.from?.emailAddress?.name,
                    to = msg.toRecipients?.map { it.emailAddress?.address ?: "" } ?: emptyList(),
                    cc = msg.ccRecipients?.map { it.emailAddress?.address ?: "" } ?: emptyList(),
                    body = msg.body?.content ?: "",
                    bodyHtml = msg.body?.contentType?.contains("html") == true,
                    date = parseOutlookDate(msg.receivedDateTime),
                    isRead = msg.isRead != false,
                    isStarred = false,
                    labels = emptyList(),
                    hasAttachments = msg.hasAttachments == true,
                    priority = if (msg.importance == "high") EmailPriority.HIGH else EmailPriority.NORMAL,
                    accountId = account.id,
                )
            } ?: emptyList()
        } catch (e: Exception) {
            Log.e(TAG, "Outlook parse error: ${e.message}")
            emptyList()
        }
    }

    /**
     * Fetch from IMAP server (Gmail/Outlook/Yahoo).
     */
    private suspend fun fetchImapEmails(
        account: EmailAccount,
        since: Long?,
        maxResults: Int,
    ): List<EmailMessage> = withContext(Dispatchers.IO) {
        // IMAP fetching requires native library or JavaMail
        // For now, return empty - can be implemented with javamail-android
        Log.w(TAG, "IMAP fetch not yet implemented - use OAuth2 APIs")
        emptyList()
    }

    /**
     * Generate summary from emails.
     */
    fun generateSummary(emails: List<EmailMessage>): EmailSummary {
        return EmailSummary(
            totalEmails = emails.size,
            unreadEmails = emails.count { !it.isRead },
            urgentEmails = emails.count { it.priority == EmailPriority.URGENT || it.priority == EmailPriority.HIGH },
            bySender = emails.groupBy { it.from }
                .mapValues { it.value.size }
                .entries
                .sortedByDescending { it.value }
                .take(10)
                .associate { it.key to it.value },
            byLabel = emails.flatMap { it.labels }
                .groupingBy { it }
                .eachCount(),
            threads = groupIntoThreads(emails),
            generatedAt = System.currentTimeMillis(),
        )
    }

    /**
     * Group emails into threads.
     */
    private fun groupIntoThreads(emails: List<EmailMessage>): List<EmailThread> {
        return emails
            .groupBy { it.threadId ?: it.id }
            .filter { it.key != null }
            .map { (_, msgs) ->
                val sorted = msgs.sortedBy { it.date }
                EmailThread(
                    threadId = it.key ?: "",
                    subject = sorted.last().subject,
                    messages = sorted,
                    participantCount = sorted.map { it.from }.distinct().size,
                    lastMessageDate = sorted.last().date,
                    isRead = sorted.last().isRead,
                )
            }
            .sortedByDescending { it.lastMessageDate }
    }

    /**
     * Generate digest text.
     */
    fun generateDigest(
        emails: List<EmailMessage>,
        template: DigestTemplate = DigestTemplate(),
    ): String {
        val summary = generateSummary(emails)
        val dateFormat = SimpleDateFormat("HH:mm dd/MM", Locale.getDefault())

        return buildString {
            appendLine("${template.title} - ${dateFormat.format(Date())}")
            appendLine()
            appendLine("═══ TỔNG QUAN ═══")
            appendLine("📧 Tổng email: ${summary.totalEmails}")
            appendLine("📬 Chưa đọc: ${summary.unreadEmails}")
            if (template.includeUrgent) {
                appendLine("🚨 Urgent: ${summary.urgentEmails}")
            }
            appendLine()

            if (template.includeUrgent && summary.urgentEmails > 0) {
                appendLine("═══ 🚨 URGENT ═══")
                emails.filter { it.priority == EmailPriority.URGENT || it.priority == EmailPriority.HIGH }
                    .take(template.maxItems)
                    .forEach { email ->
                        appendLine("• [${email.priority.name}] ${email.subject}")
                        appendLine("  Từ: ${email.fromName ?: email.from}")
                        appendLine("  ${dateFormat.format(Date(email.date))}")
                    }
                appendLine()
            }

            if (template.includeBySender && summary.bySender.isNotEmpty()) {
                appendLine("═══ 👤 TOP NGƯỜI GỬI ═══")
                summary.bySender.entries.take(5).forEach { (sender, count) ->
                    appendLine("• $sender: $count email")
                }
                appendLine()
            }

            if (template.includeThreads && summary.threads.isNotEmpty()) {
                appendLine("═══ 💬 THREADS MỚI ═══")
                summary.threads.take(template.maxItems).forEach { thread ->
                    val unread = if (!thread.isRead) " [NEW]" else ""
                    appendLine("• ${thread.subject}$unread")
                    appendLine("  ${thread.messages.size} emails • ${thread.participantCount} người")
                }
            }
        }
    }

    private fun decodeBody(payload: GmailMessage.Payload?): String {
        if (payload == null) return ""
        val data = payload.body?.data ?: return ""
        return try {
            String(Base64.getUrlDecoder().decode(data))
        } catch (e: Exception) {
            ""
        }
    }

    private fun extractNameFromEmail(from: String): String? {
        val match = Regex("\"?([^\"<]+)\"?\\s*<").find(from)
        return match?.groupValues?.get(1)?.trim()
    }

    private fun detectPriority(headers: Map<String, String>): EmailPriority {
        val subject = headers["Subject"]?.lowercase() ?: ""
        val importance = headers["Importance"] ?: ""
        
        return when {
            importance == "high" || subject.contains("urgent") || subject.contains("gấp") -> EmailPriority.URGENT
            subject.contains("important") || subject.contains("priority") -> EmailPriority.HIGH
            subject.contains("[fwd]") || subject.contains("[fw:") -> EmailPriority.LOW
            else -> EmailPriority.NORMAL
        }
    }

    private fun parseOutlookDate(dateStr: String?): Long {
        if (dateStr == null) return System.currentTimeMillis()
        return try {
            SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss", Locale.US).parse(dateStr)?.time
                ?: System.currentTimeMillis()
        } catch (e: Exception) {
            System.currentTimeMillis()
        }
    }
}

// ─── Response Models ─────────────────────────────────────────────────────────

@kotlinx.serialization.Serializable
data class GmailListResponse(
    val messages: List<GmailMessageRef>? = null,
    val resultSizeEstimate: Int = 0,
)

@kotlinx.serialization.Serializable
data class GmailMessageRef(
    val id: String? = null,
)

@kotlinx.serialization.Serializable
data class GmailMessage(
    val id: String? = null,
    val threadId: String? = null,
    val snippet: String? = null,
    val payload: GmailPayload? = null,
    val internalDate: String? = null,
    val labelIds: List<String>? = null,
)

@kotlinx.serialization.Serializable
data class GmailPayload(
    val headers: List<GmailHeader>? = null,
    val body: GmailBody? = null,
    val parts: List<GmailPart>? = null,
    val mimeType: String? = null,
    val hasAttachments: Boolean = false,
)

@kotlinx.serialization.Serializable
data class GmailHeader(
    val name: String = "",
    val value: String = "",
)

@kotlinx.serialization.Serializable
data class GmailBody(
    val data: String? = null,
    val size: Int = 0,
)

@kotlinx.serialization.Serializable
data class GmailPart(
    val mimeType: String? = null,
    val body: GmailBody? = null,
    val parts: List<GmailPart>? = null,
)

@kotlinx.serialization.Serializable
data class OutlookResponse(
    val value: List<OutlookMessage>? = null,
)

@kotlinx.serialization.Serializable
data class OutlookMessage(
    val id: String? = null,
    val conversationId: String? = null,
    val subject: String? = null,
    val from: OutlookSender? = null,
    val toRecipients: List<OutlookRecipient>? = null,
    val ccRecipients: List<OutlookRecipient>? = null,
    val body: OutlookBody? = null,
    val receivedDateTime: String? = null,
    val sentDateTime: String? = null,
    val isRead: Boolean? = null,
    val importance: String? = null,
    val hasAttachments: Boolean? = null,
)

@kotlinx.serialization.Serializable
data class OutlookSender(
    val emailAddress: OutlookEmail? = null,
)

@kotlinx.serialization.Serializable
data class OutlookRecipient(
    val emailAddress: OutlookEmail? = null,
)

@kotlinx.serialization.Serializable
data class OutlookEmail(
    val name: String? = null,
    val address: String? = null,
)

@kotlinx.serialization.Serializable
data class OutlookBody(
    val contentType: String? = null,
    val content: String? = null,
)
