// WorkflowsPage — extracted from app.js for modularity
// Uses window globals from index.html (Preact + HTM)
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

// ═══ 10 PRE-BUILT SME WORKFLOW TEMPLATES ═══
// Inspired by Easy AI's standardized workflow catalog, adapted for BizClaw's action-centric model
const SME_TEMPLATES = [
  { id: 'tpl_sales_bot', name: '🛒 Bot Chốt Sale (Zalo/Messenger)', description: 'Tự động trả lời tin nhắn bán hàng, tư vấn giá, chốt đơn qua kênh chat. Sử dụng RAG bảng giá + Stealth Browser.', tags: ['sales','zalo','auto-reply'], category: 'sales',
    steps: [{ name: 'Đọc tin khách', type: 'Sequential', agent_role: 'Listener', prompt: 'Đọc tin nhắn mới nhất từ khách qua kênh Zalo/Messenger' }, { name: 'Tra RAG bảng giá', type: 'Sequential', agent_role: 'Knowledge', prompt: 'Tìm sản phẩm liên quan trong Knowledge Base và trả về giá + tồn kho' }, { name: 'Soạn & Gửi trả lời', type: 'Sequential', agent_role: 'Writer', prompt: 'Soạn tin nhắn bán hàng thân thiện, có giá, khuyến mãi, và CTA chốt đơn' }] },
  { id: 'tpl_faq_responder', name: '❓ FAQ Auto-Responder', description: 'Tự động trả lời câu hỏi thường gặp dựa trên tài liệu FAQ/SOP đã nạp vào RAG.', tags: ['faq','support','rag'], category: 'support',
    steps: [{ name: 'Phân loại câu hỏi', type: 'Sequential', agent_role: 'Classifier', prompt: 'Phân loại câu hỏi: FAQ / Kỹ thuật / Khiếu nại / Khác' }, { name: 'Tra cứu RAG', type: 'Sequential', agent_role: 'Knowledge', prompt: 'Tìm câu trả lời trong kho FAQ operational' }, { name: 'Trả lời khách', type: 'Sequential', agent_role: 'Responder', prompt: 'Soạn câu trả lời ngắn gọn, chuyên nghiệp' }] },
  { id: 'tpl_order_tracker', name: '📦 Tra Cứu Đơn Hàng (SQL RAG)', description: 'Khách hỏi trạng thái đơn hàng bằng tiếng Việt → AI tự truy vấn Database trả kết quả.', tags: ['orders','sql','tracking'], category: 'operations',
    steps: [{ name: 'Nhận yêu cầu', type: 'Sequential', agent_role: 'Listener', prompt: 'Trích xuất mã đơn / tên khách từ tin nhắn' }, { name: 'Query Database', type: 'Sequential', agent_role: 'SQL Analyst', prompt: 'Dịch yêu cầu thành SQL query và truy vấn bảng orders' }, { name: 'Format kết quả', type: 'Sequential', agent_role: 'Formatter', prompt: 'Trình bày trạng thái đơn hàng rõ ràng cho khách' }] },
  { id: 'tpl_appointment', name: '📅 Đặt Lịch Hẹn Tự Động', description: 'Khách nhắn tin đặt lịch → AI check slot trống → Xác nhận lịch hẹn tự động.', tags: ['appointment','scheduling','crm'], category: 'operations',
    steps: [{ name: 'Parse yêu cầu đặt lịch', type: 'Sequential', agent_role: 'Parser', prompt: 'Trích xuất: ngày/giờ mong muốn, dịch vụ, tên khách' }, { name: 'Kiểm tra lịch trống', type: 'Sequential', agent_role: 'Scheduler', prompt: 'Tra cứu Google Calendar/Database tìm slot khả dụng' }, { name: 'Xác nhận & nhắc lịch', type: 'Sequential', agent_role: 'Confirmer', prompt: 'Gửi xác nhận lịch hẹn kèm reminder SMS/Zalo' }] },
  { id: 'tpl_lead_extraction', name: '🎯 Cào Lead từ Mạng Xã Hội', description: 'Stealth Browser quét comment Facebook/Group → Trích xuất Lead tiềm năng → Lưu CRM.', tags: ['leads','social','browser'], category: 'marketing',
    steps: [{ name: 'Mở trang Facebook', type: 'Sequential', agent_role: 'Browser', prompt: 'Stealth Browser mở Group/Page target, scroll lấy comment mới' }, { name: 'AI phân tích intent', type: 'FanOut', agent_role: 'Analyst', prompt: 'Phân biệt comment thể hiện nhu cầu mua vs comment rác' }, { name: 'Lưu Lead', type: 'Collect', agent_role: 'CRM Writer', prompt: 'Lưu tên + SĐT/link FB vào danh sách Lead tiềm năng' }] },
  { id: 'tpl_daily_revenue', name: '📊 Báo Cáo Doanh Thu Hàng Ngày', description: 'Mỗi 8h tối tự động query doanh thu → Tạo báo cáo → Gửi qua Zalo/Telegram cho Boss.', tags: ['report','cron','sql'], category: 'analytics',
    steps: [{ name: 'Query doanh thu', type: 'Sequential', agent_role: 'SQL Analyst', prompt: 'SELECT tổng doanh thu, số đơn, top sản phẩm hôm nay' }, { name: 'So sánh vs hôm qua', type: 'Sequential', agent_role: 'Analyst', prompt: 'So sánh % tăng/giảm với ngày hôm qua và cùng kỳ tuần trước' }, { name: 'Gửi báo cáo', type: 'Sequential', agent_role: 'Reporter', prompt: 'Format báo cáo dạng bảng đẹp, gửi qua kênh Zalo/Telegram của Boss' }] },
  { id: 'tpl_customer_360', name: '👤 Xây Dựng Hồ Sơ KH 360°', description: 'Tổng hợp lịch sử mua + chat + hành vi → Tạo profile khách hàng toàn diện cho AI nhớ.', tags: ['customer','profile','memory'], category: 'crm',
    steps: [{ name: 'Thu thập dữ liệu', type: 'FanOut', agent_role: 'Collector', prompt: 'Song song: query DB đơn hàng + đọc lịch sử chat + check interaction data' }, { name: 'Phân tích hành vi', type: 'Sequential', agent_role: 'Analyst', prompt: 'Tóm tắt: tần suất mua, giá trị trung bình, sở thích, tone chat' }, { name: 'Lưu Memory', type: 'Sequential', agent_role: 'Memory Writer', prompt: 'Lưu profile vào OpenGnothia Cumulative Memory cho agent nhớ lâu dài' }] },
  { id: 'tpl_content_creator', name: '✍️ Tạo Nội Dung Marketing (Reels/Posts)', description: 'AI viết caption + lên lịch đăng → Stealth Browser tự up lên Facebook/Tiktok.', tags: ['content','marketing','social'], category: 'marketing',
    steps: [{ name: 'Lên ý tưởng', type: 'Sequential', agent_role: 'Creative', prompt: 'Dựa trên trend + sản phẩm hot → Gợi ý 3 ý tưởng nội dung' }, { name: 'Viết caption + hashtag', type: 'Sequential', agent_role: 'Writer', prompt: 'Viết caption viral cho SME, thêm hashtag SEO, tone phù hợp thương hiệu' }, { name: 'Đăng bài tự động', type: 'Sequential', agent_role: 'Publisher', prompt: 'Stealth Browser mở Facebook/Tiktok → Upload video + paste caption → Đăng' }] },
  { id: 'tpl_sop_query', name: '📖 Tra Cứu SOP / Quy Trình Nội Bộ', description: 'Nhân viên hỏi quy trình → AI tra SOP trong Knowledge Base → Trả lời chính xác từng bước.', tags: ['sop','internal','compliance'], category: 'operations',
    steps: [{ name: 'Nhận câu hỏi NV', type: 'Sequential', agent_role: 'Listener', prompt: 'Đọc câu hỏi từ kênh nội bộ (Telegram/Discord)' }, { name: 'Tra SOP RAG', type: 'Sequential', agent_role: 'Knowledge', prompt: 'Tìm tài liệu SOP/Chính sách liên quan trong zone Vận Hành (Operational)' }, { name: 'Tóm tắt quy trình', type: 'Sequential', agent_role: 'Summarizer', prompt: 'Trình bày quy trình theo dạng từng bước 1-2-3 ngắn gọn' }] },
  { id: 'tpl_ticket_bot', name: '🎫 Quản Lý Ticket CSKH', description: 'Khách gửi khiếu nại → AI tạo ticket → Phân loại ưu tiên → Giao cho nhân viên phù hợp.', tags: ['ticket','support','automation'], category: 'support',
    steps: [{ name: 'Nhận khiếu nại', type: 'Sequential', agent_role: 'Intake', prompt: 'Đọc tin nhắn khiếu nại, trích xuất: vấn đề, mã đơn, mức độ khẩn' }, { name: 'Phân loại & ưu tiên', type: 'Conditional', agent_role: 'Classifier', prompt: 'Phân loại: Kỹ thuật / Đổi trả / Thanh toán. Ưu tiên: Cao / TB / Thấp' }, { name: 'Tạo Ticket & Assign', type: 'Sequential', agent_role: 'Dispatcher', prompt: 'Tạo ticket trên Kanban, assign cho nhân viên phụ trách đúng category' }] },
];

