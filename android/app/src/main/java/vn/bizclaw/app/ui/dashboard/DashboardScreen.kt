package vn.bizclaw.app.ui.dashboard

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.launch
import vn.bizclaw.app.service.*
import java.text.SimpleDateFormat
import java.util.*

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val capabilities = remember { DeviceCapabilities(context) }
    var isDaemonRunning by remember { mutableStateOf(BizClawDaemonService.isRunning()) }

    // Business Dashboard
    val dashboard = remember { UnifiedDashboard(context) }
    var dashboardData by remember { mutableStateOf<UnifiedDashboard.DashboardData?>(null) }
    var isRefreshing by remember { mutableStateOf(false) }

    // Initialize and refresh
    LaunchedEffect(Unit) {
        dashboard.initialize(
            zaloAggregator = ZaloGroupAggregator(context),
            emailAggregator = EmailAggregator(context),
            postManager = UnifiedPostManager(context),
            smartAutoReply = SmartAutoReply(context),
        )
        dashboard.refresh()
        dashboardData = dashboard.dashboardData.value
    }

    // Auto-refresh
    LaunchedEffect(dashboard.dashboardData) {
        dashboardData = dashboard.dashboardData.value
    }

    val battery = remember { capabilities.getBatteryInfo() }
    val storage = remember { capabilities.getStorageInfo() }
    val network = remember { capabilities.getNetworkInfo() }
    val device = remember { capabilities.getDeviceInfo() }
    val oemWarning = remember { capabilities.getOemBatteryKillerWarning() }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DashboardScreen(
    onBack: () -> Unit,
) {
    val context = LocalContext.current
    val capabilities = remember { DeviceCapabilities(context) }
    var isDaemonRunning by remember { mutableStateOf(BizClawDaemonService.isRunning()) }

    // Auto-refresh device info
    val battery = remember { capabilities.getBatteryInfo() }
    val storage = remember { capabilities.getStorageInfo() }
    val network = remember { capabilities.getNetworkInfo() }
    val device = remember { capabilities.getDeviceInfo() }
    val oemWarning = remember { capabilities.getOemBatteryKillerWarning() }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Bảng Điều Khiển", fontWeight = FontWeight.Bold) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, "Quay lại")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            // ─── Daemon Control ───────────────────────────────────────

            Card(
                colors = CardDefaults.cardColors(
                    containerColor = if (isDaemonRunning)
                        MaterialTheme.colorScheme.primaryContainer
                    else
                        MaterialTheme.colorScheme.surfaceVariant,
                ),
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(20.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(
                        if (isDaemonRunning) "🤖" else "😴",
                        fontSize = 48.sp,
                    )
                    Spacer(Modifier.height(8.dp))
                    Text(
                        if (isDaemonRunning) "Agent đang chạy" else "Agent đã dừng",
                        style = MaterialTheme.typography.headlineSmall,
                        fontWeight = FontWeight.Bold,
                    )
                    Spacer(Modifier.height(16.dp))

                    Button(
                        onClick = {
                            if (isDaemonRunning) {
                                BizClawDaemonService.stop(context)
                            } else {
                                BizClawDaemonService.start(context)
                            }
                            isDaemonRunning = !isDaemonRunning
                        },
                        colors = ButtonDefaults.buttonColors(
                            containerColor = if (isDaemonRunning)
                                MaterialTheme.colorScheme.error
                            else
                                MaterialTheme.colorScheme.primary,
                        ),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Icon(
                            if (isDaemonRunning) Icons.Default.Stop else Icons.Default.PlayArrow,
                            null,
                        )
                        Spacer(Modifier.width(8.dp))
                        Text(if (isDaemonRunning) "Dừng Agent" else "Khởi động Agent")
                    }
                }
            }

            // ─── OEM Warning ──────────────────────────────────────────

            if (oemWarning != null) {
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.tertiaryContainer,
                    ),
                ) {
                    Row(
                        modifier = Modifier.padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(
                            Icons.Default.Warning,
                            null,
                            tint = MaterialTheme.colorScheme.tertiary,
                        )
                        Spacer(Modifier.width(12.dp))
                        Text(
                            oemWarning,
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
            }

            // ─── Device Stats Grid ────────────────────────────────────

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                StatCard(
                    modifier = Modifier.weight(1f),
                    icon = Icons.Default.BatteryChargingFull,
                    label = "Pin",
                    value = "${battery.level}%",
                    subtext = if (battery.isCharging) "Đang sạc" else "${battery.temperatureCelsius}°C",
                    color = when {
                        battery.level > 50 -> MaterialTheme.colorScheme.secondary
                        battery.level > 20 -> MaterialTheme.colorScheme.tertiary
                        else -> MaterialTheme.colorScheme.error
                    },
                )
                StatCard(
                    modifier = Modifier.weight(1f),
                    icon = Icons.Default.Storage,
                    label = "Bộ nhớ",
                    value = "${storage.usedPercent}%",
                    subtext = "%.1f GB trống".format(storage.freeGb),
                    color = if (storage.usedPercent < 80)
                        MaterialTheme.colorScheme.secondary
                    else
                        MaterialTheme.colorScheme.error,
                )
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                StatCard(
                    modifier = Modifier.weight(1f),
                    icon = Icons.Default.Wifi,
                    label = "Mạng",
                    value = network.type.uppercase(),
                    subtext = network.wifiSsid ?: if (network.isConnected) "Đã kết nối" else "Ngoại tuyến",
                    color = if (network.isConnected)
                        MaterialTheme.colorScheme.secondary
                    else
                        MaterialTheme.colorScheme.error,
                )
                StatCard(
                    modifier = Modifier.weight(1f),
                    icon = Icons.Default.Memory,
                    label = "CPU",
                    value = "${device.cpuCores} nhân",
                    subtext = "${device.freeRamMb} MB RAM trống",
                    color = MaterialTheme.colorScheme.primary,
                )
            }

            // ─── Device Info ──────────────────────────────────────────

            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                ),
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Text(
                        "Thiết bị",
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold,
                    )
                    Spacer(Modifier.height(8.dp))
                    InfoRow("Hãng", device.manufacturer)
                    InfoRow("Model", device.model)
                    InfoRow("Android", "${device.androidVersion} (SDK ${device.sdkVersion})")
                    InfoRow("BizClaw", "v0.6.1")
                }
            }

            // ─── Business Metrics (v0.6.1) ───────────────────────────

            Card(
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f),
                ),
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            "📊 Tổng Quan Kinh Doanh",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                        )
                        IconButton(
                            onClick = {
                                isRefreshing = true
                                scope.launch {
                                    dashboard.refresh()
                                    isRefreshing = false
                                }
                            },
                            enabled = !isRefreshing,
                        ) {
                            if (isRefreshing) {
                                CircularProgressIndicator(modifier = Modifier.size(20.dp), strokeWidth = 2.dp)
                            } else {
                                Icon(Icons.Default.Refresh, "Refresh")
                            }
                        }
                    }

                    Spacer(Modifier.height(12.dp))

                    // Summary Stats
                    dashboardData?.summary?.let { summary ->
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            MiniStatCard(
                                modifier = Modifier.weight(1f),
                                emoji = "📬",
                                value = "${summary.totalUnread}",
                                label = "Unread",
                                color = if (summary.totalUnread > 10) Color(0xFFFF5722) else MaterialTheme.colorScheme.primary,
                            )
                            MiniStatCard(
                                modifier = Modifier.weight(1f),
                                emoji = "📋",
                                value = "${summary.totalPending}",
                                label = "Pending",
                                color = if (summary.totalPending > 5) Color(0xFFFF9800) else MaterialTheme.colorScheme.secondary,
                            )
                            MiniStatCard(
                                modifier = Modifier.weight(1f),
                                emoji = "🚨",
                                value = "${summary.urgentCount}",
                                label = "Urgent",
                                color = if (summary.urgentCount > 0) Color(0xFFF44336) else Color(0xFF4CAF50),
                            )
                            MiniStatCard(
                                modifier = Modifier.weight(1f),
                                emoji = "🏥",
                                value = "${summary.healthScore.toInt()}%",
                                label = "Health",
                                color = when {
                                    summary.healthScore >= 80 -> Color(0xFF4CAF50)
                                    summary.healthScore >= 50 -> Color(0xFFFF9800)
                                    else -> Color(0xFFF44336)
                                },
                            )
                        }
                    }
                }
            }

            // ─── Zalo Metrics ───────────────────────────────────────

            dashboardData?.zaloMetrics?.let { zalo ->
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = Color(0xFF0068FF).copy(alpha = 0.08f),
                    ),
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("💬", fontSize = 20.sp)
                            Spacer(Modifier.width(8.dp))
                            Text(
                                "Zalo",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                            )
                        }
                        Spacer(Modifier.height(12.dp))
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceEvenly,
                        ) {
                            MetricItem("New", "${zalo.newToday}", Icons.Default.Chat)
                            MetricItem("Auto-reply", "${zalo.autoRepliesSent}", Icons.Default.AutoAwesome)
                            MetricItem("Escalated", "${zalo.escalatedToAgent}", Icons.Default.Person)
                            MetricItem("CSAT", "${zalo.csatScore}", Icons.Default.Star)
                        }
                    }
                }
            }

            // ─── Email Metrics ───────────────────────────────────────

            dashboardData?.emailMetrics?.let { email ->
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = Color(0xFFE53935).copy(alpha = 0.08f),
                    ),
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("📧", fontSize = 20.sp)
                            Spacer(Modifier.width(8.dp))
                            Text(
                                "Email",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                            )
                        }
                        Spacer(Modifier.height(12.dp))
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceEvenly,
                        ) {
                            MetricItem("Total", "${email.totalEmails}", Icons.Default.Email)
                            MetricItem("Unread", "${email.unreadEmails}", Icons.Default.MarkEmailUnread)
                            MetricItem("Urgent", "${email.urgentEmails}", Icons.Default.PriorityHigh)
                            MetricItem("Pending", "${email.pendingFollowUp}", Icons.Default.Schedule)
                        }
                        email.topSender?.let { sender ->
                            Spacer(Modifier.height(8.dp))
                            Text(
                                "Top: ${sender.take(30)}${if (sender.length > 30) "..." else ""}",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }

            // ─── Social Metrics ────────────────────────────────────

            dashboardData?.socialMetrics?.let { social ->
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = Color(0xFF00C853).copy(alpha = 0.08f),
                    ),
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("📱", fontSize = 20.sp)
                            Spacer(Modifier.width(8.dp))
                            Text(
                                "Social Media",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                            )
                        }
                        Spacer(Modifier.height(12.dp))
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceEvenly,
                        ) {
                            MetricItem("Today", "${social.postsToday}", Icons.Default.PostAdd)
                            MetricItem("Scheduled", "${social.scheduledPosts}", Icons.Default.Schedule)
                            MetricItem("Reach", "${social.totalReach}", Icons.Default.Visibility)
                            MetricItem("Engage", "${social.totalEngagement}", Icons.Default.ThumbUp)
                        }
                    }
                }
            }

            // ─── Alerts ────────────────────────────────────────────

            if (dashboardData?.alerts?.any { !it.isRead } == true) {
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = Color(0xFFFF9800).copy(alpha = 0.1f),
                    ),
                ) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text(
                            "⚠️ Alerts",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.Bold,
                        )
                        Spacer(Modifier.height(8.dp))
                        dashboardData?.alerts?.filter { !it.isRead }?.take(3)?.forEach { alert ->
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .padding(vertical = 4.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(
                                    when (alert.severity) {
                                        UnifiedDashboard.AlertSeverity.CRITICAL -> "🔴"
                                        UnifiedDashboard.AlertSeverity.HIGH -> "🟠"
                                        UnifiedDashboard.AlertSeverity.MEDIUM -> "🟡"
                                        UnifiedDashboard.AlertSeverity.LOW -> "🔵"
                                    }
                                )
                                Spacer(Modifier.width(8.dp))
                                Column(modifier = Modifier.weight(1f)) {
                                    Text(alert.title, style = MaterialTheme.typography.bodySmall, fontWeight = FontWeight.Medium)
                                    Text(alert.message, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                }
                            }
                        }
                    }
                }
            }

            // ─── Generate Report ──────────────────────────────────

            OutlinedButton(
                onClick = {
                    scope.launch {
                        val report = dashboard.formatAsDailyReport()
                        android.widget.Toast.makeText(context, "Report generated!", android.widget.Toast.LENGTH_SHORT).show()
                    }
                },
                modifier = Modifier.fillMaxWidth(),
            ) {
                Icon(Icons.Default.Description, null, modifier = Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp))
                Text("📊 Tạo Báo Cáo Ngày")
            }

            Spacer(Modifier.height(16.dp))
        }
    }
}

