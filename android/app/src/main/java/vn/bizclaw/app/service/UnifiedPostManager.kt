package vn.bizclaw.app.service

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.util.Base64
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.asRequestBody
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.ByteArrayOutputStream
import java.io.File
import java.text.SimpleDateFormat
import java.util.*

/**
 * Unified Post Manager - Multi-platform social media posting.
 * 
 * Supports:
 * - Facebook Pages & Groups
 * - Zalo Official Account
 * - Instagram (via Graph API)
 * - TikTok
 * - LinkedIn
 * - Custom webhooks
 */
class UnifiedPostManager(private val context: Context) {

    companion object {
        private const val TAG = "UnifiedPostManager"
    }

    data class PostRequest(
        val content: String,
        val mediaFiles: List<MediaFile> = emptyList(),
        val platforms: List<Platform> = listOf(Platform.FACEBOOK),
        val scheduledTime: Long? = null,
        val visibility: PostVisibility = PostVisibility.PUBLIC,
    )

    data class MediaFile(
        val filePath: String,
        val type: MediaType,
        val caption: String? = null,
    )

    enum class MediaType {
        IMAGE, VIDEO, GIF
    }

    enum class Platform {
        FACEBOOK_PAGE, FACEBOOK_GROUP, ZALO_OA, INSTAGRAM, TIKTOK, LINKEDIN, WEBHOOK
    }

    enum class PostVisibility {
        PUBLIC, FRIENDS, PRIVATE, UNLISTED
    }

    data class PostResult(
        val platform: Platform,
        val success: Boolean,
        val postId: String? = null,
        val postUrl: String? = null,
        val error: String? = null,
        val impressions: Int? = null,
        val reach: Int? = null,
    )

    data class ScheduledPost(
        val id: String,
        val request: PostRequest,
        val scheduledTime: Long,
        val status: ScheduleStatus,
        val results: List<PostResult> = emptyList(),
    )

    enum class ScheduleStatus {
        PENDING, POSTING, COMPLETED, FAILED, CANCELLED
    }

    data class PlatformConfig(
        val platform: Platform,
        val accessToken: String? = null,
        val pageId: String? = null,
        val webhookUrl: String? = null,
        val enabled: Boolean = true,
    )

    private val client = OkHttpClient.Builder()
        .connectTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(60, java.util.concurrent.TimeUnit.SECONDS)
        .writeTimeout(120, java.util.concurrent.TimeUnit.SECONDS)
        .build()

    private val platformConfigs = mutableMapOf<Platform, PlatformConfig>()
    private val scheduledPosts = mutableListOf<ScheduledPost>()

    /**
     * Configure a platform.
     */
    fun configurePlatform(config: PlatformConfig) {
        platformConfigs[config.platform] = config
    }

    /**
     * Get platform configurations.
     */
    fun getPlatformConfigs(): List<PlatformConfig> = platformConfigs.values.toList()

    /**
     * Post to all configured platforms.
     */
    suspend fun post(request: PostRequest): List<PostResult> = withContext(Dispatchers.IO) {
        request.platforms.map { platform ->
            try {
                postToPlatform(platform, request)
            } catch (e: Exception) {
                Log.e(TAG, "Post to $platform failed: ${e.message}")
                PostResult(
                    platform = platform,
                    success = false,
                    error = e.message,
                )
            }
        }
    }

    /**
     * Post to a specific platform.
     */
    private suspend fun postToPlatform(platform: Platform, request: PostRequest): PostResult {
        val config = platformConfigs[platform]
        if (config == null || !config.enabled) {
            return PostResult(
                platform = platform,
                success = false,
                error = "Platform not configured",
            )
        }

        return when (platform) {
            Platform.FACEBOOK_PAGE -> postToFacebookPage(config, request)
            Platform.FACEBOOK_GROUP -> postToFacebookGroup(config, request)
            Platform.ZALO_OA -> postToZaloOA(config, request)
            Platform.INSTAGRAM -> postToInstagram(config, request)
            Platform.LINKEDIN -> postToLinkedIn(config, request)
            Platform.WEBHOOK -> postToWebhook(config, request)
            Platform.TIKTOK -> PostResult(
                platform = platform,
                success = false,
                error = "TikTok API not yet supported",
            )
        }
    }

