// ═══ BizClaw Cloud Panel ═══
// Dashboard page for managing Cloud SaaS tenants
// Proxmox VM lifecycle, tenant provisioning, billing overview
const { h, html, useState, useEffect, useCallback } = window;
const { authFetch, t } = window;

export function CloudPage() {
  const [tenants, setTenants] = useState([]);
  const [clusterStats, setClusterStats] = useState(null);
  const [loading, setLoading] = useState(true);
  const [showCreate, setShowCreate] = useState(false);
  const [cloudConfig, setCloudConfig] = useState(null);
  const [configMode, setConfigMode] = useState(false);
  const [actionLoading, setActionLoading] = useState('');

  // Form state
  const [form, setForm] = useState({
    name: '', email: '', phone: '', plan: 'starter', notes: ''
  });

  // Cloud config form
  const [cfgForm, setCfgForm] = useState({
    proxmox_host: '', proxmox_user: 'root@pam', proxmox_password: '',
    default_node: 'pve', template_vmid: '9000', domain_base: 'cloud.bizclaw.vn',
    ollama_endpoint: 'http://10.0.0.1:11434'
  });

  useEffect(() => { loadData(); }, []);

  async function loadData() {
    setLoading(true);
    try {
      const [t, s, c] = await Promise.all([
        authFetch('/api/v1/cloud/tenants').then(r => r.ok ? r.json() : { tenants: [] }),
        authFetch('/api/v1/cloud/stats').then(r => r.ok ? r.json() : null),
        authFetch('/api/v1/cloud/config').then(r => r.ok ? r.json() : null),
      ]);
      setTenants(t.tenants || []);
      setClusterStats(s);
      setCloudConfig(c);
      if (c) {
        setCfgForm({
          proxmox_host: c.proxmox_host || '',
          proxmox_user: c.proxmox_user || 'root@pam',
          proxmox_password: '',
          default_node: c.default_node || 'pve',
          template_vmid: String(c.template_vmid || 9000),
          domain_base: c.domain_base || 'cloud.bizclaw.vn',
          ollama_endpoint: c.ollama_endpoint || ''
        });
      }
    } catch(e) { console.error('Cloud data load error:', e); }
    setLoading(false);
  }

  async function createTenant() {
    if (!form.name || !form.email) return alert('Vui lòng nhập tên và email');
    setActionLoading('creating');
    try {
      const resp = await authFetch('/api/v1/cloud/tenants', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(form)
      });
      if (resp.ok) {
        const data = await resp.json();
        alert(`✅ Tenant created!\nSubdomain: ${data.subdomain}\nPairing Code: ${data.pairing_code}`);
        setShowCreate(false);
        setForm({ name: '', email: '', phone: '', plan: 'starter', notes: '' });
        loadData();
      } else {
        const err = await resp.text();
        alert('❌ Error: ' + err);
      }
    } catch(e) { alert('❌ Network error'); }
    setActionLoading('');
  }

  async function tenantAction(tenantId, action) {
    if (!confirm(`${action} tenant ${tenantId}?`)) return;
    setActionLoading(tenantId);
    try {
      const resp = await authFetch(`/api/v1/cloud/tenants/${tenantId}/${action}`, { method: 'POST' });
      if (resp.ok) { loadData(); }
      else { alert('❌ Action failed: ' + await resp.text()); }
    } catch(e) { alert('❌ Network error'); }
    setActionLoading('');
  }

  async function deleteTenant(tenantId) {
    if (!confirm(`⚠️ XOÁ VĨNH VIỄN tenant ${tenantId}? Không thể hoàn tác!`)) return;
    setActionLoading(tenantId);
    try {
      const resp = await authFetch(`/api/v1/cloud/tenants/${tenantId}`, { method: 'DELETE' });
      if (resp.ok) { loadData(); }
      else { alert('❌ Delete failed'); }
    } catch(e) { alert('❌ Network error'); }
    setActionLoading('');
  }

  async function saveConfig() {
    setActionLoading('config');
    try {
      const resp = await authFetch('/api/v1/cloud/config', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          ...cfgForm,
          template_vmid: parseInt(cfgForm.template_vmid) || 9000
        })
      });
      if (resp.ok) {
        alert('✅ Cloud config saved');
        setConfigMode(false);
        loadData();
      } else { alert('❌ Save failed'); }
    } catch(e) { alert('❌ Network error'); }
    setActionLoading('');
  }

  async function testConnection() {
    setActionLoading('testing');
    try {
      const resp = await authFetch('/api/v1/cloud/test', { method: 'POST' });
      const data = await resp.json().catch(() => ({}));
      if (resp.ok) { alert('✅ Kết nối Proxmox thành công!\n' + (data.message || '')); }
      else { alert('❌ Kết nối thất bại: ' + (data.error || 'Unknown')); }
    } catch(e) { alert('❌ Network error'); }
    setActionLoading('');
  }

  // ═══ Status helpers ═══
  const statusBadge = (status) => {
    const colors = {
      active: { bg: 'rgba(52,211,153,.15)', color: '#34d399', icon: '●' },
      provisioning: { bg: 'rgba(251,191,36,.15)', color: '#fbbf24', icon: '◐' },
      suspended: { bg: 'rgba(251,146,60,.15)', color: '#fb923c', icon: '◯' },
      expired: { bg: 'rgba(239,68,68,.15)', color: '#ef4444', icon: '✕' },
      deleted: { bg: 'rgba(107,114,128,.15)', color: '#6b7280', icon: '✕' },
    };
    const c = colors[status] || colors.expired;
    return html`<span style="background:${c.bg};color:${c.color};padding:4px 12px;border-radius:99px;font-size:12px;font-weight:600;display:inline-flex;align-items:center;gap:4px">${c.icon} ${status}</span>`;
  };

  const planBadge = (plan) => {
    const colors = { starter: '#6366f1', pro: '#06b6d4', business: '#f59e0b' };
    return html`<span style="background:${colors[plan]||'#6366f1'}22;color:${colors[plan]||'#6366f1'};padding:3px 10px;border-radius:6px;font-size:12px;font-weight:700;text-transform:uppercase">${plan}</span>`;
  };

  const formatVND = (n) => new Intl.NumberFormat('vi-VN').format(n) + 'đ';

  if (loading) {
    return html`<div style="display:flex;align-items:center;justify-content:center;height:60vh"><div style="text-align:center"><div style="font-size:36px;margin-bottom:12px">☁️</div><div style="color:var(--text2)">Đang tải Cloud Panel...</div></div></div>`;
  }

  return html`
    <div style="max-width:1200px;margin:0 auto">
      <!-- Header -->
      <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:24px;flex-wrap:wrap;gap:12px">
        <div>
          <h2 style="font-size:24px;font-weight:800;display:flex;align-items:center;gap:10px">☁️ Cloud Panel</h2>
          <p style="color:var(--text2);font-size:14px;margin-top:4px">Quản lý tenant Cloud SaaS — Proxmox VE</p>
        </div>
        <div style="display:flex;gap:8px">
          <button onclick=${() => setConfigMode(!configMode)}
            style="padding:8px 16px;border-radius:8px;border:1px solid var(--border);background:var(--surface);color:var(--text);cursor:pointer;font-size:13px;font-weight:600">
            ⚙️ Cấu hình
          </button>
          <button onclick=${() => setShowCreate(!showCreate)}
            style="padding:8px 20px;border-radius:8px;border:none;background:linear-gradient(135deg,#6366f1,#8b5cf6);color:#fff;cursor:pointer;font-size:13px;font-weight:700;box-shadow:0 4px 12px rgba(99,102,241,.3)">
            + Tạo Tenant
          </button>
        </div>
      </div>

      <!-- Cluster Stats -->
      ${clusterStats ? html`
        <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:12px;margin-bottom:24px">
          <div style="background:var(--surface);border:1px solid var(--border);border-radius:12px;padding:16px">
            <div style="font-size:11px;color:var(--text2);text-transform:uppercase;font-weight:600">Total Tenants</div>
            <div style="font-size:28px;font-weight:800;background:linear-gradient(135deg,#6366f1,#8b5cf6);-webkit-background-clip:text;-webkit-text-fill-color:transparent">${tenants.length}</div>
          </div>
          <div style="background:var(--surface);border:1px solid var(--border);border-radius:12px;padding:16px">
            <div style="font-size:11px;color:var(--text2);text-transform:uppercase;font-weight:600">Active</div>
            <div style="font-size:28px;font-weight:800;color:#34d399">${tenants.filter(t=>t.status==='active').length}</div>
          </div>
          <div style="background:var(--surface);border:1px solid var(--border);border-radius:12px;padding:16px">
            <div style="font-size:11px;color:var(--text2);text-transform:uppercase;font-weight:600">CPU Usage</div>
            <div style="font-size:28px;font-weight:800;color:#06b6d4">${clusterStats.cpu_usage || '—'}%</div>
          </div>
          <div style="background:var(--surface);border:1px solid var(--border);border-radius:12px;padding:16px">
            <div style="font-size:11px;color:var(--text2);text-transform:uppercase;font-weight:600">RAM Usage</div>
            <div style="font-size:28px;font-weight:800;color:#f59e0b">${clusterStats.ram_usage || '—'}%</div>
          </div>
          <div style="background:var(--surface);border:1px solid var(--border);border-radius:12px;padding:16px">
            <div style="font-size:11px;color:var(--text2);text-transform:uppercase;font-weight:600">GPU VRAM</div>
            <div style="font-size:28px;font-weight:800;color:#f472b6">${clusterStats.gpu_used || 0}/${clusterStats.gpu_total || 128}GB</div>
          </div>
          <div style="background:var(--surface);border:1px solid var(--border);border-radius:12px;padding:16px">
            <div style="font-size:11px;color:var(--text2);text-transform:uppercase;font-weight:600">MRR</div>
            <div style="font-size:28px;font-weight:800;color:#34d399">${formatVND(clusterStats.mrr || 0)}</div>
          </div>
        </div>
      ` : html`
        <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:12px;margin-bottom:24px">
          <div style="background:var(--surface);border:1px solid var(--border);border-radius:12px;padding:16px">
            <div style="font-size:11px;color:var(--text2);text-transform:uppercase;font-weight:600">Total Tenants</div>
            <div style="font-size:28px;font-weight:800;background:linear-gradient(135deg,#6366f1,#8b5cf6);-webkit-background-clip:text;-webkit-text-fill-color:transparent">${tenants.length}</div>
          </div>
          <div style="background:var(--surface);border:1px solid var(--border);border-radius:12px;padding:16px">
            <div style="font-size:11px;color:var(--text2);text-transform:uppercase;font-weight:600">Active</div>
            <div style="font-size:28px;font-weight:800;color:#34d399">${tenants.filter(t=>t.status==='active').length}</div>
          </div>
          <div style="background:var(--surface);border:1px solid var(--border);border-radius:12px;padding:16px;grid-column:span 2">
            <div style="font-size:13px;color:var(--text2)">⚙️ Chưa cấu hình Proxmox. Bấm <strong>Cấu hình</strong> để bắt đầu.</div>
          </div>
        </div>
      `}

      <!-- Config Panel (collapsible) -->
      ${configMode ? html`
        <div style="background:var(--surface);border:1px solid var(--border);border-radius:16px;padding:24px;margin-bottom:24px">
          <h3 style="font-size:16px;font-weight:700;margin-bottom:16px;display:flex;align-items:center;gap:8px">⚙️ Cấu hình Cloud Proxmox</h3>
          <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px">
            <div>
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Proxmox Host</label>
              <input value=${cfgForm.proxmox_host} onInput=${e => setCfgForm({...cfgForm, proxmox_host: e.target.value})}
                placeholder="https://proxmox.local:8006"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
            <div>
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Username</label>
              <input value=${cfgForm.proxmox_user} onInput=${e => setCfgForm({...cfgForm, proxmox_user: e.target.value})}
                placeholder="root@pam"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
            <div>
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Password</label>
              <input type="password" value=${cfgForm.proxmox_password} onInput=${e => setCfgForm({...cfgForm, proxmox_password: e.target.value})}
                placeholder="(leave blank to keep current)"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
            <div>
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Default Node</label>
              <input value=${cfgForm.default_node} onInput=${e => setCfgForm({...cfgForm, default_node: e.target.value})}
                placeholder="pve"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
            <div>
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Template VMID (Golden Image)</label>
              <input value=${cfgForm.template_vmid} onInput=${e => setCfgForm({...cfgForm, template_vmid: e.target.value})}
                placeholder="9000"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
            <div>
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Domain Base (wildcard DNS)</label>
              <input value=${cfgForm.domain_base} onInput=${e => setCfgForm({...cfgForm, domain_base: e.target.value})}
                placeholder="cloud.bizclaw.vn"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
            <div style="grid-column:span 2">
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Ollama Endpoint (shared GPU inference)</label>
              <input value=${cfgForm.ollama_endpoint} onInput=${e => setCfgForm({...cfgForm, ollama_endpoint: e.target.value})}
                placeholder="http://10.0.0.1:11434"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
          </div>
          <div style="display:flex;gap:8px;margin-top:16px">
            <button onclick=${saveConfig} disabled=${actionLoading==='config'}
              style="padding:10px 24px;border-radius:8px;border:none;background:linear-gradient(135deg,#6366f1,#8b5cf6);color:#fff;cursor:pointer;font-weight:700;font-size:13px">
              ${actionLoading==='config' ? '⏳ Đang lưu...' : '💾 Lưu cấu hình'}
            </button>
            <button onclick=${testConnection} disabled=${actionLoading==='testing'}
              style="padding:10px 24px;border-radius:8px;border:1px solid var(--border);background:var(--surface);color:var(--text);cursor:pointer;font-weight:600;font-size:13px">
              ${actionLoading==='testing' ? '⏳ Đang test...' : '🔌 Test kết nối'}
            </button>
          </div>
        </div>
      ` : ''}

      <!-- Create Tenant Form -->
      ${showCreate ? html`
        <div style="background:var(--surface);border:1px solid #6366f1;border-radius:16px;padding:24px;margin-bottom:24px;box-shadow:0 0 30px rgba(99,102,241,.1)">
          <h3 style="font-size:16px;font-weight:700;margin-bottom:16px;display:flex;align-items:center;gap:8px">🚀 Tạo Tenant mới</h3>
          <div style="display:grid;grid-template-columns:1fr 1fr;gap:12px">
            <div>
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Tên doanh nghiệp *</label>
              <input value=${form.name} onInput=${e => setForm({...form, name: e.target.value})}
                placeholder="Công ty ABC"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
            <div>
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Email *</label>
              <input value=${form.email} onInput=${e => setForm({...form, email: e.target.value})}
                placeholder="admin@congtyabc.vn"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
            <div>
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Số điện thoại</label>
              <input value=${form.phone} onInput=${e => setForm({...form, phone: e.target.value})}
                placeholder="0909.xxx.xxx"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
            <div>
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Gói dịch vụ</label>
              <select value=${form.plan} onChange=${e => setForm({...form, plan: e.target.value})}
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none">
                <option value="starter">☁️ Starter — 590K/tháng (2 CPU, 4GB RAM, 8GB VRAM)</option>
                <option value="pro">⚡ Pro — 1.490K/tháng (4 CPU, 8GB RAM, 16GB VRAM)</option>
                <option value="business">🏢 Business — 3.990K/tháng (8 CPU, 16GB RAM, 32GB VRAM)</option>
              </select>
            </div>
            <div style="grid-column:span 2">
              <label style="font-size:12px;color:var(--text2);font-weight:600;display:block;margin-bottom:4px">Ghi chú</label>
              <input value=${form.notes} onInput=${e => setForm({...form, notes: e.target.value})}
                placeholder="VD: KH giới thiệu từ Mr.Hoài, cần hỗ trợ setup Zalo OA"
                style="width:100%;padding:10px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px;outline:none" />
            </div>
          </div>
          <div style="display:flex;gap:8px;margin-top:16px">
            <button onclick=${createTenant} disabled=${actionLoading==='creating'}
              style="padding:10px 24px;border-radius:8px;border:none;background:linear-gradient(135deg,#6366f1,#8b5cf6);color:#fff;cursor:pointer;font-weight:700;font-size:13px;box-shadow:0 4px 12px rgba(99,102,241,.3)">
              ${actionLoading==='creating' ? '⏳ Đang tạo VM...' : '🚀 Tạo Tenant (Clone VM)'}
            </button>
            <button onclick=${() => setShowCreate(false)}
              style="padding:10px 24px;border-radius:8px;border:1px solid var(--border);background:var(--surface);color:var(--text);cursor:pointer;font-weight:600;font-size:13px">
              Huỷ
            </button>
          </div>
        </div>
      ` : ''}

      <!-- Tenant Table -->
      <div style="background:var(--surface);border:1px solid var(--border);border-radius:16px;overflow:hidden">
        <div style="padding:16px 20px;border-bottom:1px solid var(--border);display:flex;align-items:center;justify-content:between">
          <h3 style="font-size:15px;font-weight:700;display:flex;align-items:center;gap:8px">📋 Danh sách Tenant (${tenants.length})</h3>
        </div>
        ${tenants.length === 0 ? html`
          <div style="padding:48px;text-align:center">
            <div style="font-size:48px;margin-bottom:12px">☁️</div>
            <div style="color:var(--text2);font-size:15px;margin-bottom:4px">Chưa có tenant nào</div>
            <div style="color:var(--text2);font-size:13px">Bấm <strong>"+ Tạo Tenant"</strong> để provision VPS đầu tiên</div>
          </div>
        ` : html`
          <div style="overflow-x:auto">
            <table style="width:100%;border-collapse:collapse;font-size:13px">
              <thead>
                <tr style="border-bottom:1px solid var(--border)">
                  <th style="padding:10px 16px;text-align:left;font-weight:600;color:var(--text2);font-size:11px;text-transform:uppercase">Tenant</th>
                  <th style="padding:10px 16px;text-align:left;font-weight:600;color:var(--text2);font-size:11px;text-transform:uppercase">Plan</th>
                  <th style="padding:10px 16px;text-align:left;font-weight:600;color:var(--text2);font-size:11px;text-transform:uppercase">Status</th>
                  <th style="padding:10px 16px;text-align:left;font-weight:600;color:var(--text2);font-size:11px;text-transform:uppercase">VM / IP</th>
                  <th style="padding:10px 16px;text-align:left;font-weight:600;color:var(--text2);font-size:11px;text-transform:uppercase">Subdomain</th>
                  <th style="padding:10px 16px;text-align:left;font-weight:600;color:var(--text2);font-size:11px;text-transform:uppercase">Created</th>
                  <th style="padding:10px 16px;text-align:right;font-weight:600;color:var(--text2);font-size:11px;text-transform:uppercase">Actions</th>
                </tr>
              </thead>
              <tbody>
                ${tenants.map(t => html`
                  <tr style="border-bottom:1px solid var(--border);transition:background .2s" onmouseenter=${e=>e.currentTarget.style.background='var(--bg)'} onmouseleave=${e=>e.currentTarget.style.background='transparent'}>
                    <td style="padding:12px 16px">
                      <div style="font-weight:700">${t.name}</div>
                      <div style="font-size:11px;color:var(--text2)">${t.email}</div>
                    </td>
                    <td style="padding:12px 16px">${planBadge(t.plan)}</td>
                    <td style="padding:12px 16px">${statusBadge(t.status)}</td>
                    <td style="padding:12px 16px">
                      <div style="font-family:monospace;font-size:12px">VM${t.vmid} • ${t.ip_address || '—'}</div>
                    </td>
                    <td style="padding:12px 16px">
                      ${t.subdomain ? html`<a href="https://${t.subdomain}" target="_blank" style="font-size:12px;color:var(--accent)">${t.subdomain}</a>` : '—'}
                    </td>
                    <td style="padding:12px 16px;font-size:12px;color:var(--text2)">
                      ${t.created_at ? new Date(t.created_at).toLocaleDateString('vi-VN') : '—'}
                    </td>
                    <td style="padding:12px 16px;text-align:right">
                      <div style="display:flex;gap:4px;justify-content:flex-end">
                        ${t.status === 'active' ? html`
                          <button onclick=${() => tenantAction(t.id, 'suspend')} title="Suspend"
                            style="padding:6px 10px;border-radius:6px;border:1px solid var(--border);background:var(--bg);color:var(--text);cursor:pointer;font-size:11px">⏸️</button>
                        ` : t.status === 'suspended' ? html`
                          <button onclick=${() => tenantAction(t.id, 'resume')} title="Resume"
                            style="padding:6px 10px;border-radius:6px;border:1px solid var(--border);background:var(--bg);color:#34d399;cursor:pointer;font-size:11px">▶️</button>
                        ` : ''}
                        <button onclick=${() => deleteTenant(t.id)} title="Delete"
                          style="padding:6px 10px;border-radius:6px;border:1px solid rgba(239,68,68,.3);background:rgba(239,68,68,.1);color:#ef4444;cursor:pointer;font-size:11px">🗑️</button>
                      </div>
                    </td>
                  </tr>
                `)}
              </tbody>
            </table>
          </div>
        `}
      </div>

      <!-- Revenue Breakdown -->
      ${tenants.length > 0 ? html`
        <div style="background:var(--surface);border:1px solid var(--border);border-radius:16px;padding:24px;margin-top:20px">
          <h3 style="font-size:15px;font-weight:700;margin-bottom:16px;display:flex;align-items:center;gap:8px">💰 Doanh thu tháng</h3>
          <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:12px">
            <div style="background:var(--bg);border-radius:10px;padding:14px">
              <div style="font-size:11px;color:var(--text2);font-weight:600">STARTER (${tenants.filter(t=>t.plan==='starter'&&t.status==='active').length})</div>
              <div style="font-size:20px;font-weight:800;color:#6366f1">${formatVND(tenants.filter(t=>t.plan==='starter'&&t.status==='active').length * 590000)}</div>
            </div>
            <div style="background:var(--bg);border-radius:10px;padding:14px">
              <div style="font-size:11px;color:var(--text2);font-weight:600">PRO (${tenants.filter(t=>t.plan==='pro'&&t.status==='active').length})</div>
              <div style="font-size:20px;font-weight:800;color:#06b6d4">${formatVND(tenants.filter(t=>t.plan==='pro'&&t.status==='active').length * 1490000)}</div>
            </div>
            <div style="background:var(--bg);border-radius:10px;padding:14px">
              <div style="font-size:11px;color:var(--text2);font-weight:600">BUSINESS (${tenants.filter(t=>t.plan==='business'&&t.status==='active').length})</div>
              <div style="font-size:20px;font-weight:800;color:#f59e0b">${formatVND(tenants.filter(t=>t.plan==='business'&&t.status==='active').length * 3990000)}</div>
            </div>
            <div style="background:linear-gradient(135deg,rgba(52,211,153,.1),rgba(99,102,241,.1));border:1px solid rgba(52,211,153,.2);border-radius:10px;padding:14px">
              <div style="font-size:11px;color:var(--text2);font-weight:600">TỔNG MRR</div>
              <div style="font-size:20px;font-weight:800;color:#34d399">${formatVND(
                tenants.filter(t=>t.status==='active').reduce((s,t) => {
                  const prices = { starter: 590000, pro: 1490000, business: 3990000 };
                  return s + (prices[t.plan] || 0);
                }, 0)
              )}</div>
            </div>
          </div>
        </div>
      ` : ''}
    </div>
  `;
}
