// WikiPage — extracted from app.js for modularity
// Uses window globals from index.html (Preact + HTM)
const { h, html, useState } = window;

const WIKI_ARTICLES = [
  {
    id: 'overview',
    icon: '🌟',
    title: 'TỔNG QUAN HỆ SINH THÁI',
    content: `
      <h2>🧠 Khối Não Bộ (AI Engine & Trí Nhớ)</h2>
      <div class="card" style="margin-bottom:16px;">
        <strong>MAMA AI Orchestrator</strong>
        <p style="color:var(--text2);margin-top:4px;">Bộ điều phối đa Agent (Multi-Agent). Tự động phân chia tác vụ: duyệt web, tổng hợp dữ liệu, tạo kịch bản để tránh nghẽn cổ chai và tăng hiệu năng tối đa.</p>
      </div>
      <div class="card" style="margin-bottom:16px;">
        <strong>OpenGnothia Cumulative Memory</strong>
        <p style="color:var(--text2);margin-top:4px;">Hệ thống Ký ức tích lũy dài hạn. AI tự động ghi nhớ thói quen, hành vi và lịch sử chat của TỪNG khách hàng bằng việc lưu lại file SOUL/MEMORY riêng tư.</p>
      </div>
      <div class="card" style="margin-bottom:24px;">
        <strong>Tứ Trụ Omni-modal (Đa Phương Thức)</strong>
        <p style="color:var(--text2);margin-top:4px;">Tích hợp Lõi LLM O1, Claude, Ollama (Local) xử lý Văn Bản (Text), Âm thanh (Voice) và Hình Ảnh (Vision) trong thời gian thực.</p>
      </div>

      <h2>⚡ Khối Cơ Bắp Điện Tử (Automation)</h2>
      <div class="card" style="margin-bottom:16px;">
        <strong>Native Stealth Browser (Rust)</strong>
        <p style="color:var(--text2);margin-top:4px;">Đăng nhập Web tự động, mô phỏng gõ phím/click như người thật. Chọc thủng 100% Anti-Bot hệ thống (Shopee/Cloudflare) qua công nghệ cô lập Profile gốc.</p>
      </div>
      <div class="card" style="margin-bottom:16px;">
        <strong>Chronos & Hands (Lập Lịch Tự Động)</strong>
        <p style="color:var(--text2);margin-top:4px;">Lệnh Cron cho Đôi tay Robot tự động thức dậy lúc rạng sáng quét doanh thu, báo cáo, đăng bài truyền thông.</p>
      </div>
      <div class="card" style="margin-bottom:24px;">
        <strong>Android Interaction Client (Hack Zalo/SĐT)</strong>
        <p style="color:var(--text2);margin-top:4px;">Kết nối thiết bị Android thật (WSS). Agent đọc tin nhắn từ thanh Notification và ra lệnh trả lời tin qua cáp AI. KHÔNG cần Zalo OA đắt tiền.</p>
      </div>
    `
  },
  {
    id: 'manual-agents',
    icon: '🤖',
    title: 'HƯỚNG DẪN: QUẢN LÝ AGENT',
    content: `
      <h2>Module Quản Lý Não Bộ (AI Agents)</h2>
      <p>Agent đóng vai trò là "Nhân viên ảo". Module này cho phép tạo ra nhiều Nhân viên, gán Mô hình trí tuệ cho nó (VD: GPT-4 thông minh hoặc Llama3 tiết kiệm) và nạp Tính cách linh hoạt.</p>
      
      <h3>Cách Setup cơ bản:</h3>
      <ol>
        <li>Vào Menu <strong>Agents</strong>, bấm tạo mới.</li>
        <li>Tại ô <strong>System Prompt</strong>, miêu tả vai trò: <em>"Bạn là lễ tân chuyên nghiệp chốt sale khách sạn, tuyệt đối không trả lời câu hỏi lạc đề."</em></li>
        <li>Chọn <strong>Model</strong> mà nhân viên này sẽ dùng (vd: gpt-4o).</li>
        <li>Vào tab <strong>Channels</strong> để liên kết Agent với Zalo/Telegram.</li>
      </ol>
    `
  },
  {
    id: 'manual-knowledge',
    icon: '📚',
    title: 'HƯỚNG DẪN: KHO TRI THỨC (RAG)',
    content: `
      <h2>Module Tri Thức & Tài Liệu (Knowledge)</h2>
      <p>Thay vì copy paste đoạn dài bắt Bot nhớ, kiến trúc RAG cho phép anh em Bơm thẳng 1000 trang PDF hoặc Báo giá Excel trực tiếp vào Bộ nhớ Vector.</p>
      
      <h3>Cách Setup (Chỉ dùng Text/Tài liệu):</h3>
      <ul>
        <li>Vào tab <strong>Knowledge</strong>, kéo thả file PDF/Word lên hệ thống.</li>
        <li>Quá trình Index mất 2-5 giây để băm tệp thành hàng triệu Vector.</li>
        <li>Bật nút <strong>Auto-RAG</strong> trong cài đặt của Agent. Từ giờ khi bị hóc kiến thức, bot sẽ tự tra cứu File để phản hồi.</li>
      </ul>
      <p style="margin-top:20px;padding:12px;background:var(--bg2);border-radius:8px;border-left:4px solid var(--accent)">
        <strong>LƯU Ý QUAN TRỌNG TỪ TRƯỞNG NHÓM:</strong> Kho Tri Thức này CHỈ DÙNG CHO FILE VĂN BẢN (PDF/DOCX). Nếu anh muốn kết nối Cơ Sở Dữ Liệu SQL (Postgres/MySQL) để lấy Data động, anh vui lòng qua Menu <strong>🗄️ SQL RAG</strong>! Chúng ta cô lập rõ ràng giữa File tĩnh và Data hệ thống.
      </p>
    `
  },
  {
    id: 'manual-sqlrag',
    icon: '🗄️',
    title: 'HƯỚNG DẪN: KHO DỮ LIỆU ĐỘNG (SQL RAG)',
    content: `
      <h2>Module Trí Tuệ Dữ Liệu Thực Tế (SQL RAG)</h2>
      <p>Mang sức mạnh phân tích số liệu lên Chat! Khách hỏi "Nay bán được nhiêu em", Agent sẽ chọc vào MySQL đếm Tiền và trả lời thay vì chỉ là con Chatbot nhại lời.</p>
      
      <h3>Cách triển khai thực tế:</h3>
      <ol>
        <li>Truy cập Menu <strong>DB Assistant (SQL RAG)</strong> bên trái.</li>
        <li>Khai báo Chuỗi kết nối Database. Gợi ý: Hãy dùng user <code>Read-only</code>.</li>
        <li>Bấm <strong>Index Schema</strong> để AI load toàn bộ Sơ đồ Cột, Khóa ngoại.</li>
        <li>Điền vào Business Rules. Vd: <em>"Đơn hủy thì status='CANCELLED', đừng đếm vào Doanh thu"</em>.</li>
        <li>Sau khi Setup, Agent sẽ được kích hoạt Skill Text2SQL.</li>
      </ol>
    `
  },
  {
    id: 'cases',
    icon: '💼',
    title: 'CASE STUDY THỰC CHIẾN',
    content: `
      <h2>Các Kịch Bản Cỗ Máy In Tiền</h2>
      
      <div class="card" style="margin-bottom:16px;border-left:4px solid #3b82f6;">
        <h3 style="margin-top:0">CASE 1: Vận Hành Homestay (Booking vs KiotViet)</h3>
        <p><strong>Quét Booking xuyên đêm:</strong> Bot chụp ảnh hộp thư Khách sạn Extranet mỗi 5 phút bằng <em>Stealth Browser (Snapshot)</em>.</p>
        <p><strong>Sinh Đơn Tự Động:</strong> Agent dịch file HTML thành Text, bốc Tên + IP Khách sang trang KiotViet, tự Find Element, Click Lưu. Giảm phí thuê ca đêm Lễ Tân 100%!</p>
      </div>

      <div class="card" style="margin-bottom:16px;border-left:4px solid #f97316;">
        <h3 style="margin-top:0">CASE 2: Bán Chatbot SaaS qua SePay Thu Tiền Tự Động</h3>
        <p>Boss đóng gói hệ thống BizClaw đi cho thuê với giá 2 triệu/tháng.</p>
        <p>Người thuê chuyển khoản ghi mã <code>BZ123</code>. Cổng Webhook của Bizclaw bắt tín hiệu từ SePay -> Gọi thẳng vào <strong>Multi-Tenant Gateway</strong> -> Nâng cấp tài khoản, kích hoạt Token API cho User. Boss chỉ việc ngủ và đếm tít tít ting ting.</p>
      </div>
    `
  },
  {
    id: 'channel-setup',
    icon: '📡',
    title: 'HƯỚNG DẪN: KẾT NỐI KÊNH CHAT',
    content: `
      <h2>📡 Kết Nối 9 Kênh Messaging — Hướng Dẫn Từng Bước</h2>
      <p>BizClaw hỗ trợ 9 kênh messaging. Mỗi kênh có thể tạo nhiều instance và gán cho Agent riêng biệt.</p>

      <h3 style="margin-top:24px">💜 Facebook Messenger (Phổ biến nhất cho SME Việt)</h3>
      <div class="card" style="margin-bottom:16px;border-left:4px solid #a046ff;">
        <p><strong>Yêu cầu:</strong> Facebook Page của shop + Tài khoản Meta Developer</p>
        <ol>
          <li>Truy cập <strong>developers.facebook.com</strong> → Tạo App mới (loại "Business")</li>
          <li>Trong App Dashboard → <strong>Add Product</strong> → Chọn <strong>Messenger</strong></li>
          <li>Mục <strong>Access Tokens</strong> → Link Facebook Page → Bấm <strong>Generate Token</strong></li>
          <li>Copy <strong>Page Access Token</strong> (dạng EAA...)</li>
          <li>Mục <strong>Webhooks</strong> → Subscribe:
            <ul>
              <li>Callback URL: <code>https://your-bizclaw-domain/api/v1/webhooks/messenger</code></li>
              <li>Verify Token: nhập bất kỳ (VD: <code>bizclaw_verify_2024</code>)</li>
              <li>Tick chọn: <strong>messages, messaging_postbacks</strong></li>
            </ul>
          </li>
          <li>Vào BizClaw Dashboard → <strong>Channels</strong> → Thêm <strong>Facebook Messenger</strong></li>
          <li>Paste: Page ID, Page Access Token, App Secret, Verify Token</li>
          <li>Gán Agent → Bật → <strong>Done! ✅</strong></li>
        </ol>
        <p style="padding:10px;background:var(--bg2);border-radius:6px;font-size:12px;margin-top:12px">
          ⚠️ <strong>Lưu ý:</strong> Ở Development Mode chỉ admin/tester nhắn được. Để mở cho TẤT CẢ khách → vào <strong>App Review</strong> → Submit quyền <code>pages_messaging</code> → Meta duyệt 1-5 ngày. Cần <strong>Business Verification</strong> (upload GPKD).
        </p>
        <p style="padding:10px;background:var(--bg2);border-radius:6px;font-size:12px;margin-top:8px">
          💡 <strong>Quy tắc 24h:</strong> Bot chỉ trả lời trong 24h kể từ tin nhắn cuối của khách. Sau 24h phải dùng Message Tags hoặc One-Time Notification.
        </p>
      </div>

      <h3>🏪 Zalo OA (Official Account)</h3>
      <div class="card" style="margin-bottom:16px;border-left:4px solid #006aff;">
        <p><strong>Yêu cầu:</strong> Tài khoản Zalo OA + Đăng ký developers.zalo.me</p>
        <ol>
          <li>Truy cập <strong>developers.zalo.me</strong> → Tạo ứng dụng mới</li>
          <li>Chọn <strong>Official Account API</strong> → Link OA của shop</li>
          <li>Tab <strong>Cài đặt</strong> → Copy <strong>App ID</strong> và <strong>Secret Key</strong></li>
          <li>Tab <strong>Official Account</strong> → Bấm <strong>Cấp quyền</strong> → Đăng nhập OA → Lấy <strong>Access Token</strong></li>
          <li>Mục <strong>Webhook</strong>:
            <ul>
              <li>URL: <code>https://your-bizclaw-domain/api/v1/webhooks/zalo-oa</code></li>
              <li>Tick chọn: <strong>Gửi và nhận tin nhắn, Quản lý OA</strong></li>
            </ul>
          </li>
          <li>Vào BizClaw Dashboard → <strong>Channels</strong> → Thêm <strong>Zalo OA</strong></li>
          <li>Paste credentials → Gán Agent → Bật</li>
        </ol>
        <p style="padding:10px;background:var(--bg2);border-radius:6px;font-size:12px;margin-top:12px">
          ℹ️ <strong>Access Token hết hạn sau 90 ngày.</strong> Cần lưu Refresh Token để tự động gia hạn. BizClaw hỗ trợ auto-refresh khi cấu hình đầy đủ.
        </p>
      </div>

      <h3>💙 Zalo Cá Nhân</h3>
      <div class="card" style="margin-bottom:16px;border-left:4px solid #006aff;">
        <p><strong>Ưu điểm:</strong> Không cần OA, dùng tài khoản cá nhân. Phù hợp shop nhỏ.</p>
        <ol>
          <li>Vào BizClaw Dashboard → <strong>Channels</strong> → Cấu hình <strong>Zalo Cá Nhân</strong></li>
          <li>Bấm <strong>🔲 Quét QR</strong> → Mở Zalo trên điện thoại → Quét mã</li>
          <li>Hoặc: Vào <code>chat.zalo.me</code> → Copy Cookie → Paste vào ô Cookie</li>
        </ol>
        <p style="padding:10px;background:var(--bg2);border-radius:6px;font-size:12px;margin-top:12px">
          ⚠️ Cần license <strong>zca-cli</strong> để auto-listen tin nhắn. Chạy <code>zca license support-code</code> để lấy mã thiết bị.
        </p>
      </div>

      <h3>📱 Telegram Bot</h3>
      <div class="card" style="margin-bottom:16px;border-left:4px solid #0088cc;">
        <ol>
          <li>Mở Telegram → Nhắn <code>@BotFather</code> → <code>/newbot</code></li>
          <li>Đặt tên bot → Nhận <strong>Bot Token</strong></li>
          <li>Nếu dùng trong Group: <code>/setprivacy</code> → <strong>Disable</strong></li>
          <li>Vào BizClaw → Paste Bot Token → Gán Agent → xong!</li>
        </ol>
      </div>

      <h3>Các kênh khác</h3>
      <table>
        <tr><th>Kênh</th><th>Cần gì</th><th>Thời gian setup</th></tr>
        <tr><td>🎮 Discord</td><td>Bot Token + Message Content Intent</td><td>5 phút</td></tr>
        <tr><td>💬 WhatsApp</td><td>Meta Cloud API + Phone Number ID</td><td>15 phút</td></tr>
        <tr><td>📧 Email</td><td>SMTP/IMAP credentials (Gmail App Password)</td><td>5 phút</td></tr>
        <tr><td>🌐 Webhook</td><td>URL endpoint + Secret</td><td>2 phút</td></tr>
      </table>
    `
  },
  {
    id: 'guide-products-payment',
    icon: '🛍️',
    title: 'HƯỚNG DẪN: BÁN HÀNG + THANH TOÁN',
    content: `
      <h2>🛍️ Bán Hàng Tự Động: Product Catalog → VietQR → Xác Nhận</h2>

      <h3>Bước 1: Nạp Sản Phẩm (Product Catalog)</h3>
      <div class="card" style="margin-bottom:16px;border-left:4px solid var(--accent);">
        <ol>
          <li>Vào Menu <strong>🛍️ Sản Phẩm</strong> → Bấm <strong>+ Thêm Sản Phẩm</strong></li>
          <li>Điền: Tên, Giá, Tồn kho, Phân loại, Mô tả sản phẩm</li>
          <li>Bấm <strong>🔄 Đồng bộ → RAG</strong> để AI "nuốt" bảng giá</li>
          <li>Từ giờ khách hỏi giá qua BẤT KỲ kênh nào → AI trả lời chính xác!</li>
        </ol>
      </div>

      <h3>Bước 2: Cấu Hình Thanh Toán (VietQR)</h3>
      <div class="card" style="margin-bottom:16px;border-left:4px solid #10b981;">
        <ol>
          <li>Vào Menu <strong>💳 Thanh Toán QR</strong></li>
          <li>Chọn ngân hàng (MB, VCB, TCB, ACB...)</li>
          <li>Nhập số tài khoản + tên chủ TK</li>
          <li>(Optional) Nhập SePay API Key để xác nhận tự động</li>
        </ol>
      </div>

      <h3>Bước 3: Flow Hoàn Chỉnh</h3>
      <div class="card" style="margin-bottom:16px;padding:16px;">
        <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;font-size:13px;line-height:2.2">
          <span style="padding:4px 12px;background:var(--bg2);border-radius:6px;">Khách nhắn Zalo: "Áo khoác giá bao nhiêu?"</span>
          <span>→</span>
          <span style="padding:4px 12px;background:var(--bg2);border-radius:6px;">AI tra Product Catalog → "450k, còn 124 cái"</span>
          <span>→</span>
          <span style="padding:4px 12px;background:var(--bg2);border-radius:6px;">Khách: "Lấy 2 cái"</span>
          <span>→</span>
          <span style="padding:4px 12px;background:var(--bg2);border-radius:6px;">AI tạo QR 900k gửi khách</span>
          <span>→</span>
          <span style="padding:4px 12px;background:#10b981;color:#fff;border-radius:6px;">SePay webhook → Xác nhận ✅</span>
        </div>
      </div>
    `
  },
  {
    id: 'guide-handoff',
    icon: '🤝',
    title: 'HƯỚNG DẪN: AI CHUYỂN CHO NGƯỜI',
    content: `
      <h2>🤝 Human Handoff — Khi Nào AI Cần "Gọi Sếp"?</h2>
      <p>Không có AI nào hoàn hảo 100%. Tính năng Handoff cho phép AI tự nhận biết giới hạn và chuyển cuộc chat cho người thật, kèm toàn bộ context.</p>

      <h3>Cấu hình</h3>
      <div class="card" style="margin-bottom:16px;border-left:4px solid var(--accent);">
        <ol>
          <li>Vào Menu <strong>🤝 Human Handoff</strong></li>
          <li>Bật các <strong>Trigger</strong> (khi nào chuyển):
            <ul>
              <li>🤔 AI không chắc chắn (confidence < 60%)</li>
              <li>😤 Phát hiện khiếu nại (từ khóa: "gặp quản lý", "refund"...)</li>
              <li>💳 Vấn đề thanh toán</li>
              <li>🔄 Khách hỏi cùng câu 3+ lần</li>
              <li>🙋 Khách yêu cầu gặp người</li>
              <li>💎 Đơn giá trị cao (tùy chỉnh ngưỡng)</li>
            </ul>
          </li>
          <li>Tuỳ chỉnh <strong>Tin Nhắn Handoff</strong> (AI nói gì khi chuyển)</li>
          <li>Cài <strong>Giờ làm việc</strong> (ngoài giờ → gửi tin nhắn tự động)</li>
        </ol>
      </div>

      <h3>Khi có Handoff</h3>
      <div class="card" style="margin-bottom:16px;border-left:4px solid #ef4444;">
        <p>Hàng đợi sẽ hiện 🔴 số ticket đang chờ. Boss/nhân viên:</p>
        <ol>
          <li>Xem <strong>Context AI</strong>: Tóm tắt cuộc chat + lý do chuyển</li>
          <li>Bấm <strong>💬 Nhảy vào Chat</strong> để trả lời trực tiếp</li>
          <li>Xử lý xong → Bấm <strong>✅ Đã xử lý</strong> → AI tiếp tục phục vụ</li>
        </ol>
    </div>
    `
  },
  {
    id: 'module-connection',
    icon: '🔗',
    title: 'BẢN ĐỒ LIÊN KẾT MODULE',
    content: `<h2>🔗 Bản Đồ Liên Kết — Tất Cả Module Hoạt Động Cùng Nhau</h2>
      <p>BizClaw gồm 8 module chính. Dưới đây là cách setup nhanh nhất cho SME.</p>

      <h3>📋 5 Bước Setup — Từ 0 Đến Bán Hàng Tự Động</h3>
      <div class="card" style="margin-bottom:24px;border-left:4px solid var(--accent);padding:16px;">
        <ol style="font-size:14px;line-height:2.2">
          <li><strong>Tạo Agent</strong> — Menu Agents → Tạo mới → Gán model (GPT-4o/Gemini) → Viết System Prompt.</li>
          <li><strong>Nạp Sản Phẩm</strong> — Menu Products → Thêm sản phẩm → Bấm <strong>🔄 Sync RAG</strong>.</li>
          <li><strong>Kết Nối Kênh</strong> — Menu Channels → Thêm Zalo OA / Messenger / Telegram → Gán Agent.</li>
          <li><strong>Cấu Hình Thanh Toán</strong> — Menu Payment QR → Nhập STK ngân hàng + SePay.</li>
          <li><strong>Bật Handoff</strong> — Menu Human Handoff → Chọn trigger → Setup giờ làm việc → <strong>Done!</strong></li>
        </ol>
      </div>

      <h3>🗺️ Flow Dữ Liệu</h3>
      <div class="card" style="margin-bottom:16px;padding:16px;font-size:13px;">
        <div style="display:flex;flex-direction:column;align-items:center;gap:8px;">
          <div style="padding:8px 20px;background:var(--bg2);border-radius:8px;text-align:center;width:80%;">👤 <strong>Khách hàng</strong> nhắn qua Zalo OA / Messenger / Telegram / WhatsApp</div>
          <div>⬇️</div>
          <div style="padding:8px 20px;background:var(--bg2);border-radius:8px;text-align:center;width:80%;">📡 <strong>Channel Gateway</strong> nhận tin → route đến Agent phụ trách</div>
          <div>⬇️</div>
          <div style="display:flex;gap:12px;width:90%;justify-content:center;">
            <div style="padding:8px 16px;background:rgba(59,130,246,0.1);border:1px solid rgba(59,130,246,0.3);border-radius:8px;flex:1;text-align:center;">🤖 <strong>AI Agent</strong><br>Xử lý + Trả lời</div>
            <div style="padding:8px 16px;background:rgba(239,68,68,0.1);border:1px solid rgba(239,68,68,0.3);border-radius:8px;flex:1;text-align:center;">🤝 <strong>Handoff</strong><br>Chuyển người nếu cần</div>
          </div>
          <div>⬇️ Agent cần tra data</div>
          <div style="display:flex;gap:12px;width:90%;justify-content:center;">
            <div style="padding:8px 16px;background:rgba(16,185,129,0.1);border:1px solid rgba(16,185,129,0.3);border-radius:8px;flex:1;text-align:center;">📚 <strong>RAG</strong><br>Tài liệu + Bảng giá</div>
            <div style="padding:8px 16px;background:rgba(249,115,22,0.1);border:1px solid rgba(249,115,22,0.3);border-radius:8px;flex:1;text-align:center;">🗄️ <strong>SQL RAG</strong><br>DB thực tế</div>
          </div>
          <div>⬇️ Chốt đơn → Tạo QR</div>
          <div style="padding:8px 20px;background:rgba(16,185,129,0.15);border:1px solid rgba(16,185,129,0.3);border-radius:8px;text-align:center;width:80%;">💳 <strong>VietQR</strong> gửi khách → SePay webhook xác nhận ✅</div>
      </div>
    </div>
    <h3>📊 Module nào dùng cái gì?</h3>
    <table>
      <tr><th>Module</th><th>Đọc từ</th><th>Ghi tới</th></tr>
      <tr><td>🤖 Agent</td><td>RAG, SQL RAG, Products</td><td>Channel (reply), Handoff</td></tr>
      <tr><td>📡 Channel</td><td>Webhook/API</td><td>Agent (route tin nhắn)</td></tr>
      <tr><td>🛍️ Products</td><td>—</td><td>RAG (Sync bảng giá)</td></tr>
      <tr><td>📚 Knowledge</td><td>Files, Product Sync</td><td>Agent (kết quả tìm kiếm)</td></tr>
      <tr><td>🗄️ SQL RAG</td><td>MySQL / PostgreSQL</td><td>Agent (kết quả query)</td></tr>
      <tr><td>💳 Payment QR</td><td>Agent yêu cầu</td><td>Channel (gửi hình QR)</td></tr>
      <tr><td>🤝 Handoff</td><td>Agent trigger</td><td>Người thật (notification)</td></tr>
      <tr><td>⏰ Scheduler</td><td>Cron config</td><td>Agent (chạy prompt tự động)</td></tr>
    </table>`
  },
  {
    id: 'quickstart',
    icon: '🚀',
    title: 'BẮT ĐẦU NHANH (5 PHÚT)',
    content: `
      <h2>🚀 Quick Start — Từ 0 Đến Có AI Trả Lời Khách Trong 5 Phút</h2>
      <p style="color:var(--accent2);font-weight:600;font-size:15px">Nếu anh/chị chỉ muốn "cho Bot trả lời tin nhắn", hãy làm đúng 5 bước dưới đây. Không cần biết code!</p>

      <div class="card" style="margin:20px 0;padding:20px;border-left:4px solid #3b82f6;">
        <h3 style="margin-top:0">Bước 1: Chọn Provider AI (1 phút)</h3>
        <ol>
          <li>Vào sidebar <strong>⚙️ Cài đặt</strong></li>
          <li>Mục <strong>Nhà cung cấp AI</strong> → chọn <strong>Gemini</strong> (miễn phí) hoặc OpenAI</li>
          <li>Paste <strong>API Key</strong> → Lưu</li>
        </ol>
        <p style="padding:8px;background:var(--bg2);border-radius:6px;font-size:12px">💡 <strong>Chưa có API Key?</strong> Vào <a href="https://aistudio.google.com/apikey" target="_blank" style="color:var(--accent2)">aistudio.google.com/apikey</a> → Tạo miễn phí!</p>
      </div>

      <div class="card" style="margin:20px 0;padding:20px;border-left:4px solid #10b981;">
        <h3 style="margin-top:0">Bước 2: Tạo Agent AI (1 phút)</h3>
        <ol>
          <li>Vào <strong>🤖 AI Agent</strong> → Bấm <strong>"Tạo Agent"</strong></li>
          <li>Đặt tên: VD <code>sales-bot</code></li>
          <li>Viết System Prompt: <em>"Bạn là nhân viên tư vấn chuyên nghiệp của [Tên shop]. Xưng 'em', gọi 'anh/chị'. Trả lời ngắn gọn, đúng trọng tâm."</em></li>
          <li>Chọn model: <code>gemini-2.5-flash</code></li>
          <li>Bấm <strong>Lưu</strong></li>
        </ol>
      </div>

      <div class="card" style="margin:20px 0;padding:20px;border-left:4px solid #f97316;">
        <h3 style="margin-top:0">Bước 3: Kết nối Kênh Chat (2 phút)</h3>
        <ol>
          <li>Vào <strong>📱 Kênh liên lạc</strong></li>
          <li>Chọn kênh phù hợp:<br/>
            • <strong>Telegram</strong>: Nhanh nhất — chỉ cần Bot Token từ @BotFather<br/>
            • <strong>Zalo OA</strong>: Phổ biến nhất cho SME Việt Nam<br/>
            • <strong>Messenger</strong>: Cho shop bán trên Facebook</li>
          <li>Paste thông tin → Gán Agent đã tạo ở Bước 2</li>
          <li>Bấm <strong>Bật</strong> → Done!</li>
        </ol>
      </div>

      <div class="card" style="margin:20px 0;padding:20px;border-left:4px solid #a855f7;">
        <h3 style="margin-top:0">Bước 4: Nạp Tri Thức (1 phút)</h3>
        <ol>
          <li>Vào <strong>📚 Kho Dữ Liệu RAG</strong></li>
          <li>Bấm <strong>"Thêm tài liệu"</strong></li>
          <li>Upload file PDF/Word bảng giá, FAQ, hoặc paste text trực tiếp</li>
          <li>Từ giờ AI sẽ tra cứu tài liệu để trả lời chính xác!</li>
        </ol>
      </div>

      <div class="card" style="margin:20px 0;padding:20px;border-left:4px solid var(--green);background:rgba(16,185,129,0.05)">
        <h3 style="margin-top:0">✅ Bước 5: Test thử!</h3>
        <ol>
          <li>Vào <strong>💬 Trò chuyện</strong> trong Dashboard → Thử chat với Bot</li>
          <li>Hoặc mở Telegram/Zalo → Nhắn tin cho Bot bạn vừa tạo</li>
          <li>Thấy Bot trả lời? <strong>Chúc mừng — hệ thống đã hoạt động!</strong> 🎉</li>
        </ol>
      </div>

      <div style="padding:16px;background:rgba(59,130,246,0.08);border-radius:8px;border:1px solid rgba(59,130,246,0.2);margin-top:24px">
        <strong>📌 Bước tiếp theo (nâng cấp):</strong>
        <ul style="margin:8px 0 0;font-size:13px">
          <li>Nạp Sản Phẩm → Menu <strong>🛍️ Sản Phẩm</strong></li>
          <li>Thanh toán QR tự động → Menu <strong>💳 Thanh Toán QR</strong></li>
          <li>AI chuyển cho người khi cần → Menu <strong>🤝 Chuyển cho Người</strong></li>
          <li>Broadcast tin nhắn hàng loạt → Menu <strong>📢 Broadcast</strong></li>
          <li>Tự động hóa quy trình → Menu <strong>🔄 Workflows</strong></li>
        </ul>
      </div>
    `
  },
  {
    id: 'dev-guide',
    icon: '👨‍💻',
    title: 'HƯỚNG DẪN DEVELOPER',
    content: `
      <h2>👨‍💻 Developer Setup Guide — Cài Đặt & Triển Khai BizClaw</h2>

      <h3>📦 Yêu cầu hệ thống</h3>
      <div class="card" style="margin-bottom:16px;padding:16px;">
        <table>
          <tr><th>Thành phần</th><th>Yêu cầu tối thiểu</th><th>Đề xuất</th></tr>
          <tr><td>OS</td><td>Linux / macOS / Windows (WSL2)</td><td>Ubuntu 22.04+</td></tr>
          <tr><td>CPU</td><td>2 cores</td><td>4+ cores</td></tr>
          <tr><td>RAM</td><td>2 GB</td><td>8 GB (nếu chạy Ollama local)</td></tr>
          <tr><td>Disk</td><td>500 MB</td><td>10 GB (cho model AI local)</td></tr>
          <tr><td>Rust</td><td>1.80+</td><td>Latest stable</td></tr>
        </table>
      </div>

      <h3>🔧 Cài đặt từ Source</h3>
      <div class="card" style="margin-bottom:16px;padding:16px;border-left:4px solid var(--accent);">
        <pre style="background:var(--bg2);padding:12px;border-radius:6px;font-size:12px;overflow-x:auto;line-height:1.8">
# 1. Clone repo
git clone https://github.com/bizclaw/bizclaw.git
cd bizclaw

# 2. Build release binary
cargo build --release

# 3. Chạy server
./target/release/bizclaw serve --port 3000

# 4. Mở dashboard
open http://localhost:3000</pre>
      </div>

      <h3>🌐 Chạy với Cloudflare Tunnel (truy cập từ xa)</h3>
      <div class="card" style="margin-bottom:16px;padding:16px;border-left:4px solid #f97316;">
        <pre style="background:var(--bg2);padding:12px;border-radius:6px;font-size:12px;overflow-x:auto;line-height:1.8">
# Cài cloudflared (macOS)
brew install cloudflared

# Chạy BizClaw + Tunnel tự động
./run-local.sh

# Output:
#   🏠 Local:  http://localhost:3000
#   🌐 Remote: https://xyz.trycloudflare.com</pre>
        <p style="font-size:12px;color:var(--text2);margin:8px 0 0">💡 URL Remote dùng để paste vào Webhook URL của Zalo OA / Facebook / Telegram.</p>
      </div>

      <h3>⚙️ Cấu hình (config.toml)</h3>
      <div class="card" style="margin-bottom:16px;padding:16px;border-left:4px solid #a855f7;">
        <pre style="background:var(--bg2);padding:12px;border-radius:6px;font-size:12px;overflow-x:auto;line-height:1.8">
[server]
port = 3000
host = "0.0.0.0"

[provider]
name = "gemini"  # openai | ollama | anthropic
api_key = "AIza..."
model = "gemini-2.5-flash"

[security]
jwt_secret = ""  # Để trống = dev mode (no auth)
autonomy = "supervised"

[brain]
workspace = "~/.bizclaw/brain"
auto_memory = true</pre>
      </div>

      <h3>🚀 Deploy Production (VPS)</h3>
      <div class="card" style="margin-bottom:16px;padding:16px;border-left:4px solid #10b981;">
        <pre style="background:var(--bg2);padding:12px;border-radius:6px;font-size:12px;overflow-x:auto;line-height:1.8">
# 1. Build trên VPS (hoặc copy binary từ CI/CD)
cargo build --release

# 2. Tạo systemd service
sudo cat > /etc/systemd/system/bizclaw.service << 'EOF'
[Unit]
Description=BizClaw AI Gateway
After=network.target

[Service]
Type=simple
User=bizclaw
ExecStart=/opt/bizclaw/bizclaw serve --port 3000
WorkingDirectory=/opt/bizclaw
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# 3. Enable & start
sudo systemctl enable bizclaw
sudo systemctl start bizclaw

# 4. Reverse proxy (Caddy — tự động HTTPS)
# Caddyfile:
# bizclaw.yourdomain.com {
#   reverse_proxy localhost:3000
# }</pre>
      </div>

      <h3>📡 API Endpoints chính</h3>
      <div class="card" style="margin-bottom:16px;padding:16px;">
        <table>
          <tr><th>Endpoint</th><th>Method</th><th>Mô tả</th></tr>
          <tr><td><code>/api/v1/info</code></td><td>GET</td><td>Health check, version, status</td></tr>
          <tr><td><code>/api/v1/chat</code></td><td>POST</td><td>Chat với Agent (streaming)</td></tr>
          <tr><td><code>/api/v1/agents</code></td><td>GET/POST</td><td>CRUD Agent</td></tr>
          <tr><td><code>/api/v1/channels/*</code></td><td>*</td><td>Channel management</td></tr>
          <tr><td><code>/api/v1/knowledge/*</code></td><td>*</td><td>RAG documents</td></tr>
          <tr><td><code>/api/v1/brain/files/*</code></td><td>GET/PUT</td><td>Brain workspace CRUD</td></tr>
          <tr><td><code>/api/v1/webhooks/*</code></td><td>POST</td><td>Webhook receivers (Zalo, FB, Telegram)</td></tr>
          <tr><td><code>/api/v1/handoff/*</code></td><td>*</td><td>Human handoff queue</td></tr>
          <tr><td><code>/api/v1/campaigns/*</code></td><td>*</td><td>Broadcast campaigns</td></tr>
          <tr><td><code>/api/v1/scheduler/*</code></td><td>*</td><td>Scheduled tasks</td></tr>
        </table>
      </div>

      <h3>🏗️ Kiến trúc Codebase</h3>
      <div class="card" style="padding:16px;font-size:13px;">
        <pre style="background:var(--bg2);padding:12px;border-radius:6px;font-size:11px;line-height:1.6;overflow-x:auto">
bizclaw/
├── crates/
│   ├── bizclaw-core/          # LLM providers, tools, memory
│   ├── bizclaw-gateway/       # Axum HTTP server + Dashboard
│   │   ├── src/
│   │   │   ├── server.rs      # Route definitions
│   │   │   ├── dashboard.rs   # Static file embedding
│   │   │   ├── dashboard/     # Preact SPA (embedded)
│   │   │   │   ├── app.js     # Main app shell
│   │   │   │   ├── shared.js  # Sidebar + auth + i18n
│   │   │   │   └── pages/     # Lazy-loaded page modules
│   │   │   └── routes/        # API handlers
│   │   └── Cargo.toml
│   └── bizclaw-orchestrator/  # Multi-agent + workflows
├── data/                      # Runtime configs (JSON)
├── config.toml                # Main config
└── run-local.sh               # Dev launcher + tunnel</pre>
      </div>
    `
  },
  {
    id: 'faq',
    icon: '❓',
    title: 'CÂU HỎI THƯỜNG GẶP (FAQ)',
    content: `
      <h2>❓ FAQ — Giải Đáp Nhanh</h2>

      <div class="card" style="margin-bottom:12px;padding:16px;border-left:3px solid var(--accent)">
        <h4 style="margin:0 0 4px">🆓 BizClaw có miễn phí không?</h4>
        <p style="margin:0;color:var(--text2);font-size:13px">Có! Bản <strong>Single (Free)</strong> chạy trên máy cá nhân, không giới hạn Agent. Bản <strong>Cloud (Trả phí)</strong> có GPU + VPS + support team.</p>
      </div>

      <div class="card" style="margin-bottom:12px;padding:16px;border-left:3px solid var(--accent)">
        <h4 style="margin:0 0 4px">🤖 Cần biết lập trình không?</h4>
        <p style="margin:0;color:var(--text2);font-size:13px">Không! Dashboard thiết kế cho người không biết code. Chỉ cần paste API key + viết System Prompt bằng tiếng Việt.</p>
      </div>

      <div class="card" style="margin-bottom:12px;padding:16px;border-left:3px solid var(--accent)">
        <h4 style="margin:0 0 4px">💸 AI tốn bao nhiêu tiền?</h4>
        <p style="margin:0;color:var(--text2);font-size:13px">Dùng <strong>Gemini Flash</strong> miễn phí 1500 request/ngày. Dùng <strong>Ollama</strong> trên máy mình = hoàn toàn miễn phí, không giới hạn. GPT-4o khoảng $0.01/câu.</p>
      </div>

      <div class="card" style="margin-bottom:12px;padding:16px;border-left:3px solid var(--accent)">
        <h4 style="margin:0 0 4px">📱 Zalo cá nhân vs Zalo OA?</h4>
        <p style="margin:0;color:var(--text2);font-size:13px"><strong>Zalo cá nhân:</strong> Dùng SĐT cá nhân, setup nhanh, phù hợp shop nhỏ. <strong>Zalo OA:</strong> Chuyên nghiệp hơn, có template message, broadcast, phù hợp doanh nghiệp.</p>
      </div>

      <div class="card" style="margin-bottom:12px;padding:16px;border-left:3px solid var(--accent)">
        <h4 style="margin:0 0 4px">🔒 Dữ liệu có an toàn không?</h4>
        <p style="margin:0;color:var(--text2);font-size:13px">100%! BizClaw Single chạy trên máy bạn — dữ liệu KHÔNG ra khỏi máy (trừ API call tới OpenAI/Gemini). Dùng Ollama local = hoàn toàn offline.</p>
      </div>

      <div class="card" style="margin-bottom:12px;padding:16px;border-left:3px solid var(--accent)">
        <h4 style="margin:0 0 4px">🤝 AI gặp giới hạn thì sao?</h4>
        <p style="margin:0;color:var(--text2);font-size:13px">Tính năng <strong>Human Handoff</strong> cho phép AI tự nhận biết và chuyển cuộc chat cho người thật kèm toàn bộ context. Boss nhận thông báo qua Zalo/Telegram.</p>
      </div>

      <div class="card" style="margin-bottom:12px;padding:16px;border-left:3px solid var(--accent)">
        <h4 style="margin:0 0 4px">🔄 Muốn thêm tính năng thì sao?</h4>
        <p style="margin:0;color:var(--text2);font-size:13px">BizClaw hỗ trợ <strong>MCP (Model Context Protocol)</strong> và <strong>Plugin Market</strong>. Dev có thể viết tool/plugin riêng hoặc cài từ marketplace.</p>
      </div>

      <div class="card" style="margin-bottom:12px;padding:16px;border-left:3px solid var(--accent)">
        <h4 style="margin:0 0 4px">📧 Liên hệ hỗ trợ?</h4>
        <p style="margin:0;color:var(--text2);font-size:13px">Telegram: <code>@bizclaw_support</code> • Email: <code>support@bizclaw.io</code> • Cộng đồng: <code>t.me/bizclaw_community</code></p>
      </div>
    `
  }
];

