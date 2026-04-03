// ═══════════════════════════════════════════════════════════════
// BizClaw — Campaigns Page (Broadcast / Mass Messaging)
// Phase 1: SME Vietnam — Zalo, Telegram, Messenger Broadcasts
// ═══════════════════════════════════════════════════════════════
const { h, html, useState, useEffect, useCallback } = window;
const { authFetch, t } = window;

// ── Campaign Status Badge ──
function StatusBadge({ status }) {
  const map = {
    draft: { cls: 'badge-blue', label: '📝 Nháp' },
    scheduled: { cls: 'badge-orange', label: '⏰ Đã lên lịch' },
    running: { cls: 'badge-purple', label: '🔄 Đang gửi' },
    completed: { cls: 'badge-green', label: '✅ Hoàn tất' },
    paused: { cls: 'badge-yellow', label: '⏸ Tạm dừng' },
    failed: { cls: 'badge-red', label: '❌ Lỗi' },
  };
  const m = map[status] || map.draft;
  return html`<span class="badge ${m.cls}">${m.label}</span>`;
}

// ── Channel selector chips ──
function ChannelChips({ selected, onChange }) {
  const channels = [
    { id: 'zalo', icon: '💬', label: 'Zalo' },
    { id: 'zalo_oa', icon: '🏢', label: 'Zalo OA' },
    { id: 'telegram', icon: '✈️', label: 'Telegram' },
    { id: 'messenger', icon: '💭', label: 'Messenger' },
    { id: 'email', icon: '📧', label: 'Email' },
  ];
  return html`<div style="display:flex;gap:6px;flex-wrap:wrap">
    ${channels.map(ch => html`
      <div onClick=${() => {
        const s = new Set(selected);
        s.has(ch.id) ? s.delete(ch.id) : s.add(ch.id);
        onChange([...s]);
      }} style="cursor:pointer;padding:6px 14px;border-radius:20px;font-size:12px;font-weight:600;
        border:1px solid ${selected.includes(ch.id) ? 'var(--accent)' : 'var(--border)'};
        background:${selected.includes(ch.id) ? 'var(--accent-glow)' : 'var(--surface2)'};
        color:${selected.includes(ch.id) ? 'var(--accent2)' : 'var(--text2)'};
        transition:all .2s;display:flex;align-items:center;gap:4px">
        ${ch.icon} ${ch.label}
      </div>
    `)}
  </div>`;
}

// ── Stats Card Row ──
function CampaignStats({ campaigns }) {
  const total = campaigns.length;
  const sent = campaigns.filter(c => c.status === 'completed').reduce((a, c) => a + (c.sent || 0), 0);
  const delivered = campaigns.filter(c => c.status === 'completed').reduce((a, c) => a + (c.delivered || 0), 0);
  const readRate = delivered > 0 ? Math.round((campaigns.reduce((a, c) => a + (c.read || 0), 0) / delivered) * 100) : 0;

  return html`<div class="stats" style="margin-bottom:20px">
    <div class="card stats-card">
      <div class="stats-label">📊 Tổng chiến dịch</div>
      <div class="stats-value accent">${total}</div>
    </div>
    <div class="card stats-card">
      <div class="stats-label">📤 Đã gửi</div>
      <div class="stats-value green">${sent.toLocaleString()}</div>
    </div>
    <div class="card stats-card">
      <div class="stats-label">✅ Đã nhận</div>
      <div class="stats-value blue">${delivered.toLocaleString()}</div>
    </div>
    <div class="card stats-card">
      <div class="stats-label">👁 Tỷ lệ đọc</div>
      <div class="stats-value orange">${readRate}%</div>
    </div>
  </div>`;
}