const TEMPLATE_CATEGORIES = [
  { id: '', label: 'Tất cả', icon: '📋' },
  { id: 'sales', label: 'Bán Hàng', icon: '🛒' },
  { id: 'support', label: 'Hỗ Trợ KH', icon: '🎧' },
  { id: 'operations', label: 'Vận Hành', icon: '⚙️' },
  { id: 'marketing', label: 'Marketing', icon: '📢' },
  { id: 'analytics', label: 'Phân Tích', icon: '📊' },
  { id: 'crm', label: 'CRM', icon: '👤' },
];

function WorkflowsPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [workflows, setWorkflows] = useState([]);
  const [loading, setLoading] = useState(true);
  const [selectedWf, setSelectedWf] = useState(null);
  const [showForm, setShowForm] = useState(false);
  const [editWf, setEditWf] = useState(null);
  const [form, setForm] = useState({name:'',description:'',tags:'',steps:[{name:'',type:'Sequential',agent_role:'',prompt:''}]});
  const [runResult, setRunResult] = useState(null);
  const [running, setRunning] = useState(null);
  const [runInput, setRunInput] = useState('');
  const [showRunInput, setShowRunInput] = useState(null);
  const [templateCat, setTemplateCat] = useState('');
  const [showTemplates, setShowTemplates] = useState(false);

  const visibleTemplates = templateCat ? SME_TEMPLATES.filter(t => t.category === templateCat) : SME_TEMPLATES;

  const useTemplate = (tpl) => {
    setEditWf(null);
    setForm({
      name: tpl.name.replace(/^[^\s]+ /, ''),
      description: tpl.description,
      tags: tpl.tags.join(', '),
      steps: tpl.steps.map(s => ({...s})),
    });
    setShowForm(true);
    setShowTemplates(false);
    showToast('📋 Đã nạp template. Tuỳ chỉnh rồi bấm Tạo!', 'success');
  };

  const load = async () => {
    try {
      const r = await authFetch('/api/v1/workflows');
      if(!r.ok) throw new Error('HTTP '+r.status);
      const d = await r.json();
      setWorkflows(d.workflows || []);
    } catch (e) {
      console.error('Workflows load:', e);
      setWorkflows([]);
    }
    setLoading(false);
  };
  useEffect(() => { load(); }, []);

  const stepTypeIcon = (type) => {
    const icons = { Sequential: '➡️', FanOut: '🔀', Collect: '📥', Conditional: '🔀', Loop: '🔁', Transform: '✨' };
    return icons[type] || '⚙️';
  };
  const stepTypeBadge = (type) => {
    const colors = { Sequential: 'badge-blue', FanOut: 'badge-purple', Collect: 'badge-green', Conditional: 'badge-orange', Loop: 'badge-yellow', Transform: 'badge-blue' };
    return colors[type] || 'badge-blue';
  };
  const stepTypes = ['Sequential','FanOut','Collect','Conditional','Loop','Transform'];

  const openCreate = () => {
    setEditWf(null);
    setForm({name:'',description:'',tags:'',steps:[{name:'Step 1',type:'Sequential',agent_role:'',prompt:''}]});
    setShowForm(true);
  };
  const openEdit = (wf) => {
    if(wf.builtin) { showToast('ℹ️ Template mẫu không chỉnh sửa được. Hãy tạo workflow mới.','info'); return; }
    setEditWf(wf);
    setForm({
      name: wf.name||'',
      description: wf.description||'',
      tags: (wf.tags||[]).join(', '),
      steps: (wf.steps||[]).map(s=>({name:s.name||'',type:s.type||'Sequential',agent_role:s.agent_role||'',prompt:s.prompt||''})),
    });
    setShowForm(true);
  };

  const addStep = () => setForm(f=>({...f, steps:[...f.steps, {name:'Step '+(f.steps.length+1),type:'Sequential',agent_role:'',prompt:''}]}));
  const removeStep = (idx) => setForm(f=>({...f, steps:f.steps.filter((_,i)=>i!==idx)}));
  const updateStep = (idx, key, val) => setForm(f=>({...f, steps:f.steps.map((s,i)=>i===idx?{...s,[key]:val}:s)}));

  const saveWorkflow = async () => {
    if(!form.name.trim()) { showToast('⚠️ Nhập tên workflow','error'); return; }
    if(form.steps.length===0) { showToast('⚠️ Thêm ít nhất 1 step','error'); return; }
    const body = {
      name: form.name,
      description: form.description,
      tags: form.tags.split(',').map(t=>t.trim()).filter(Boolean),
      steps: form.steps,
    };
    try {
      if(editWf && editWf.id) {
        const r = await authFetch('/api/v1/workflows/'+encodeURIComponent(editWf.id), {
          method:'PUT', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)
        });
        if(!r.ok) throw new Error('HTTP '+r.status);
        const d = await r.json();
        if(d.ok) { showToast('✅ Đã cập nhật: '+form.name,'success'); setShowForm(false); load(); }
        else showToast('❌ '+(d.error||'Lỗi'),'error');
      } else {
        const r = await authFetch('/api/v1/workflows', {
          method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify(body)
        });
        if(!r.ok) throw new Error('HTTP '+r.status);
        const d = await r.json();
        if(d.ok) { showToast('✅ Đã tạo: '+form.name,'success'); setShowForm(false); load(); }
        else showToast('❌ '+(d.error||'Lỗi'),'error');
      }
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const runWorkflow = async (wf) => {
    setRunning(wf.id);
    setRunResult(null);
    try {
      const r = await authFetch('/api/v1/workflows/run', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body:JSON.stringify({workflow_id:wf.id, input:runInput})
      });
      if(!r.ok) throw new Error('HTTP '+r.status);
      const d = await r.json();
      if(d.ok) {
        showToast('✅ Hoàn thành: '+wf.name+' ('+d.steps_completed+' steps)','success');
        setRunResult(d);
        setShowRunInput(null);
      } else {
        showToast('❌ '+(d.error||'Lỗi'),'error');
      }
    } catch(e) { showToast('❌ '+e.message,'error'); }
    setRunning(null);
  };

  const deleteWorkflow = async (wf) => {
    if(wf.builtin) { showToast('ℹ️ Không thể xoá template mẫu','info'); return; }
    if(!confirm('Xoá workflow "'+wf.name+'"?')) return;
    try {
      const r = await authFetch('/api/v1/workflows/'+encodeURIComponent(wf.id), {method:'DELETE'});
      if(!r.ok) throw new Error('HTTP '+r.status);
      const d = await r.json();
      if(d.ok) { showToast('🗑️ Đã xoá: '+wf.name,'success'); load(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <div class="page-header"><div>
      <h1>🔄 ${t('wf.title', lang)}</h1>
      <div class="sub">${t('wf.subtitle', lang)}</div>
    </div>
      <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${openCreate}>+ Tạo Workflow</button>
    </div>

    <div class="stats">
      <${StatsCard} label=${t('wf.total', lang)} value=${workflows.length} color="accent" icon="🔄" />
      <${StatsCard} label="Custom" value=${workflows.filter(w=>!w.builtin).length} color="green" icon="✨" />
      <${StatsCard} label="SME Templates" value=${SME_TEMPLATES.length} color="blue" icon="📋" />
    </div>

    <!-- TEMPLATE GALLERY TOGGLE -->
    <div style="margin-bottom:14px">
      <button class="btn ${showTemplates ? '' : 'btn-outline'}" onClick=${() => setShowTemplates(!showTemplates)}
        style="${showTemplates ? 'background:var(--accent);color:#fff;' : ''}padding:8px 18px;font-size:13px;display:flex;align-items:center;gap:6px">
        📦 ${showTemplates ? 'Ẩn' : 'Xem'} Kho Mẫu SME (${SME_TEMPLATES.length} templates)
      </button>
    </div>

    ${showTemplates ? html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent);padding:16px">
        <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:14px">
          <h3 style="margin:0;display:flex;align-items:center;gap:8px">📦 Kho Workflow Mẫu cho Doanh Nghiệp SME
            <span class="badge badge-blue" style="font-size:10px">Easy AI Model</span>
          </h3>
          <button class="btn btn-outline btn-sm" onClick=${() => setShowTemplates(false)}>✕</button>
        </div>
        <p style="font-size:12px;color:var(--text2);margin:0 0 14px">Chọn template → Tuỳ chỉnh Agent Role & Prompt → Chạy. Tiết kiệm 80% thời gian setup.</p>
        <!-- Category Filter -->
        <div style="display:flex;gap:6px;margin-bottom:14px;flex-wrap:wrap">
          ${TEMPLATE_CATEGORIES.map(c => html`
            <button key=${c.id} class="btn btn-sm" onClick=${() => setTemplateCat(c.id)}
              style="padding:5px 12px;border-radius:8px;font-size:11px;font-weight:600;
                border:1px solid ${templateCat === c.id ? 'var(--accent)' : 'var(--border)'};
                background:${templateCat === c.id ? 'var(--accent)' : 'transparent'};
                color:${templateCat === c.id ? '#fff' : 'var(--text2)'}">
              ${c.icon} ${c.label}
            </button>
          `)}
        </div>
        <!-- Template Grid -->
        <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:10px">
          ${visibleTemplates.map(tpl => html`
            <div key=${tpl.id} style="padding:14px;background:var(--bg2);border-radius:10px;border:1px solid var(--border);cursor:pointer;transition:all 0.2s"
              onMouseOver=${e => { e.currentTarget.style.borderColor = 'var(--accent)'; e.currentTarget.style.transform = 'translateY(-1px)'; }}
              onMouseOut=${e => { e.currentTarget.style.borderColor = 'var(--border)'; e.currentTarget.style.transform = 'none'; }}>
              <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:8px">
                <strong style="font-size:13px">${tpl.name}</strong>
              </div>
              <p style="font-size:11px;color:var(--text2);margin:0 0 10px;line-height:1.5">${tpl.description}</p>
              <div style="display:flex;gap:4px;flex-wrap:wrap;margin-bottom:10px">
                ${tpl.tags.map(tag => html`<span key=${tag} class="badge" style="font-size:9px">${tag}</span>`)}
              </div>
              <div style="display:flex;align-items:center;justify-content:space-between">
                <span style="font-size:11px;color:var(--text2)">${tpl.steps.length} steps</span>
                <button class="btn btn-sm" style="background:var(--accent);color:#fff;padding:4px 12px;font-size:11px" onClick=${() => useTemplate(tpl)}>📋 Dùng Mẫu</button>
              </div>
            </div>
          `)}
        </div>
      </div>
    ` : ''}

    ${showForm && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <h3>${editWf ? '✏️ Sửa: '+editWf.name : '➕ Tạo Workflow mới'}</h3>
          <button class="btn btn-outline btn-sm" onClick=${()=>setShowForm(false)}>✕ Đóng</button>
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr;gap:10px;font-size:13px">
          <label>Tên Workflow<input style="${inp}" value=${form.name} onInput=${e=>setForm(f=>({...f,name:e.target.value}))} placeholder="My Workflow" /></label>
          <label>Tags (phân cách bằng dấu phẩy)<input style="${inp}" value=${form.tags} onInput=${e=>setForm(f=>({...f,tags:e.target.value}))} placeholder="content, writing" /></label>
          <label style="grid-column:span 2">Mô tả<input style="${inp}" value=${form.description} onInput=${e=>setForm(f=>({...f,description:e.target.value}))} placeholder="Mô tả ngắn..." /></label>
        </div>

        <h4 style="margin-top:14px;margin-bottom:8px">📋 Steps (${form.steps.length})</h4>
        <div style="display:grid;gap:8px">
          ${form.steps.map((step, idx) => html`
            <div key=${idx} style="padding:10px;background:var(--bg2);border-radius:8px;border:1px solid var(--border)">
              <div style="display:grid;grid-template-columns:1fr 140px 1fr auto;gap:8px;align-items:end;font-size:12px">
                <label>Step Name<input style="${inp}" value=${step.name} onInput=${e=>updateStep(idx,'name',e.target.value)} placeholder="Step name" /></label>
                <label>Type
                  <select style="${inp};cursor:pointer" value=${step.type} onChange=${e=>updateStep(idx,'type',e.target.value)}>
                    ${stepTypes.map(t=>html`<option key=${t} value=${t}>${stepTypeIcon(t)} ${t}</option>`)}
                  </select>
                </label>
                <label>Agent Role<input style="${inp}" value=${step.agent_role} onInput=${e=>updateStep(idx,'agent_role',e.target.value)} placeholder="Writer, Analyst..." /></label>
                <button class="btn btn-outline btn-sm" style="color:var(--red);margin-bottom:2px" onClick=${()=>removeStep(idx)} title="Xoá step">🗑️</button>
              </div>
              <label style="display:block;margin-top:6px;font-size:12px">Prompt (tuỳ chọn)<input style="${inp}" value=${step.prompt||''} onInput=${e=>updateStep(idx,'prompt',e.target.value)} placeholder="Custom prompt cho step này (để trống = auto-generate)" /></label>
            </div>
          `)}
        </div>
        <button class="btn btn-outline btn-sm" style="margin-top:8px" onClick=${addStep}>+ Thêm Step</button>

        <div style="margin-top:14px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${()=>setShowForm(false)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${saveWorkflow}>💾 ${editWf?'Cập nhật':'Tạo'}</button>
        </div>
      </div>
    `}

    ${showRunInput && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--green)">
        <h3 style="margin-bottom:8px">▶ Chạy: ${showRunInput.name}</h3>
        <label style="font-size:13px">Input (context đầu vào cho workflow)
          <textarea style="${inp};min-height:60px;resize:vertical" value=${runInput} onInput=${e=>setRunInput(e.target.value)} placeholder="Nhập nội dung/yêu cầu cho workflow xử lý..." />
        </label>
        <div style="margin-top:10px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${()=>{setShowRunInput(null);setRunInput('');}}>Huỷ</button>
          <button class="btn" style="background:var(--green);color:#fff;padding:8px 20px" onClick=${()=>runWorkflow(showRunInput)} disabled=${running}>
            ${running ? '⏳ Đang chạy...' : '▶ Chạy'}
          </button>
        </div>
      </div>
    `}

    ${runResult && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--green)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px">
          <h3>✅ Kết quả: ${runResult.workflow} (${runResult.steps_completed} steps)</h3>
          <button class="btn btn-outline btn-sm" onClick=${()=>setRunResult(null)}>✕ Đóng</button>
        </div>
        ${(runResult.results||[]).map(r => html`
          <div key=${r.step} style="padding:10px;margin-bottom:8px;background:var(--bg2);border-radius:8px;border-left:3px solid var(--accent)">
            <div style="display:flex;align-items:center;gap:6px;margin-bottom:6px">
              <span class="badge badge-blue">Step ${r.step}</span>
              <strong>${r.name}</strong>
              <span style="color:var(--text2);font-size:11px">→ ${r.agent_role}</span>
            </div>
            <pre style="font-size:12px;white-space:pre-wrap;background:var(--bg);padding:8px;border-radius:4px;margin:0;max-height:200px;overflow-y:auto">${r.output}</pre>
          </div>
        `)}
        <div style="margin-top:10px;padding:10px;background:var(--bg2);border-radius:8px;border-left:3px solid var(--green)">
          <strong>📋 Final Output:</strong>
          <pre style="font-size:12px;white-space:pre-wrap;margin-top:6px;max-height:200px;overflow-y:auto">${runResult.final_output}</pre>
        </div>
      </div>
    `}

    <div style="display:grid;grid-template-columns:1fr 2fr;gap:14px">
      <div class="card">
        <h3 style="margin-bottom:12px">⚙️ ${t('wf.step_types', lang)}</h3>
        <div style="display:grid;gap:6px">
          ${[['Sequential','➡️','Steps run one after another'],['FanOut','🔀','Multiple steps run in parallel'],['Collect','📥','Gather results (All/Best/Vote/Merge)'],['Conditional','🔀','If/else branching'],['Loop','🔁','Repeat until condition met'],['Transform','✨','Template transformation']].map(([name,icon,desc]) => html`
            <div key=${name} style="display:flex;align-items:center;gap:10px;padding:8px 12px;background:var(--bg2);border-radius:6px">
              <span style="font-size:20px">${icon}</span>
              <div style="flex:1"><strong style="font-size:13px">${name}</strong><div style="font-size:11px;color:var(--text2)">${desc}</div></div>
              <span class="badge ${stepTypeBadge(name)}">${name}</span>
            </div>
          `)}
        </div>
      </div>

      <div class="card">
        <h3 style="margin-bottom:12px">📋 Workflows (${workflows.length})</h3>
        ${loading ? html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>` : html`
          <div style="display:grid;gap:8px">
            ${workflows.map(wf => html`<div key=${wf.id} style="padding:12px;background:var(--bg2);border-radius:8px;border:1px solid ${selectedWf===wf.id?'var(--accent)':'var(--border)'};cursor:pointer" onClick=${()=>setSelectedWf(selectedWf===wf.id?null:wf.id)}>
              <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:6px">
                <div style="display:flex;align-items:center;gap:6px">
                  <strong style="font-size:14px">${wf.name}</strong>
                  ${wf.builtin ? html`<span class="badge" style="font-size:9px;opacity:0.6">built-in</span>` : html`<span class="badge badge-green" style="font-size:9px">custom</span>`}
                </div>
                <div style="display:flex;gap:4px;align-items:center">
                  ${(wf.tags||[]).map(tag=>html`<span key=${tag} class="badge" style="font-size:10px">${tag}</span>`)}
                  <button class="btn btn-outline btn-sm" onClick=${(e)=>{e.stopPropagation();setShowRunInput(wf);setRunInput('');}} title="Chạy" disabled=${!!running}>▶</button>
                  ${!wf.builtin && html`<button class="btn btn-outline btn-sm" onClick=${(e)=>{e.stopPropagation();openEdit(wf);}} title="Sửa">✏️</button>`}
                  ${!wf.builtin && html`<button class="btn btn-outline btn-sm" style="color:var(--red)" onClick=${(e)=>{e.stopPropagation();deleteWorkflow(wf);}} title="Xoá">🗑️</button>`}
                </div>
              </div>
              <div style="font-size:12px;color:var(--text2);margin-bottom:8px">${wf.description}</div>
              ${selectedWf===wf.id && html`<div style="display:flex;gap:4px;flex-wrap:wrap;margin-top:8px;padding-top:8px;border-top:1px solid var(--border)">
                ${(wf.steps||[]).map((s,i)=>html`<div key=${i} style="display:flex;align-items:center;gap:4px;padding:4px 8px;background:var(--bg);border-radius:4px;font-size:11px">
                  <span>${stepTypeIcon(s.type)}</span>
                  <strong>${s.name}</strong>
                  <span style="color:var(--text2)">→ ${s.agent_role}</span>
                  ${i<wf.steps.length-1?html`<span style="margin-left:4px">→</span>`:''}
                </div>`)}
              </div>`}
            </div>`)}
          </div>
        `}
      </div>
    </div>
  </div>`;
}


export { WorkflowsPage };
