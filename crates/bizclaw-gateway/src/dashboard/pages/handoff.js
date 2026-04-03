// HandoffPage — Human Handoff Configuration & Live Queue
// When AI can't handle → auto-escalate to human with full context
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
const { authFetch, t, AppContext } = window;
import { StatsCard } from '/static/dashboard/shared.js';

const HANDOFF_TRIGGERS = [
  { id: 'low_confidence', label: 'AI độ tin cậy thấp (<60%)', icon: '🤔', desc: 'Khi AI không chắc chắn câu trả lời' },
  { id: 'complaint', label: 'Phát hiện khiếu nại', icon: '😤', desc: 'Khách dùng từ: "gặp quản lý", "tệ quá", "refund"...' },
  { id: 'payment_issue', label: 'Vấn đề thanh toán', icon: '💳', desc: 'Khách hỏi hoàn tiền, chuyển khoản sai' },
  { id: 'repeat_question', label: 'Hỏi lại 3+ lần', icon: '🔄', desc: 'Khách hỏi cùng câu nhiều lần → AI chưa giải quyết được' },
  { id: 'explicit_request', label: 'Khách yêu cầu gặp người', icon: '🙋', desc: 'Khách nói: "cho tôi nói chuyện với nhân viên"' },
  { id: 'high_value', label: 'Đơn giá trị cao (>5M)', icon: '💎', desc: 'Đơn hàng trên 5 triệu cần xác nhận thủ công' },
];

