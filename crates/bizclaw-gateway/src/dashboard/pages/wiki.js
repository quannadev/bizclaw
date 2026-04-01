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
  }
];

function WikiPage({ lang }) {
  const [activeId, setActiveId] = useState('overview');
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