    /**
     * Post to Facebook Page.
     */
    private suspend fun postToFacebookPage(config: PlatformConfig, request: PostRequest): PostResult {
        val token = config.accessToken ?: return PostResult(
            platform = Platform.FACEBOOK_PAGE,
            success = false,
            error = "No access token",
        )
        val pageId = config.pageId ?: return PostResult(
            platform = Platform.FACEBOOK_PAGE,
            success = false,
            error = "No page ID",
        )

        val content = adaptContentForFacebook(request.content)

        // If has media, post with photo
        if (request.mediaFiles.isNotEmpty()) {
            return postFacebookWithMedia(token, pageId, request.mediaFiles.first(), content)
        }

        // Text-only post
        val formBody = FormBody.Builder()
            .add("message", content)
            .add("access_token", token)
            .build()

        val httpRequest = Request.Builder()
            .url("https://graph.facebook.com/v19.0/$pageId/feed")
            .post(formBody)
            .build()

        val response = client.newCall(httpRequest).execute()
        val body = response.body?.string() ?: return PostResult(
            platform = Platform.FACEBOOK_PAGE,
            success = false,
            error = "Empty response",
        )

        return if (response.isSuccessful) {
            val json = kotlinx.serialization.json.Json { ignoreUnknownKeys = true }
            try {
                val result = json.decodeFromString<FacebookPostResponse>(body)
                PostResult(
                    platform = Platform.FACEBOOK_PAGE,
                    success = true,
                    postId = result.id,
                    postUrl = "https://facebook.com/$pageId/posts/${result.id}",
                )
            } catch (e: Exception) {
                PostResult(
                    platform = Platform.FACEBOOK_PAGE,
                    success = false,
                    error = "Parse error: ${e.message}",
                )
            }
        } else {
            PostResult(
                platform = Platform.FACEBOOK_PAGE,
                success = false,
                error = body,
            )
        }
    }

    private suspend fun postFacebookWithMedia(
        token: String,
        pageId: String,
        media: MediaFile,
        caption: String,
    ): PostResult = withContext(Dispatchers.IO) {
        val file = File(media.filePath)
        if (!file.exists()) {
            return@withContext PostResult(
                platform = Platform.FACEBOOK_PAGE,
                success = false,
                error = "File not found: ${media.filePath}",
            )
        }

        val mediaType = when (media.type) {
            MediaType.IMAGE -> "image/jpeg"
            MediaType.VIDEO -> "video/mp4"
            MediaType.GIF -> "image/gif"
        }

        val requestBody = MultipartBody.Builder()
            .setType(MultipartBody.FORM)
            .addFormDataPart(
                "source",
                file.name,
                file.asRequestBody(mediaType.toMediaType()),
            )
            .addFormDataPart("access_token", token)
            .build()

        val request = Request.Builder()
            .url("https://graph.facebook.com/v19.0/$pageId/photos")
            .post(requestBody)
            .build()

        val response = client.newCall(request).execute()
        val body = response.body?.string() ?: return@withContext PostResult(
            platform = Platform.FACEBOOK_PAGE,
            success = false,
            error = "Empty response",
        )

        if (response.isSuccessful) {
            PostResult(
                platform = Platform.FACEBOOK_PAGE,
                success = true,
                postId = body,
                postUrl = "https://facebook.com/$pageId/photos",
            )
        } else {
            PostResult(
                platform = Platform.FACEBOOK_PAGE,
                success = false,
                error = body,
            )
        }
    }

    /**
     * Post to Facebook Group.
     */
    private suspend fun postToFacebookGroup(config: PlatformConfig, request: PostRequest): PostResult {
        // Similar to page posting but with group ID
        return postToFacebookPage(config, request).copy(platform = Platform.FACEBOOK_GROUP)
    }

