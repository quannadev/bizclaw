package vn.bizclaw.app.engine

import android.content.Context
import android.util.Log
import com.google.ai.edge.litertlm.Tool
import com.google.ai.edge.litertlm.ToolParam
import com.google.ai.edge.litertlm.ToolSet

/**
 * Agent Skills sử dụng LiteRT-LM (Gemma 4).
 * Hỗ trợ các Function Calling tích hợp trực tiếp vào trong mô hình.
 */
class BizClawAgentTools(private val context: Context) : ToolSet {
    
    private val TAG = "BizClawAgentTools"

    @Tool(description = "Chạy một lệnh hệ thống của BizClaw Terminal")
    fun runBizClawCommand(
        @ToolParam(description = "Tên lệnh (ví dụ: deploy, restart)") command: String,
        @ToolParam(description = "Tham số của lệnh") parameters: String
    ): Map<String, String> {
        Log.i(TAG, "Agent executing command: \$command, params: \$parameters")
        // Giả lập kết quả thực thi
        return mapOf(
            "status" to "success",
            "message" to "Lệnh \$command đã được cấp phát trên BizClaw"
        )
    }

    @Tool(description = "Lấy dữ liệu thời tiết hiện tại cho cuộc họp")
    fun getWeather(
        @ToolParam(description = "Tên thành phố") location: String
    ): Map<String, String> {
        return mapOf(
            "location" to location,
            "temperature" to "28 độ C",
            "condition" to "Nhiều mây"
        )
    }
}
