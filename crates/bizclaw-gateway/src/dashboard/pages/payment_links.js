// PaymentLinksPage — In-Chat Payment Link Generator (VietQR)
// AI closes sale → generates VietQR → sends to customer in chat → auto-confirm via SePay webhook
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

const BANKS = [
  { code: 'VCB', name: 'Vietcombank', color: '#006838' },
  { code: 'TCB', name: 'Techcombank', color: '#e31837' },
  { code: 'MB', name: 'MB Bank', color: '#1a4d8f' },
  { code: 'ACB', name: 'ACB', color: '#1a3c6d' },
  { code: 'VPB', name: 'VPBank', color: '#00693e' },
  { code: 'TPB', name: 'TPBank', color: '#6f2da8' },
  { code: 'BIDV', name: 'BIDV', color: '#0254a5' },
  { code: 'VTB', name: 'Vietinbank', color: '#1a3c78' },
];

function PaymentLinksPage({ lang }) {
  const { showToast } = useContext(AppContext);

  // Config
  const [config, setConfig] = useState({
    bank_code: 'MB',
    account_number: '',
    account_name: '',
    prefix: 'BC',
    sepay_key: '',
    auto_confirm: true,
  });

  // Generator
  const [form, setForm] = useState({ amount: 0, description: '', customer: '', order_id: '' });
  const [generatedLinks, setGeneratedLinks] = useState([]);

  // Transaction history (real data will be fetched from API later)
  const [transactions, setTransactions] = useState([]);

  const todayRevenue = transactions.filter(t => t.status === 'paid').reduce((s, t) => s + t.amount, 0);
  const paidCount = transactions.filter(t => t.status === 'paid').length;
  const pendingCount = transactions.filter(t => t.status === 'pending').length;

  const generateQR = () => {
    if (!form.amount || !config.account_number) {
      showToast('⚠️ Nhập số tài khoản và số tiền', 'error'); return;
    }
    const orderId = config.prefix + new Date().toISOString().slice(0, 10).replace(/-/g, '') + String(generatedLinks.length + transactions.length + 1).padStart(3, '0');
    const transferContent = orderId + (form.customer ? ' ' + form.customer : '');
    const bank = BANKS.find(b => b.code === config.bank_code);
    const qrUrl = `https://img.vietqr.io/image/${config.bank_code}-${config.account_number}-compact2.png?amount=${form.amount}&addInfo=${encodeURIComponent(transferContent)}&accountName=${encodeURIComponent(config.account_name)}`;

    const link = {
      id: 'ql' + Date.now(),
      order_id: orderId,
      amount: form.amount,
      customer: form.customer,
      description: form.description,
      qr_url: qrUrl,
      transfer_content: transferContent,
      bank: bank,
      created: new Date().toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' }),
      status: 'pending',
    };
    setGeneratedLinks(prev => [link, ...prev]);
    showToast('✅ Đã tạo mã QR: ' + orderId + ' (' + fmtPrice(form.amount) + ')', 'success');
    setForm(f => ({ ...f, amount: 0, description: '', customer: '', order_id: '' }));
  };

  const copyLink = (link) => {
    const text = `💳 Thanh toán đơn hàng ${link.order_id}\n💰 Số tiền: ${fmtPrice(link.amount)}\n🏦 ${link.bank?.name}: ${config.account_number}\n👤 Chủ TK: ${config.account_name}\n📝 Nội dung CK: ${link.transfer_content}\n\n📱 Quét mã QR: ${link.qr_url}`;
    navigator.clipboard.writeText(text).then(() => showToast('📋 Đã copy tin nhắn thanh toán!', 'success'));
  };

  const fmtPrice = (v) => v.toLocaleString('vi-VN') + ' ₫';
  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <div class="page-header"><div>
      <h1>💳 Thanh Toán Trong Chat (VietQR)</h1>
      <div class="sub">AI chốt sale → Tạo mã QR → Gửi cho khách → SePay webhook xác nhận tự động</div>
    </div></div>

    <div class="stats">
      <${StatsCard} label="Doanh Thu Hôm Nay" value=${fmtPrice(todayRevenue)} color="green" icon="💰" />
      <${StatsCard} label="Đã Thanh Toán" value=${paidCount} color="green" icon="✅" />
      <${StatsCard} label="Chờ Thanh Toán" value=${pendingCount} color=${pendingCount > 0 ? 'yellow' : 'green'} icon="⏳" />
      <${StatsCard} label="QR Đã Tạo" value=${generatedLinks.length} color="accent" icon="📱" />
    </div>

    <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
      <!-- LEFT: Config + Generator -->
      <div>
        <!-- Bank Config -->
        <div class="card" style="margin-bottom:14px">
          <h3 style="margin-bottom:12px">🏦 Cấu Hình Tài Khoản Nhận Tiền</h3>
          <div style="display:grid;gap:10px;font-size:13px">
            <label>Ngân hàng
              <div style="display:grid;grid-template-columns:repeat(4,1fr);gap:6px;margin-top:6px">
                ${BANKS.map(b => html`
                  <button key=${b.code} class="btn btn-sm" onClick=${() => setConfig(c => ({ ...c, bank_code: b.code }))}
                    style="padding:8px 4px;border-radius:8px;font-size:11px;font-weight:600;text-align:center;
                      border:2px solid ${config.bank_code === b.code ? b.color : 'var(--border)'};
                      background:${config.bank_code === b.code ? b.color + '15' : 'var(--bg2)'};
                      color:${config.bank_code === b.code ? b.color : 'var(--text2)'}">
                    ${b.name}
                  </button>
                `)}
              </div>
            </label>
            <label>Số tài khoản *<input style="${inp}" value=${config.account_number} onInput=${e => setConfig(c => ({ ...c, account_number: e.target.value }))} placeholder="0123456789" /></label>
            <label>Tên chủ tài khoản<input style="${inp}" value=${config.account_name} onInput=${e => setConfig(c => ({ ...c, account_name: e.target.value }))} placeholder="NGUYEN VAN A" /></label>
            <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px">
              <label>Tiền tố mã đơn<input style="${inp}" value=${config.prefix} onInput=${e => setConfig(c => ({ ...c, prefix: e.target.value }))} placeholder="BC" /></label>
              <label>SePay API Key
                <input type="password" style="${inp}" value=${config.sepay_key} onInput=${e => setConfig(c => ({ ...c, sepay_key: e.target.value }))} placeholder="sk_live_..." />
              </label>
            </div>
          </div>
        </div>

        <!-- QR Generator -->
        <div class="card" style="border:1px solid var(--accent)">
          <h3 style="margin-bottom:12px">📱 Tạo Mã QR Thanh Toán</h3>
          <div style="display:grid;gap:10px;font-size:13px">
            <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px">
              <label>Số tiền (VNĐ) *<input type="number" style="${inp}" value=${form.amount} onInput=${e => setForm(f => ({ ...f, amount: +e.target.value || 0 }))} placeholder="450000" /></label>
              <label>Tên khách hàng<input style="${inp}" value=${form.customer} onInput=${e => setForm(f => ({ ...f, customer: e.target.value }))} placeholder="Nguyễn Văn A" /></label>
            </div>
            <label>Ghi chú đơn hàng<input style="${inp}" value=${form.description} onInput=${e => setForm(f => ({ ...f, description: e.target.value }))} placeholder="Áo Khoác Gió x2" /></label>
          </div>
          <div style="margin-top:14px;display:flex;gap:8px;justify-content:flex-end">
            <button class="btn" style="background:var(--grad1);color:#fff;padding:10px 24px;font-size:14px;display:flex;align-items:center;gap:6px" onClick=${generateQR}>
              📱 Tạo Mã QR
            </button>
          </div>
        </div>
      </div>

      <!-- RIGHT: Generated Links + Transactions -->
      <div>
        ${generatedLinks.length > 0 ? html`
          <div class="card" style="margin-bottom:14px">
            <h3 style="margin-bottom:12px">📱 Mã QR Vừa Tạo (${generatedLinks.length})</h3>
            <div style="display:grid;gap:10px">
              ${generatedLinks.map(link => html`
                <div key=${link.id} style="padding:14px;border-radius:10px;border:1px solid var(--accent);background:var(--accent)08">
                  <div style="display:flex;gap:14px;align-items:flex-start">
                    <!-- QR Preview -->
                    <div style="flex-shrink:0;width:120px;height:120px;border-radius:8px;overflow:hidden;background:#fff;display:flex;align-items:center;justify-content:center;border:1px solid var(--border)">
                      <img src=${link.qr_url} style="width:100%;height:100%;object-fit:contain" onerror=${e => { e.target.style.display = 'none'; e.target.parentElement.innerHTML = '<div style="color:var(--text2);font-size:11px;text-align:center;padding:8px">Nhập STK<br>để tạo QR</div>'; }} />
                    </div>
                    <div style="flex:1;min-width:0">
                      <div style="display:flex;align-items:center;gap:8px;margin-bottom:6px">
                        <strong style="font-size:14px;color:var(--accent)">${fmtPrice(link.amount)}</strong>
                        <span class="badge badge-outline" style="font-size:10px">${link.order_id}</span>
                      </div>
                      <div style="font-size:12px;color:var(--text2);margin-bottom:4px">🏦 ${link.bank?.name} • ${config.account_number}</div>
                      <div style="font-size:12px;color:var(--text2);margin-bottom:4px">👤 ${link.customer || 'Khách vãng lai'}</div>
                      <div style="font-size:11px;color:var(--text2);margin-bottom:8px">📝 CK: <code style="background:var(--bg2);padding:2px 6px;border-radius:4px">${link.transfer_content}</code></div>
                      <div style="display:flex;gap:6px">
                        <button class="btn btn-sm" style="background:var(--accent);color:#fff;padding:4px 12px;font-size:11px" onClick=${() => copyLink(link)}>📋 Copy Tin Nhắn</button>
                      </div>
                    </div>
                  </div>
                </div>
              `)}
            </div>
          </div>
        ` : ''}

        <!-- Transaction History -->
        <div class="card">
          <h3 style="margin-bottom:12px">📋 Lịch Sử Giao Dịch Hôm Nay</h3>
          ${transactions.length === 0 ? html`
            <div style="text-align:center;padding:30px;color:var(--text2)">Chưa có giao dịch hôm nay</div>
          ` : html`
            <div style="display:grid;gap:6px">
              ${transactions.map(tx => html`
                <div key=${tx.id} style="display:flex;align-items:center;gap:10px;padding:10px 14px;border-radius:8px;background:var(--bg2);border-left:3px solid ${tx.status === 'paid' ? 'var(--green)' : '#f59e0b'}">
                  <div style="font-size:20px">${tx.status === 'paid' ? '✅' : '⏳'}</div>
                  <div style="flex:1;min-width:0">
                    <div style="display:flex;align-items:center;gap:6px">
                      <strong style="font-size:13px">${tx.customer}</strong>
                      <span class="badge" style="font-size:9px">${tx.channel}</span>
                    </div>
                    <div style="font-size:11px;color:var(--text2)">${tx.order_id} • ${tx.time}</div>
                  </div>
                  <div style="text-align:right">
                    <div style="font-size:14px;font-weight:700;color:${tx.status === 'paid' ? 'var(--green)' : 'var(--text)'}">${fmtPrice(tx.amount)}</div>
                    <span class="badge ${tx.status === 'paid' ? 'badge-green' : 'badge-outline'}" style="font-size:9px">${tx.status === 'paid' ? 'Đã nhận' : 'Chờ CK'}</span>
                  </div>
                </div>
              `)}
            </div>
          `}
        </div>
      </div>
    </div>

    <!-- How it works -->
    <div style="margin-top:14px;padding:16px 20px;background:var(--bg2);border-radius:12px;border:1px solid var(--border)">
      <h4 style="margin:0 0 10px;font-size:13px;color:var(--text)">🔄 Luồng hoạt động: AI → QR → Thanh toán → Xác nhận</h4>
      <div style="display:flex;gap:8px;flex-wrap:wrap;font-size:12px;color:var(--text2)">
        <span style="padding:6px 12px;background:var(--bg);border-radius:8px;border:1px solid var(--border)">1️⃣ Khách chốt đơn qua Chat</span>
        <span style="color:var(--accent)">→</span>
        <span style="padding:6px 12px;background:var(--bg);border-radius:8px;border:1px solid var(--border)">2️⃣ AI gọi Tool tạo mã QR</span>
        <span style="color:var(--accent)">→</span>
        <span style="padding:6px 12px;background:var(--bg);border-radius:8px;border:1px solid var(--border)">3️⃣ Gửi QR cho khách trong Chat</span>
        <span style="color:var(--accent)">→</span>
        <span style="padding:6px 12px;background:var(--bg);border-radius:8px;border:1px solid var(--border)">4️⃣ Khách quét QR chuyển khoản</span>
        <span style="color:var(--accent)">→</span>
        <span style="padding:6px 12px;background:var(--green);color:#fff;border-radius:8px">5️⃣ SePay Webhook → Xác nhận tự động ✅</span>
      </div>
    </div>
  </div>`;
}

export { PaymentLinksPage };