function WikiPage({ lang }) {
  const isFirstVisit = !localStorage.getItem('bizclaw_wiki_visited');
  const [activeId, setActiveId] = useState(isFirstVisit ? 'quickstart' : 'overview');
  const [searchQ, setSearchQ] = useState('');
  const [showSearch, setShowSearch] = useState(false);

  const article = WIKI_ARTICLES.find(a => a.id === activeId) || WIKI_ARTICLES[0];
  const results = searchQ ? WIKI_ARTICLES.filter(a =>
    a.title.toLowerCase().includes(searchQ.toLowerCase()) ||
    a.content.toLowerCase().includes(searchQ.toLowerCase())
  ) : null;

  return html`
    <div class="page-header" style="margin-bottom:24px;">
      <div>
        <h1 style="font-size:28px;margin:0;display:flex;align-items:center;gap:12px">
          💡 Học Viện BizClaw (Wiki)
        </h1>
        <div class="sub" style="color:var(--text2);margin-top:4px;">Cẩm nang Hướng dẫn, Cấu hình Module và Thực chiến</div>
      </div>
      <button class="btn btn-outline btn-sm" onClick=${() => setShowSearch(!showSearch)} style="border:1px solid var(--border);background:var(--bg);color:var(--text);border-radius:8px;padding:6px 12px;cursor:pointer;">
        🔍 Tìm kiếm
      </button>
    </div>

    ${showSearch && html`
      <div style="margin-bottom:20px;animation:fadeIn 0.2s">
        <input type="text" placeholder="Tìm kiếm hướng dẫn..." value=${searchQ} onInput=${e => setSearchQ(e.target.value)} 
               style="width:100%;padding:12px 16px;background:var(--bg2);border:1px solid var(--accent);border-radius:8px;color:var(--text);font-size:14px;box-shadow:0 0 0 2px rgba(59,130,246,0.1);" />
      </div>
    `}

    <div style="display:grid;grid-template-columns:250px 1fr;gap:24px;align-items:start;">
      <!-- Sidebar Menu -->
      <div class="card" style="padding:16px;">
        <div style="font-size:11px;text-transform:uppercase;letter-spacing:1px;font-weight:700;color:var(--text2);margin-bottom:12px">Bảng Điều Hướng</div>
        <div style="display:flex;flex-direction:column;gap:4px;">
          ${WIKI_ARTICLES.map(a => html`
            <a href="#" onClick=${e => { e.preventDefault(); setActiveId(a.id); setSearchQ(''); }}
               style="display:flex;align-items:center;gap:10px;padding:8px 12px;border-radius:8px;text-decoration:none;font-size:13px;transition:all 0.2s;
                      color:${activeId===a.id ? 'var(--accent)' : 'var(--text)'};
                      background:${activeId===a.id ? 'var(--bg2)' : 'transparent'};
                      font-weight:${activeId===a.id ? '600' : '400'};
                      border:1px solid ${activeId===a.id ? 'var(--border)' : 'transparent'};">
              <span style="font-size:18px">${a.icon}</span> ${a.title}
            </a>
          `)}
        </div>
      </div>

      <!-- Main Content -->
      <div class="card" style="min-height:500px;padding:32px;line-height:1.7;">
        ${results ? html`
          <h2 style="margin-top:0">🔍 Có ${results.length} kết quả cho "${searchQ}"</h2>
          ${results.length ? results.map(a => html`
            <div class="card" style="margin-bottom:12px;cursor:pointer;border-left:3px solid var(--accent)" onClick=${() => {setActiveId(a.id); setSearchQ('');}}>
              <h3 style="margin:0;font-size:15px;">${a.icon} ${a.title}</h3>
            </div>
          `) : html`<div style="text-align:center;padding:40px;color:var(--text2)">Không tìm thấy tài liệu nào khớp.</div>`}
        ` : html`
          <div dangerouslySetInnerHTML=${{ __html: article.content }} />
        `}
      </div>
    </div>
  `;
}

export { WikiPage };