// ── Campaign Create/Edit Form ──
function CampaignForm({ onSave, onCancel, editData }) {
  const [name, setName] = useState(editData?.name || '');
  const [channels, setChannels] = useState(editData?.channels || ['zalo']);
  const [message, setMessage] = useState(editData?.message || '');
  const [segment, setSegment] = useState(editData?.segment || 'all');
  const [scheduleAt, setScheduleAt] = useState(editData?.schedule_at || '');
  const [useAi, setUseAi] = useState(false);
  const [aiGenerating, setAiGenerating] = useState(false);

  const generateWithAI = useCallback(async () => {
    if (!name) return;
    setAiGenerating(true);
    try {
      const res = await authFetch('/api/v1/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          message: `Viết một tin nhắn broadcast chuyên nghiệp cho chiến dịch "${name}". 
Yêu cầu: ngắn gọn (dưới 200 ký tự), thân thiện, có emoji, kêu gọi hành động. 
Chỉ trả về nội dung tin nhắn, không giải thích.`,
          agent: 'default'
        })
      });
      const data = await res.json();
      if (data.response) setMessage(data.response.trim());
    } catch(e) { console.error(e); }
    setAiGenerating(false);
  }, [name]);

  return html`<div class="card" style="padding:24px;margin-bottom:20px;animation:fadeIn .3s">
    <h3 style="margin-bottom:16px;font-size:16px">
      ${editData ? '✏️ Sửa chiến dịch' : '✨ Tạo chiến dịch mới'}
    </h3>

    <div style="display:grid;gap:12px">
      <div class="form-row">
        <label class="form-label">Tên chiến dịch</label>
        <input value=${name} onInput=${e => setName(e.target.value)}
          placeholder="VD: Khuyến mãi tháng 3, Chúc mừng sinh nhật..." />
      </div>

      <div class="form-row" style="align-items:start">
        <label class="form-label" style="padding-top:8px">Kênh gửi</label>
        <${ChannelChips} selected=${channels} onChange=${setChannels} />
      </div>

      <div class="form-row">
        <label class="form-label">Đối tượng</label>
        <select value=${segment} onChange=${e => setSegment(e.target.value)}>
          <option value="all">👥 Tất cả khách hàng</option>
          <option value="active">🟢 Khách hoạt động (30 ngày)</option>
          <option value="inactive">🔴 Khách lâu không tương tác</option>
          <option value="vip">⭐ Khách VIP</option>
          <option value="new">🆕 Khách mới</option>
        </select>
      </div>

      <div class="form-row" style="align-items:start">
        <label class="form-label" style="padding-top:8px">
          Nội dung
          <div style="margin-top:6px">
            <button class="btn btn-sm btn-outline" onClick=${generateWithAI}
              disabled=${aiGenerating} style="font-size:10px">
              ${aiGenerating ? '⏳ Đang viết...' : '🤖 AI viết giúp'}
            </button>
          </div>
        </label>
        <textarea value=${message} onInput=${e => setMessage(e.target.value)}
          placeholder="Nhập nội dung tin nhắn broadcast..."
          style="min-height:100px;font-family:var(--font)" />
      </div>

      <div class="form-row">
        <label class="form-label">Lên lịch gửi</label>
        <input type="datetime-local" value=${scheduleAt}
          onInput=${e => setScheduleAt(e.target.value)} />
      </div>
    </div>

    <div style="display:flex;gap:8px;margin-top:18px;justify-content:flex-end">
      <button class="btn btn-outline" onClick=${onCancel}>Huỷ</button>
      <button class="btn btn-primary" onClick=${() => onSave({
        name, channels, message, segment,
        schedule_at: scheduleAt || null,
        status: scheduleAt ? 'scheduled' : 'draft'
      })}>
        ${scheduleAt ? '⏰ Lên lịch' : '💾 Lưu nháp'}
      </button>
      ${!scheduleAt && html`
        <button class="btn btn-green" onClick=${() => onSave({
          name, channels, message, segment, status: 'running'
        })}>
          🚀 Gửi ngay
        </button>
      `}
    </div>
  </div>`;
}

// ── Campaign List Table ──
function CampaignTable({ campaigns, onEdit, onRun, onDelete }) {
  if (!campaigns.length) return html`
    <div class="card" style="padding:40px;text-align:center">
      <div style="font-size:48px;margin-bottom:12px">📢</div>
      <div style="font-size:15px;font-weight:600;margin-bottom:6px">Chưa có chiến dịch nào</div>
      <div style="font-size:13px;color:var(--text2)">Tạo chiến dịch broadcast để gửi tin nhắn hàng loạt</div>
    </div>`;

  return html`<div class="card" style="overflow:hidden">
    <table>
      <thead>
        <tr>
          <th>Chiến dịch</th>
          <th>Kênh</th>
          <th>Đối tượng</th>
          <th>Đã gửi</th>
          <th>Đọc</th>
          <th>Trạng thái</th>
          <th style="text-align:right">Hành động</th>
        </tr>
      </thead>
      <tbody>
        ${campaigns.map((c, i) => html`
          <tr style="animation:slideIn ${0.1 + i * 0.05}s">
            <td>
              <div style="font-weight:600">${c.name}</div>
              <div style="font-size:11px;color:var(--text2)">${c.created_at || 'Vừa tạo'}</div>
            </td>
            <td>${(c.channels || []).map(ch => html`
              <span class="badge badge-blue" style="margin-right:3px">${ch}</span>
            `)}</td>
            <td style="font-size:12px">${
              { all: '👥 Tất cả', active: '🟢 Hoạt động', inactive: '🔴 Không HĐ', vip: '⭐ VIP', new: '🆕 Mới' }[c.segment] || c.segment
            }</td>
            <td class="green">${(c.sent || 0).toLocaleString()}</td>
            <td class="blue">${(c.read || 0).toLocaleString()}</td>
            <td><${StatusBadge} status=${c.status} /></td>
            <td style="text-align:right">
              <div style="display:flex;gap:4px;justify-content:flex-end">
                ${c.status === 'draft' && html`
                  <button class="btn btn-sm btn-green" onClick=${() => onRun(c)}>🚀</button>
                `}
                <button class="btn btn-sm btn-outline" onClick=${() => onEdit(c)}>✏️</button>
                <button class="btn btn-sm btn-red" onClick=${() => onDelete(c)}>🗑</button>
              </div>
            </td>
          </tr>
        `)}
      </tbody>
    </table>
  </div>`;
}

// ═══ MAIN PAGE ═══
export function CampaignsPage() {
  const [campaigns, setCampaigns] = useState([]);
  const [showForm, setShowForm] = useState(false);
  const [editData, setEditData] = useState(null);
  const [filter, setFilter] = useState('all');

  useEffect(() => { loadCampaigns(); }, []);

  const loadCampaigns = async () => {
    try {
      const res = await authFetch('/api/v1/campaigns');
      if (res.ok) {
        const data = await res.json();
        setCampaigns(data.campaigns || data || []);
      }
    } catch(e) {
      // Use demo data if API not ready
      setCampaigns([
        { id: '1', name: 'Khuyến mãi Tháng 3', channels: ['zalo', 'telegram'], segment: 'all', message: '🎉 Giảm 30% tất cả dịch vụ! Liên hệ ngay!', status: 'completed', sent: 1250, delivered: 1180, read: 892, created_at: '2026-03-25' },
        { id: '2', name: 'Chúc mừng sinh nhật', channels: ['zalo'], segment: 'vip', message: '🎂 Chúc mừng sinh nhật! Nhận voucher 500K...', status: 'scheduled', sent: 0, delivered: 0, read: 0, created_at: '2026-03-26' },
        { id: '3', name: 'Nhắc thanh toán', channels: ['zalo', 'email'], segment: 'active', message: 'Nhắc bạn thanh toán hoá đơn...', status: 'draft', sent: 0, delivered: 0, read: 0, created_at: '2026-03-27' },
      ]);
    }
  };

  const saveCampaign = async (data) => {
    try {
      const method = editData ? 'PUT' : 'POST';
      const url = editData ? `/api/v1/campaigns/${editData.id}` : '/api/v1/campaigns';
      await authFetch(url, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data)
      });
    } catch(e) {}
    // Optimistic update
    if (editData) {
      setCampaigns(prev => prev.map(c => c.id === editData.id ? { ...c, ...data } : c));
    } else {
      setCampaigns(prev => [...prev, { id: Date.now().toString(), ...data, sent: 0, delivered: 0, read: 0, created_at: new Date().toISOString().slice(0, 10) }]);
    }
    setShowForm(false);
    setEditData(null);
  };

  const runCampaign = async (c) => {
    if (!confirm(`🚀 Gửi chiến dịch "${c.name}" ngay bây giờ?`)) return;
    try {
      await authFetch(`/api/v1/campaigns/${c.id}/run`, { method: 'POST' });
    } catch(e) {}
    setCampaigns(prev => prev.map(x => x.id === c.id ? { ...x, status: 'running' } : x));
  };

  const deleteCampaign = async (c) => {
    if (!confirm(`🗑 Xoá chiến dịch "${c.name}"?`)) return;
    try {
      await authFetch(`/api/v1/campaigns/${c.id}`, { method: 'DELETE' });
    } catch(e) {}
    setCampaigns(prev => prev.filter(x => x.id !== c.id));
  };

  const filtered = filter === 'all' ? campaigns : campaigns.filter(c => c.status === filter);

  return html`<div>
    <div class="page-header">
      <div>
        <h1>📢 Chiến dịch Broadcast</h1>
        <span class="sub">Gửi tin nhắn hàng loạt qua Zalo, Telegram, Messenger, Email</span>
      </div>
      <button class="btn btn-primary" onClick=${() => { setEditData(null); setShowForm(true); }}>
        ✨ Tạo chiến dịch
      </button>
    </div>

    <${CampaignStats} campaigns=${campaigns} />

    ${showForm && html`
      <${CampaignForm}
        editData=${editData}
        onSave=${saveCampaign}
        onCancel=${() => { setShowForm(false); setEditData(null); }}
      />
    `}

    <div style="display:flex;gap:6px;margin-bottom:16px">
      ${['all', 'draft', 'scheduled', 'running', 'completed'].map(f => html`
        <button class="btn btn-sm ${filter === f ? 'btn-primary' : 'btn-outline'}"
          onClick=${() => setFilter(f)}>
          ${{ all: '📋 Tất cả', draft: '📝 Nháp', scheduled: '⏰ Lên lịch', running: '🔄 Đang gửi', completed: '✅ Xong' }[f]}
        </button>
      `)}
    </div>

    <${CampaignTable}
      campaigns=${filtered}
      onEdit=${c => { setEditData(c); setShowForm(true); }}
      onRun=${runCampaign}
      onDelete=${deleteCampaign}
    />
  </div>`;
}