    /**
     * Post to Zalo OA.
     */
    private suspend fun postToZaloOA(config: PlatformConfig, request: PostRequest): PostResult {
        val accessToken = config.accessToken ?: return PostResult(
            platform = Platform.ZALO_OA,
            success = false,
            error = "No access token",
        )

        val content = adaptContentForZalo(request.content)

        val jsonBody = """
        {
            "message": {
                "text": "$content"
            }
        }
        """.trimIndent()

        val requestBody = jsonBody.toRequestBody("application/json".toMediaType())

        val httpRequest = Request.Builder()
            .url("https://openapi.zalo.me/v3.0/oa/message/text")
            .post(requestBody)
            .addHeader("access_token", accessToken)
            .build()

        val response = client.newCall(httpRequest).execute()
        val body = response.body?.string() ?: return PostResult(
            platform = Platform.ZALO_OA,
            success = false,
            error = "Empty response",
        )

        return if (response.isSuccessful) {
            PostResult(
                platform = Platform.ZALO_OA,
                success = true,
                postId = "zalo_${System.currentTimeMillis()}",
                postUrl = null,
            )
        } else {
            PostResult(
                platform = Platform.ZALO_OA,
                success = false,
                error = body,
            )
        }
    }

    /**
     * Post to Instagram (via Graph API).
     */
    private suspend fun postToInstagram(config: PlatformConfig, request: PostRequest): PostResult {
        val token = config.accessToken ?: return PostResult(
            platform = Platform.INSTAGRAM,
            success = false,
            error = "No access token",
        )
        val pageId = config.pageId ?: return PostResult(
            platform = Platform.INSTAGRAM,
            success = false,
            error = "No IG Business Account ID",
        )

        val caption = adaptContentForInstagram(request.content)

        // Instagram requires photo upload to FB Page first, then create IG container
        if (request.mediaFiles.isEmpty()) {
            return PostResult(
                platform = Platform.INSTAGRAM,
                success = false,
                error = "Instagram posts require an image",
            )
        }

        val media = request.mediaFiles.first()
        val file = File(media.filePath)
        if (!file.exists()) {
            return PostResult(
                platform = Platform.INSTAGRAM,
                success = false,
                error = "File not found",
            )
        }

        // Create media container
        val containerBody = FormBody.Builder()
            .add("image_url", "file://${file.absolutePath}")
            .add("caption", caption)
            .add("access_token", token)
            .build()

        val containerRequest = Request.Builder()
            .url("https://graph.facebook.com/v19.0/$pageId/media")
            .post(containerBody)
            .build()

        val containerResponse = client.newCall(containerRequest).execute()
        val containerJson = containerResponse.body?.string() ?: return PostResult(
            platform = Platform.INSTAGRAM,
            success = false,
            error = "Container creation failed",
        )

        if (!containerResponse.isSuccessful) {
            return PostResult(
                platform = Platform.INSTAGRAM,
                success = false,
                error = containerJson,
            )
        }

        val json = kotlinx.serialization.json.Json { ignoreUnknownKeys = true }
        val container = json.decodeFromString<InstagramContainerResponse>(containerJson)

        // Publish media
        val publishBody = FormBody.Builder()
            .add("creation_id", container.id)
            .add("access_token", token)
            .build()

        val publishRequest = Request.Builder()
            .url("https://graph.facebook.com/v19.0/$pageId/media_publish")
            .post(publishBody)
            .build()

        val publishResponse = client.newCall(publishRequest).execute()

        return if (publishResponse.isSuccessful) {
            PostResult(
                platform = Platform.INSTAGRAM,
                success = true,
                postId = container.id,
                postUrl = null,
            )
        } else {
            PostResult(
                platform = Platform.INSTAGRAM,
                success = false,
                error = publishResponse.body?.string(),
            )
        }
    }