function HandoffPage({ lang }) {
  const { showToast } = useContext(AppContext);

  // Settings
  const [settings, setSettings] = useState({
    enabled: true,
    auto_handoff: true,
    notify_channels: ['zalo', 'telegram'],
    triggers: ['low_confidence', 'complaint', 'explicit_request'],
    trigger_configs: {},
    greeting: 'Dạ em xin phép chuyển cuộc trò chuyện cho đồng nghiệp hỗ trợ anh/chị tốt hơn ạ. Vui lòng đợi trong giây lát! 🙏',
    resume_greeting: 'AI Assistant đã quay lại phục vụ anh/chị! Nếu cần gặp nhân viên, cứ nhắn "gặp nhân viên" nhé 😊',
    timeout_minutes: 30,
    working_hours: { start: '08:00', end: '22:00' },
    fallback_message: 'Hiện đang ngoài giờ làm việc. Tin nhắn của anh/chị đã được ghi nhận, chúng em sẽ phản hồi sớm nhất vào sáng mai ạ!',
  });

  // Live queue (demo data)
  const [queue, setQueue] = useState([
    { id: 'hf1', customer: 'Nguyễn Thị Lan', channel: 'Zalo', reason: 'complaint', message: 'Sản phẩm bị lỗi, tôi muốn đổi trả', time: '10:42', status: 'waiting', context_summary: 'KH mua Áo Khoác Gió (AKG-001) ngày 28/03. Phản hồi sản phẩm bị rách đường may.', ai_attempts: 2 },
    { id: 'hf2', customer: 'Trần Văn Minh', channel: 'Messenger', reason: 'high_value', message: 'Tôi muốn đặt 50 bộ đồng phục cho công ty', time: '11:15', status: 'waiting', context_summary: 'KH doanh nghiệp, hỏi đồng phục số lượng lớn. Ước tính đơn 25M VNĐ.', ai_attempts: 1 },
    { id: 'hf3', customer: 'Lê Hoàng Anh', channel: 'Telegram', reason: 'explicit_request', message: 'Cho tôi nói chuyện với quản lý', time: '09:30', status: 'resolved', context_summary: 'KH VIP hỏi về chính sách đại lý. Đã chuyển cho Boss xử lý.', ai_attempts: 3 },
  ]);

  useEffect(() => { loadQueue(); loadSettings(); }, []);

  const loadQueue = async () => {
    try {
      const res = await authFetch('/api/v1/handoff/queue');
      if (res.ok) {
        const data = await res.json();
        if (data.queue /* && data.queue.length > 0 */) {
            setQueue(data.queue);
        }
      }
    } catch(e) {}
  };

  const loadSettings = async () => {
    try {
      const res = await authFetch('/api/v1/handoff/settings');
      if (res.ok) {
        setSettings(await res.json());
      }
    } catch(e) {}
  };

  const saveSettings = async () => {
    try {
      const res = await authFetch('/api/v1/handoff/settings', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(settings)
      });
      if (res.ok) showToast('✅ Đã lưu cấu hình Handoff', 'success');
      else showToast('❌ Không thể lưu', 'error');
    } catch(e) {
      showToast('❌ Lỗi kết nối', 'error');
    }
  };

  const waitingCount = queue.filter(q => q.status === 'waiting').length;
  const resolvedToday = queue.filter(q => q.status === 'resolved').length;

  const toggleTrigger = (id) => {
    setSettings(s => ({
      ...s,
      triggers: s.triggers.includes(id) ? s.triggers.filter(t => t !== id) : [...s.triggers, id]
    }));
  };

  const resolveTicket = async (id) => {
    try {
        await authFetch(`/api/v1/handoff/resolve/${id}`, { method: 'POST' });
    } catch(e) {}
    setQueue(prev => prev.map(q => q.id === id ? { ...q, status: 'resolved' } : q));
    showToast('✅ Đã giải quyết — AI tiếp tục phục vụ khách', 'success');
  };

  const deleteTicket = async (id) => {
    if (!confirm('Bạn có chắc muốn xoá yêu cầu này không?')) return;
    try {
        await authFetch(`/api/v1/handoff/delete/${id}`, { method: 'DELETE' });
    } catch(e) {}
    setQueue(prev => prev.filter(q => q.id !== id));
    showToast('🗑️ Đã xoá yêu cầu hỗ trợ', 'success');
  };

  const jumpToChat = (ticket) => {
    showToast('💬 Đang mở cuộc trò chuyện với ' + ticket.customer + '...', 'info');
    // In production: navigate to chat with this customer's thread
  };

  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <div class="page-header"><div>
      <h1>🤝 Human Handoff (Chuyển Cho Người Thật)</h1>
      <div class="sub">Khi AI gặp giới hạn → tự động chuyển cho Boss/Nhân viên kèm toàn bộ context cuộc chat</div>
    </div>
      <div style="display:flex;align-items:center;gap:10px">
        <span style="font-size:12px;color:var(--text2)">${settings.enabled ? 'Đang bật' : 'Đang tắt'}</span>
        <div style="position:relative;width:44px;height:24px;background:${settings.enabled?'var(--green)':'var(--border)'};border-radius:12px;cursor:pointer;transition:background 0.3s"
          onClick=${() => setSettings(s => ({ ...s, enabled: !s.enabled }))}>
          <div style="position:absolute;top:2px;left:${settings.enabled?'22px':'2px'};width:20px;height:20px;background:#fff;border-radius:50%;transition:left 0.3s;box-shadow:0 1px 3px rgba(0,0,0,0.3)"></div>
        </div>
      </div>
    </div>

    <div class="stats">
      <${StatsCard} label="Đang Chờ Xử lý" value=${waitingCount} color=${waitingCount > 0 ? 'red' : 'green'} icon=${waitingCount > 0 ? '🔴' : '✅'} />
      <${StatsCard} label="Đã Giải Quyết Hôm Nay" value=${resolvedToday} color="green" icon="✅" />
      <${StatsCard} label="Triggers Bật" value=${settings.triggers.length + '/' + HANDOFF_TRIGGERS.length} color="accent" icon="⚡" />
      <${StatsCard} label="Timeout" value=${settings.timeout_minutes + ' phút'} color="blue" icon="⏰" />
    </div>

    <div style="display:grid;grid-template-columns:1fr 1fr;gap:14px">
      <!-- LEFT: Live Queue -->
      <div class="card">
        <h3 style="margin-bottom:12px;display:flex;align-items:center;gap:8px">
          📥 Hàng Đợi Handoff
          ${waitingCount > 0 ? html`<span class="badge badge-red" style="animation:pulse 2s infinite">${waitingCount} chờ</span>` : html`<span class="badge badge-green">Trống</span>`}
        </h3>
        ${queue.length === 0 ? html`
          <div style="text-align:center;padding:40px;color:var(--text2)">
            <div style="font-size:40px;margin-bottom:8px">🎉</div>
            <p>Không có cuộc gọi nào cần hỗ trợ. AI đang xử lý tốt!</p>
          </div>
        ` : html`
          <div style="display:grid;gap:8px">
            ${queue.map(ticket => html`
              <div key=${ticket.id} style="padding:14px;border-radius:10px;border:1px solid ${ticket.status === 'waiting' ? 'var(--red)' : 'var(--border)'};background:${ticket.status === 'waiting' ? 'rgba(239,68,68,0.04)' : 'var(--bg2)'}">
                <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
                  <div style="display:flex;align-items:center;gap:8px">
                    <div style="width:32px;height:32px;border-radius:50%;background:var(--accent);color:#fff;display:flex;align-items:center;justify-content:center;font-size:13px;font-weight:700">
                      ${ticket.customer.charAt(0)}
                    </div>
                    <div>
                      <strong style="font-size:13px">${ticket.customer}</strong>
                      <div style="font-size:11px;color:var(--text2)">${ticket.channel} • ${ticket.time}</div>
                    </div>
                  </div>
                  <span class="badge ${ticket.status === 'waiting' ? 'badge-red' : 'badge-green'}" style="font-size:10px">
                    ${ticket.status === 'waiting' ? '⏳ Đang chờ' : '✅ Đã xử lý'}
                  </span>
                </div>
                <div style="padding:8px 12px;background:var(--bg);border-radius:6px;font-size:12px;margin-bottom:8px;border-left:3px solid var(--accent)">
                  <div style="color:var(--text2);margin-bottom:4px">💬 Tin nhắn gốc:</div>
                  <div style="color:var(--text);font-style:italic">"${ticket.message}"</div>
                </div>
                <div style="font-size:11px;color:var(--text2);margin-bottom:8px">
                  🧠 <strong>AI Context:</strong> ${ticket.context_summary}
                  <span style="margin-left:8px" class="badge" style="font-size:9px">AI thử ${ticket.ai_attempts} lần</span>
                </div>
                <div style="font-size:11px;color:var(--text2);margin-bottom:8px">
                  ⚡ Lý do: <span class="badge badge-outline" style="font-size:9px">${HANDOFF_TRIGGERS.find(t => t.id === ticket.reason)?.label || ticket.reason}</span>
                </div>
                ${ticket.status === 'waiting' ? html`
                  <div style="display:flex;gap:6px;justify-content:flex-end">
                    <button class="btn btn-sm" style="background:var(--accent);color:#fff;padding:5px 14px;font-size:11px" onClick=${() => jumpToChat(ticket)}>💬 Nhảy vào Chat</button>
                    <button class="btn btn-sm btn-outline" style="font-size:11px" onClick=${() => resolveTicket(ticket.id)}>✅ Đã xử lý</button>
                    <button class="btn btn-sm" style="background:transparent;border:1px solid var(--red);color:var(--red);padding:5px 10px;font-size:11px" onClick=${() => deleteTicket(ticket.id)}>🗑️ Xoá</button>
                  </div>
                ` : html`
                  <div style="display:flex;gap:6px;justify-content:flex-end">
                    <button class="btn btn-sm" style="background:transparent;border:1px solid var(--red);color:var(--red);padding:5px 10px;font-size:11px" onClick=${() => deleteTicket(ticket.id)}>🗑️ Xoá</button>
                  </div>
                `}
              </div>
            `)}
          </div>
        `}
      </div>

      <!-- RIGHT: Settings -->
      <div>
        <div class="card" style="margin-bottom:14px">
          <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
            <h3 style="margin:0">⚡ Trigger Conditions (Khi nào chuyển?)</h3>
            <button class="btn btn-sm" style="background:var(--accent);color:#fff" onClick=${saveSettings}>💾 Lưu Cấu Hình</button>
          </div>
          <p style="font-size:12px;color:var(--text2);margin:0 0 12px">Chọn các tình huống AI cần tự động chuyển cho người thật:</p>
          <div style="display:grid;gap:6px">
            ${HANDOFF_TRIGGERS.map(trigger => {
              const isEnabled = settings.triggers.includes(trigger.id);
              return html`
              <div key=${trigger.id}
                style="border-radius:8px;transition:all 0.2s;margin-bottom:6px;overflow:hidden;
                  border:1px solid ${isEnabled ? 'var(--accent)' : 'var(--border)'};
                  background:${isEnabled ? 'var(--accent)' + '0a' : 'var(--bg2)'}">
                
                <div onClick=${() => toggleTrigger(trigger.id)}
                  style="display:flex;align-items:center;gap:10px;padding:10px 14px;cursor:pointer">
                  <span style="font-size:18px">${trigger.icon}</span>
                  <div style="flex:1">
                    <div style="font-size:12px;font-weight:600;color:var(--text)">${trigger.label}</div>
                    <div style="font-size:10px;color:var(--text2)">${trigger.desc}</div>
                  </div>
                  <div style="width:20px;height:20px;border-radius:4px;border:2px solid ${isEnabled ? 'var(--accent)' : 'var(--border)'};background:${isEnabled ? 'var(--accent)' : 'transparent'};display:flex;align-items:center;justify-content:center;color:#fff;font-size:12px;font-weight:700">
                    ${isEnabled ? '✓' : ''}
                  </div>
                </div>

                ${isEnabled ? html`
                  <div style="padding:10px 14px;border-top:1px solid rgba(var(--accent-rgb), 0.2);background:var(--bg)">
                    <div style="font-size:11px;font-weight:600;margin-bottom:6px;color:var(--text2)">Kênh nhận thông báo ca này:</div>
                    <div style="display:flex;gap:6px;flex-wrap:wrap">
                      ${['zalo', 'telegram', 'slack', 'email'].map(ch => {
                        const icons = {zalo:'💙', telegram:'✈️', slack:'💬', email:'📧'};
                        const labels = {zalo:'Zalo OA', telegram:'Telegram', slack:'Slack', email:'Email'};
                        const tConfigs = settings.trigger_configs || {};
                        const actCfg = tConfigs[trigger.id] || {notify_channels:['zalo','telegram'], assignee_group:'general'};
                        const active = (actCfg.notify_channels||[]).includes(ch);
                        return html`<button key=${ch} type="button" class="btn btn-sm ${active?'':'btn-outline'}" 
                          style="font-size:10px;padding:4px 8px;${active?'background:var(--accent);color:#fff;border-color:var(--accent)':''}" 
                          onClick=${(e)=>{
                            e.stopPropagation();
                            const newChannels = active ? (actCfg.notify_channels||[]).filter(c=>c!==ch) : [...(actCfg.notify_channels||[]),ch];
                            setSettings(s => ({...s, trigger_configs: {...(s.trigger_configs||{}), [trigger.id]: { ...actCfg, notify_channels: newChannels}}}));
                        }}>${icons[ch]||'📡'} ${labels[ch]||ch}</button>`;
                      })}
                    </div>
                    
                    <div style="font-size:11px;font-weight:600;margin-top:12px;margin-bottom:6px;color:var(--text2)">Phân nhóm tiếp nhận:</div>
                    <select
                      style="font-size:12px;padding:6px 10px;width:100%;background:var(--bg2);border:1px solid var(--border);color:var(--text);border-radius:6px;cursor:pointer"
                      value=${actCfg.assignee_group || 'general'}
                      onChange=${(e) => {
                        e.stopPropagation();
                        setSettings(s => ({...s, trigger_configs: {...(s.trigger_configs||{}), [trigger.id]: { ...actCfg, assignee_group: e.target.value}}}));
                      }}>
                      <option value="general">👥 Mặc định (Tất cả bộ phận)</option>
                      <option value="sales">💰 Phòng Kinh Doanh (Sales)</option>
                      <option value="support">🛠 Chăm Sóc Khách Hàng (CSKH)</option>
                      <option value="management">🚨 Ban Quản Lý (Khẩn cấp)</option>
                    </select>
                  </div>
                ` : ''}
              </div>
            `})}
          </div>
        </div>

        <div class="card">
          <h3 style="margin-bottom:12px">💬 Tin Nhắn Handoff</h3>
          <div style="display:grid;gap:10px;font-size:13px">
            <label>Tin nhắn khi chuyển cho người
              <textarea style="${inp};min-height:60px;resize:vertical" value=${settings.greeting} onInput=${e => setSettings(s => ({ ...s, greeting: e.target.value }))} /></label>
            <label>Tin nhắn khi AI quay lại
              <textarea style="${inp};min-height:60px;resize:vertical" value=${settings.resume_greeting} onInput=${e => setSettings(s => ({ ...s, resume_greeting: e.target.value }))} /></label>
            <label>Tin nhắn ngoài giờ
              <textarea style="${inp};min-height:60px;resize:vertical" value=${settings.fallback_message} onInput=${e => setSettings(s => ({ ...s, fallback_message: e.target.value }))} /></label>
            <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:10px">
              <label>Timeout (phút)<input type="number" style="${inp}" value=${settings.timeout_minutes} onInput=${e => setSettings(s => ({ ...s, timeout_minutes: +e.target.value || 30 }))} /></label>
              <label>Giờ mở cửa<input type="time" style="${inp}" value=${settings.working_hours.start} onInput=${e => setSettings(s => ({ ...s, working_hours: { ...s.working_hours, start: e.target.value } }))} /></label>
              <label>Giờ đóng cửa<input type="time" style="${inp}" value=${settings.working_hours.end} onInput=${e => setSettings(s => ({ ...s, working_hours: { ...s.working_hours, end: e.target.value } }))} /></label>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>`;
}

export { HandoffPage };