@Composable
fun MiniStatCard(
    modifier: Modifier = Modifier,
    emoji: String,
    value: String,
    label: String,
    color: Color,
) {
    Card(
        modifier = modifier,
        colors = CardDefaults.cardColors(containerColor = color.copy(alpha = 0.1f)),
    ) {
        Column(
            modifier = Modifier.padding(8.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(emoji, fontSize = 16.sp)
            Text(
                value,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold,
                color = color,
            )
            Text(
                label,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
fun MetricItem(
    label: String,
    value: String,
    icon: androidx.compose.ui.graphics.vector.ImageVector,
) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Icon(icon, label, modifier = Modifier.size(20.dp), tint = MaterialTheme.colorScheme.primary)
        Spacer(Modifier.height(4.dp))
        Text(value, style = MaterialTheme.typography.titleSmall, fontWeight = FontWeight.Bold)
        Text(label, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
fun StatCard(
    modifier: Modifier = Modifier,
    icon: ImageVector,
    label: String,
    value: String,
    subtext: String,
    color: androidx.compose.ui.graphics.Color,
) {
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(16.dp),
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
        ) {
            Icon(
                icon,
                contentDescription = label,
                tint = color,
                modifier = Modifier.size(24.dp),
            )
            Spacer(Modifier.height(8.dp))
            Text(
                value,
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
                color = color,
            )
            Text(
                label,
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                subtext,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.6f),
            )
        }
    }
}

@Composable
fun InfoRow(label: String, value: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
    ) {
        Text(
            label,
            modifier = Modifier.width(80.dp),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(
            value,
            style = MaterialTheme.typography.bodySmall,
            fontWeight = FontWeight.Medium,
        )
    }
}