    /**
     * Post to LinkedIn.
     */
    private suspend fun postToLinkedIn(config: PlatformConfig, request: PostRequest): PostResult {
        val token = config.accessToken ?: return PostResult(
            platform = Platform.LINKEDIN,
            success = false,
            error = "No access token",
        )
        val pageId = config.pageId ?: return PostResult(
            platform = Platform.LINKEDIN,
            success = false,
            error = "No LinkedIn UGC Post ID",
        )

        val content = adaptContentForLinkedIn(request.content)

        val jsonBody = """
        {
            "author": "$pageId",
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": {
                        "text": "$content"
                    },
                    "shareMediaCategory": "NONE"
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": "PUBLIC"
            }
        }
        """.trimIndent()

        val requestBody = jsonBody.toRequestBody("application/json".toMediaType())

        val httpRequest = Request.Builder()
            .url("https://api.linkedin.com/v2/ugcPosts")
            .post(requestBody)
            .addHeader("Authorization", "Bearer $token")
            .addHeader("X-Restli-Protocol-Version", "2.0.0")
            .build()

        val response = client.newCall(httpRequest).execute()
        val body = response.body?.string() ?: return PostResult(
            platform = Platform.LINKEDIN,
            success = false,
            error = "Empty response",
        )

        return if (response.isSuccessful) {
            PostResult(
                platform = Platform.LINKEDIN,
                success = true,
                postId = "linkedin_${System.currentTimeMillis()}",
                postUrl = null,
            )
        } else {
            PostResult(
                platform = Platform.LINKEDIN,
                success = false,
                error = body,
            )
        }
    }

    /**
     * Post to custom webhook.
     */
    private suspend fun postToWebhook(config: PlatformConfig, request: PostRequest): PostResult {
        val url = config.webhookUrl ?: return PostResult(
            platform = Platform.WEBHOOK,
            success = false,
            error = "No webhook URL",
        )

        val jsonBody = """
        {
            "content": "${request.content.replace("\"", "\\\"")}",
            "platforms": ${request.platforms.map { it.name }},
            "timestamp": ${System.currentTimeMillis()}
        }
        """.trimIndent()

        val requestBody = jsonBody.toRequestBody("application/json".toMediaType())

        val httpRequest = Request.Builder()
            .url(url)
            .post(requestBody)
            .build()

        val response = client.newCall(httpRequest).execute()

        return PostResult(
            platform = Platform.WEBHOOK,
            success = response.isSuccessful,
            postId = "webhook_${System.currentTimeMillis()}",
            error = if (!response.isSuccessful) response.body?.string() else null,
        )
    }

    /**
     * Schedule a post for later.
     */
    fun schedulePost(request: PostRequest): String {
        val id = "sched_${System.currentTimeMillis()}"
        val scheduled = ScheduledPost(
            id = id,
            request = request,
            scheduledTime = request.scheduledTime ?: throw IllegalArgumentException("scheduledTime required"),
            status = ScheduleStatus.PENDING,
        )
        scheduledPosts.add(scheduled)
        return id
    }

    /**
     * Get scheduled posts.
     */
    fun getScheduledPosts(): List<ScheduledPost> = scheduledPosts.toList()

    /**
     * Cancel a scheduled post.
     */
    fun cancelScheduledPost(id: String): Boolean {
        val post = scheduledPosts.find { it.id == id } ?: return false
        val index = scheduledPosts.indexOf(post)
        scheduledPosts[index] = post.copy(status = ScheduleStatus.CANCELLED)
        return true
    }

    // ─── Content Adapters ─────────────────────────────────────────────────

    private fun adaptContentForFacebook(content: String): String {
        var adapted = content
        if (adapted.length > 632) {
            adapted = adapted.take(629) + "..."
        }
        return adapted
    }

    private fun adaptContentForZalo(content: String): String {
        var adapted = content
        if (adapted.length > 2000) {
            adapted = adapted.take(1997) + "..."
        }
        return adapted
    }

    private fun adaptContentForInstagram(content: String): String {
        // Instagram: no links, max 2200 chars, use hashtags
        var adapted = content
        if (adapted.length > 2200) {
            adapted = adapted.take(2197) + "..."
        }
        // Remove URLs for Instagram
        adapted = adapted.replace(Regex("https?://[^\\s]+"), "")
        return adapted
    }

    private fun adaptContentForLinkedIn(content: String): String {
        // LinkedIn: professional tone, max 3000 chars
        var adapted = content
        if (adapted.length > 3000) {
            adapted = adapted.take(2997) + "..."
        }
        return adapted
    }
}

// ─── Response Models ─────────────────────────────────────────────────────────

@kotlinx.serialization.Serializable
data class FacebookPostResponse(
    val id: String = "",
    val post_id: String = "",
)

@kotlinx.serialization.Serializable
data class InstagramContainerResponse(
    val id: String = "",
    val uri: String = "",
)
