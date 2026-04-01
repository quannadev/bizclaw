// KnowledgePage — Unified RAG Hub (Doc RAG + SQL RAG)
// Uses window globals from index.html (Preact + HTM)
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

function KnowledgePage({ lang }) {
  const { showToast } = useContext(AppContext);
  // Shared Tabs: doc_rag | sql_rag
  const [activeRagMode, setActiveRagMode] = useState('doc_rag');

  // ==========================================
  // 1. DOC RAG STATE
  // ==========================================
  const [docs, setDocs] = useState([]);
  const [docLoading, setDocLoading] = useState(true);
  const [showAdd, setShowAdd] = useState(false);
  const [addForm, setAddForm] = useState({name:'',content:'',source:'upload'});
  const [uploading, setUploading] = useState(false);
  const [dragOver, setDragOver] = useState(false);

  // Vector Collections (Tài liệu Vector RAG)
  const collections = useMemo(() => {
    const s = new Set(docs.map(d => d.owner).filter(Boolean));
    return Array.from(s).sort();
  }, [docs]);
  const [selectedCol, setSelectedCol] = useState('');
  const [showAddCol, setShowAddCol] = useState(false);
  const [newColName, setNewColName] = useState('');
  const filteredDocs = selectedCol ? docs.filter(d => d.owner === selectedCol) : docs;

  // ==========================================
  // 2. SQL RAG STATE (DB Assistant)
  // ==========================================
  const [connections, setConnections] = useState([]);
  const [selectedConn, setSelectedConn] = useState('');
  const [question, setQuestion] = useState('');
  const [sqlLoading, setSqlLoading] = useState(false);
  const [history, setHistory] = useState([]);
  const [rules, setRules] = useState([]);
  const [newRule, setNewRule] = useState('');
  const [examples, setExamples] = useState([]);
  const [indexedDbs, setIndexedDbs] = useState([]);
  const [activeSqlTab, setActiveSqlTab] = useState('chat');
  const inputRef = useRef(null);

  // Connection Form State
  const [showAddConn, setShowAddConn] = useState(false);
  const [connForm, setConnForm] = useState({ id: '', db_type: 'postgres', connection_string: '', description: '' });

  // ==========================================
  // LOADERS
  // ==========================================
  useEffect(() => {
    if (activeRagMode === 'doc_rag') {
      loadDocs();
    } else {
      loadSqlStatus();
    }
  }, [activeRagMode]);

  useEffect(() => {
    if (activeRagMode === 'sql_rag' && selectedConn) {
      loadRules();
      loadExamples();
    }
  }, [selectedConn, activeRagMode]);

  // --- Doc RAG methods ---
  const loadDocs = async () => {
    try {
      const r = await authFetch('/api/v1/knowledge/documents');
      const d = await r.json();
      setDocs(d.documents||[]);
    } catch(e) {}
    setDocLoading(false);
  };

  const addDoc = async () => {
    if(!addForm.name.trim()||!addForm.content.trim()) { showToast('⚠️ Nhập tên và nội dung','error'); return; }
    try {
      const r = await authFetch('/api/v1/knowledge/documents', {
        method:'POST', headers:{'Content-Type':'application/json'},
        body:JSON.stringify({...addForm, owner: selectedCol})
      });
      const d=await r.json();
      if(d.ok) { showToast('✅ Đã thêm: '+addForm.name+' ('+d.chunks+' chunks)','success'); setShowAdd(false); setAddForm({name:'',content:'',source:'upload'}); loadDocs(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const uploadFile = async (file) => {
    if (!file) return;
    const maxSize = 10 * 1024 * 1024;
    if (file.size > maxSize) { showToast('❌ File quá lớn (tối đa 10MB)', 'error'); return; }
    setUploading(true);
    try {
      const formData = new FormData(); formData.append('file', file);
      formData.append('owner', selectedCol);
      const r = await authFetch('/api/v1/knowledge/upload', { method: 'POST', body: formData });
      const d = await r.json();
      if (d.ok) {
        const sizeKB = Math.round((d.size || file.size) / 1024);
        showToast('✅ ' + d.name + ' → ' + d.chunks + ' chunks (' + sizeKB + 'KB)', 'success');
        loadDocs();
      } else { showToast('❌ ' + (d.error || 'Upload failed'), 'error'); }
    } catch(e) { showToast('❌ Upload error: ' + e.message, 'error'); }
    setUploading(false);
  };

  const onDrop = (e) => {
    e.preventDefault(); setDragOver(false);
    const files = e.dataTransfer?.files;
    if (files && files.length > 0) { for (let i = 0; i < files.length; i++) uploadFile(files[i]); }
  };
  const onDragOver = (e) => { e.preventDefault(); setDragOver(true); };
  const onDragLeave = () => { setDragOver(false); };
  const pickFile = () => {
    const input = document.createElement('input'); input.type = 'file';
    input.accept = '.pdf,.txt,.md,.json,.csv,.log,.toml,.yaml,.yml'; input.multiple = true;
    input.onchange = (e) => { const files = e.target.files; for (let i = 0; i < files.length; i++) uploadFile(files[i]); };
    input.click();
  };
  const deleteDoc = async (id,name) => {
    if(!confirm('Xoá tài liệu "'+name+'"?')) return;
    try {
      const r = await authFetch('/api/v1/knowledge/documents/'+id, {method:'DELETE'});
      const d=await r.json();
      if(d.ok) { showToast('🗑️ Đã xoá: '+name,'success'); loadDocs(); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  // --- SQL RAG methods ---
  const loadSqlStatus = async () => {
    try {
      const r = await authFetch('/api/v1/nl-query/status');
      const d = await r.json();
      setConnections(d.connections || []); setIndexedDbs(d.indexed || []);
      if (d.connections?.length > 0 && !selectedConn) setSelectedConn(d.connections[0].id);
    } catch (e) { console.warn('NL query status:', e); }
  };
  const loadRules = async () => {
    if (!selectedConn) return;
    try {
      const r = await authFetch(`/api/v1/nl-query/rules/${selectedConn}`);
      const d = await r.json(); setRules(d.rules || []);
    } catch (e) {}
  };
  const loadExamples = async () => {
    if (!selectedConn) return;
    try {
      const r = await authFetch(`/api/v1/nl-query/examples/${selectedConn}`);
      const d = await r.json(); setExamples(d.examples || []);
    } catch (e) {}
  };
  const askQuestion = async () => {
    if (!question.trim() || !selectedConn || sqlLoading) return;
    const q = question.trim(); setQuestion(''); setSqlLoading(true);
    setHistory(prev => [...prev, { type: 'user', content: q, time: new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit' }) }]);
    try {
      const r = await authFetch('/api/v1/nl-query/ask', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ connection_id: selectedConn, question: q })
      });
      const d = await r.json();
      setHistory(prev => [...prev, { type: 'bot', content: d.result || d.error || 'No response', sql: d.sql || null, success: d.ok !== false, time: new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit' }) }]);
    } catch (e) {
      setHistory(prev => [...prev, { type: 'error', content: `❌ Error: ${e.message}`, time: new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit' }) }]);
    } finally { setSqlLoading(false); }
  };
  const indexSchema = async () => {
    if (!selectedConn || sqlLoading) return;
    setSqlLoading(true);
    try {
      const r = await authFetch('/api/v1/nl-query/index', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ connection_id: selectedConn })
      });
      const d = await r.json();
      setHistory(prev => [...prev, { type: 'system', content: d.result || d.error || 'Index completed', time: new Date().toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit' }) }]);
      loadSqlStatus();
    } catch (e) { setHistory(prev => [...prev, { type: 'error', content: `❌ ${e.message}` }]); } finally { setSqlLoading(false); }
  };
  const addRule = async () => {
    if (!newRule.trim() || !selectedConn) return;
    try {
      await authFetch('/api/v1/nl-query/rules', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ connection_id: selectedConn, rule: newRule.trim() }) });
      setNewRule(''); loadRules();
    } catch (e) {}
  };
  const addConnection = async () => {
    if (!connForm.id.trim() || !connForm.connection_string.trim()) { showToast('⚠️ Hãy nhập ID và Connection String', 'error'); return; }
    try {
      const r = await authFetch('/api/v1/nl-query/connections', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(connForm)
      });
      const d = await r.json();
      if(d.ok) {
        showToast('✅ Đã lưu kết nối CSDL', 'success');
        setShowAddConn(false);
        setConnForm({ id: '', db_type: 'postgres', connection_string: '', description: '' });
        loadSqlStatus();
      } else { showToast('❌ ' + (d.error || 'Lỗi'), 'error'); }
    } catch(e) { showToast('❌ '+e.message, 'error'); }
  };
  const isIndexed = indexedDbs.includes(selectedConn);

  const dropZoneStyle = dragOver
    ? 'border:2px dashed var(--accent);background:rgba(99,102,241,0.08);border-radius:12px;padding:32px;text-align:center;transition:all 0.2s;cursor:pointer'
    : 'border:2px dashed var(--border);background:var(--bg2);border-radius:12px;padding:32px;text-align:center;transition:all 0.2s;cursor:pointer';
  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <!-- UNIFIED HEADER & MODES -->
    <div class="page-header" style="margin-bottom:16px;">
      <div>
        <h1>📚 Kho Dữ Liệu RAG (Unified Hub)</h1>
        <div class="sub">Tập trung quản lý toàn bộ nguồn kiến thức của Multi-Agent (PDF & SQL DB)</div>
      </div>
      <!-- Mode Switcher -->
      <div style="display:flex;gap:4px;background:var(--bg2);padding:4px;border-radius:8px;border:1px solid var(--border)">
        <button class="btn btn-sm" style="background:${activeRagMode==='doc_rag'?'var(--accent)':'transparent'};color:${activeRagMode==='doc_rag'?'#fff':'var(--text)'};border:none" onClick=${()=>setActiveRagMode('doc_rag')}>
          📄 Tài Liệu (Vector RAG)
        </button>
        <button class="btn btn-sm" style="background:${activeRagMode==='sql_rag'?'var(--accent)':'transparent'};color:${activeRagMode==='sql_rag'?'#fff':'var(--text)'};border:none" onClick=${()=>setActiveRagMode('sql_rag')}>
          🗄️ Database (SQL RAG)
        </button>
      </div>
    </div>

    <!-- MAIN BODY BASED ON MODE -->
    ${activeRagMode === 'doc_rag' ? html`
      <!-- DOC RAG VIEW -->
      <div class="stats" style="margin-bottom:20px">
        <${StatsCard} label="Tổng Tài liệu" value=${docs.length} color="accent" icon="📄" />
        <${StatsCard} label="Vector Chunks" value=${docs.reduce((s,d)=>s+(d.chunks||0),0)} color="blue" icon="📝" />
        <${StatsCard} label="Bộ Sưu Tập (Collections)" value=${collections.length} color="purple" icon="🗂️" />
      </div>

      <div style="display:flex;gap:12px;align-items:center;margin-bottom:16px;flex-wrap:wrap">
        <select value=${selectedCol} onChange=${e => setSelectedCol(e.target.value)}
          style="padding:8px 12px;border-radius:8px;border:1px solid var(--border);background:var(--bg2);color:var(--text);font-size:13px;min-width:200px">
          <option value="">📁 Tất cả tài liệu (Global)</option>
          ${collections.map(c => html`<option key=${c} value=${c}>🗂️ ${c}</option>`)}
        </select>
        <button class="btn btn-outline btn-sm" onClick=${()=>setShowAddCol(!showAddCol)}>+ Tạo Bộ Sưu Tập</button>

        <div style="flex:1"></div>
        <div style="display:flex;gap:8px;justify-content:flex-end">
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${pickFile}>📤 Upload File</button>
          <button class="btn btn-outline" onClick=${()=>setShowAdd(!showAdd)}>✏️ Paste Text</button>
        </div>
      </div>

      ${showAddCol && html`
        <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
          <h3 style="margin-bottom:12px">🗂️ Tạo Bộ Sưu Tập Tài Liệu (Vector Collection)</h3>
          <p style="font-size:12px;color:var(--text2);margin:0 0 16px">Tạo nhóm để phân loại tài liệu (Ví dụ: "tailieu_sales", "kythuat"). Agent có thể kết nối chuyên biệt vào 1 nhánh này giúp RAG chính xác hơn.</p>
          <div style="display:flex;gap:10px;font-size:13px">
            <input style="${inp};margin:0" value=${newColName} onInput=${e=>setNewColName(e.target.value)} placeholder="Tên bộ sưu tập (VD: hdsd_phan_mem_v2)" />
            <button class="btn" style="background:var(--accent);color:#fff;padding:8px 20px" onClick=${() => {
              if(!newColName.trim()) { showToast('⚠️ Nhập tên bộ sưu tập', 'warning'); return; }
              setSelectedCol(newColName.trim());
              setNewColName('');
              setShowAddCol(false);
              showToast('✅ Đã tạo & Chọn bộ sưu tập mới. Giờ anh có thể upload file vào đó.', 'success');
            }}>Tạo & Chọn</button>
          </div>
        </div>
      `}

      <div class="card" style="margin-bottom:14px" onDrop=${onDrop} onDragOver=${onDragOver} onDragLeave=${onDragLeave}>
        <div style="${dropZoneStyle}">
          ${uploading ? html`
            <div style="font-size:32px;margin-bottom:8px">⏳</div>
            <div style="font-size:14px;color:var(--text2)">Đang xử lý...</div>
          ` : html`
            <div style="font-size:32px;margin-bottom:8px">${dragOver ? '📥' : '📄'}</div>
            <div style="font-size:14px;color:var(--text2)">Kéo thả PDF/Word vào đây hoặc click <strong>Upload File</strong></div>
            <div style="margin-top:6px;font-size:11px;color:var(--text2)">Bot sẽ dùng File này làm nguồn học thuật (Q&A/FAQ/CSKH)</div>
          `}
        </div>
      </div>

      ${showAdd && html`
        <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
          <h3 style="margin-bottom:12px">✏️ Paste nội dung trực tiếp</h3>
          <div style="display:grid;gap:10px;font-size:13px">
            <label>Tên tài liệu<input style="${inp}" value=${addForm.name} onInput=${e=>setAddForm(f=>({...f,name:e.target.value}))} placeholder="guide.md" /></label>
            <label>Nội dung<textarea style="${inp};min-height:200px;resize:vertical;font-family:var(--mono)" value=${addForm.content} onInput=${e=>setAddForm(f=>({...f,content:e.target.value}))} placeholder="Paste text..." /></label>
          </div>
          <div style="margin-top:12px;display:flex;gap:8px;justify-content:flex-end">
            <button class="btn btn-outline" onClick=${()=>setShowAdd(false)}>Huỷ</button>
            <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${addDoc}>💾 Thêm</button>
          </div>
        </div>
      `}

      <div class="card">${docLoading?html`<div style="text-align:center;padding:20px;color:var(--text2)">Loading...</div>`:filteredDocs.length===0?html`<div style="text-align:center;padding:40px;color:var(--text2)"><div style="font-size:48px;margin-bottom:12px">📚</div><p>Chưa có tài liệu trong nhóm này. Kéo thả PDF/Word ở trên để nạp tri thức vào kho <strong>${selectedCol||'Global'}</strong> nhé!</p></div>`:html`
        <table><thead><tr><th>Tài liệu</th><th>Chunks</th><th>Bộ Sưu Tập (Owner)</th><th style="text-align:right">Thao tác</th></tr></thead><tbody>
          ${filteredDocs.map(d=>html`<tr key=${d.id}><td><strong>${d.name && d.name.endsWith('.pdf') ? '📄 ' : '📝 '}${d.title||d.name}</strong></td><td>${d.chunks}</td><td style="font-size:12px"><span class="badge ${d.owner ? 'badge-blue' : 'badge-outline'}">${d.owner || 'Global'}</span></td>
            <td style="text-align:right"><button class="btn btn-outline btn-sm" style="color:var(--red)" onClick=${()=>deleteDoc(d.id,d.title||d.name)} title="Xoá">🗑️</button></td>
          </tr>`)}
        </tbody></table>
      `}</div>
    ` : html`
      <!-- SQL RAG VIEW (DB ASSISTANT) -->
      <div class="stats" style="margin-bottom:20px">
        <${StatsCard} icon="🗄️" label="Connections" value=${connections.length} />
        <${StatsCard} icon="📊" label="Indexed" value=${indexedDbs.length} accent="green" />
        <${StatsCard} icon="📝" label="Learned Q&A" value=${examples.length} accent="blue" />
        <${StatsCard} icon="📏" label="Business Rules" value=${rules.length} accent="yellow" />
      </div>

      <div style="display:flex;gap:12px;align-items:center;margin-bottom:16px;flex-wrap:wrap">
        <select value=${selectedConn} onChange=${e => setSelectedConn(e.target.value)}
          style="padding:8px 12px;border-radius:8px;border:1px solid var(--border);background:var(--bg2);color:var(--text);font-size:13px;min-width:200px">
          ${connections.length === 0 ? html`<option>Chưa cấu hình Connection DB</option>` : ''}
          ${connections.map(c => html`<option key=${c.id} value=${c.id}>🗄️ ${c.id} (${c.db_type}) — ${c.description}</option>`)}
        </select>
        <button class="btn btn-outline btn-sm" onClick=${()=>setShowAddConn(!showAddConn)}>+ Thêm Kết nối</button>

        <span class="badge ${isIndexed ? 'badge-green' : 'badge-red'}">${isIndexed ? '✅ Đã Index Data' : '⚠️ Chưa Index Dữ Liệu'}</span>
        ${!isIndexed && selectedConn ? html`
          <button class="btn btn-sm" style="background:var(--accent);color:#fff" onClick=${indexSchema} disabled=${sqlLoading}>
            ${sqlLoading ? '⏳ Indexing...' : '📊 Index Lập Tức'}
          </button>
        ` : ''}
        <div style="flex:1"></div>
        <div style="display:flex;gap:2px;background:var(--bg);border-radius:8px;padding:2px;border:1px solid var(--border)">
          ${['chat', 'rules', 'examples'].map(tab => html`
            <button key=${tab} class="btn btn-sm" onClick=${() => setActiveSqlTab(tab)}
              style="padding:6px 14px;border-radius:6px;font-size:12px;${activeSqlTab === tab ? 'background:var(--accent);color:#fff' : 'background:transparent;color:var(--text2)'}">
              ${tab === 'chat' ? '💬 Phân tích DB' : tab === 'rules' ? '📏 Rules/Kịch bản' : '📝 Examples lưu'}
            </button>
          `)}
        </div>
      </div>

      ${showAddConn && html`
        <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
          <h3 style="margin-bottom:12px">🔌 Thêm kết nối Cơ Sở Dữ Liệu</h3>
          <div style="display:grid;gap:10px;font-size:13px">
            <div style="display:flex;gap:10px">
              <label style="flex:1">ID (vd: primary_db)<input style="${inp}" value=${connForm.id} onInput=${e=>setConnForm(f=>({...f,id:e.target.value}))} placeholder="my_postgres" /></label>
              <label style="flex:1">Loại DB
                <select style="${inp}" value=${connForm.db_type} onChange=${e=>setConnForm(f=>({...f,db_type:e.target.value}))}>
                  <option value="postgres">PostgreSQL</option>
                  <option value="mysql">MySQL</option>
                  <option value="sqlite">SQLite</option>
                  <option value="mongo">MongoDB</option>
                </select>
              </label>
            </div>
            <label>Connection String (URI)<input style="${inp}" value=${connForm.connection_string} onInput=${e=>setConnForm(f=>({...f,connection_string:e.target.value}))} placeholder="postgres://user:pass@localhost:5432/dbname" /></label>
            <label>Mô tả (Dùng làm Context cho Agent biết DB này chứa gì)<input style="${inp}" value=${connForm.description} onInput=${e=>setConnForm(f=>({...f,description:e.target.value}))} placeholder="Database chính chứa dữ liệu đơn hàng và khách hàng..." /></label>
          </div>
          <div style="margin-top:12px;display:flex;gap:8px;justify-content:flex-end">
            <button class="btn btn-outline" onClick=${()=>setShowAddConn(false)}>Huỷ</button>
            <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${addConnection}>💾 Lưu Kết Nối</button>
          </div>
        </div>
      `}

      ${activeSqlTab === 'chat' ? html`
        <div class="card" style="height:calc(100vh - 350px);display:flex;flex-direction:column">
          <div style="flex:1;overflow-y:auto;padding:16px">
            ${history.length === 0 ? html`
              <div style="text-align:center;padding:60px 20px;color:var(--text2)">
                <div style="font-size:48px;margin-bottom:16px">🧠</div>
                <h3 style="font-size:16px;margin:0 0 8px;color:var(--text)">Giao diện test SQL Tool cho Agent</h3>
                <p style="font-size:13px;max-width:400px;margin:0 auto 16px">
                  Anh hỏi thử Data ở đây → AI gọi DB đếm doanh thu → Hiển thị kết quả. Từ đó Bot có thể thay kế toán truy vấn số!
                </p>
                <div style="display:flex;gap:8px;justify-content:center;flex-wrap:wrap">
                  ${['Doanh thu tháng này?', 'Top 10 khách hàng?', 'So sánh QoQ'].map(q => html`
                    <button key=${q} class="btn btn-outline btn-sm" onClick=${() => setQuestion(q)}>${q}</button>
                  `)}
                </div>
              </div>
            ` : html`
              ${history.map((m, i) => html`
                <div key=${i} style="margin-bottom:12px;padding:12px 16px;border-radius:12px;font-size:13px;line-height:1.6;
                  ${m.type === 'user' ? 'background:var(--accent);color:#fff;margin-left:60px;border-bottom-right-radius:4px' :
                    m.type === 'error' ? 'background:rgba(239,68,68,.1);color:var(--red);border:1px solid rgba(239,68,68,.2)' :
                    m.type === 'system' ? 'background:rgba(99,102,241,.05);border:1px solid var(--border)' :
                    'background:var(--bg);border:1px solid var(--border);margin-right:60px;border-bottom-left-radius:4px'}">
                  ${m.sql ? html`
                    <div style="margin-bottom:8px;font-size:11px;color:var(--text2)">Generated SQL:</div>
                    <pre style="background:var(--bg2);padding:10px;border-radius:8px;font-size:12px;font-family:var(--mono);overflow-x:auto;margin-bottom:8px;color:var(--cyan)">${m.sql}</pre>
                  ` : ''}
                  <div style="white-space:pre-wrap">${m.content}</div>
                  ${m.time ? html`<div style="font-size:10px;color:var(--text2);margin-top:4px;text-align:right">${m.time}</div>` : ''}
                </div>
              `)}
              ${sqlLoading ? html`<div style="display:flex;align-items:center;gap:6px;color:var(--text2);font-size:13px;padding:8px">
                <span class="pulse">●</span> AI đang dịch câu hỏi sang SQL Query...
              </div>` : ''}
            `}
          </div>
          <div style="padding:12px 16px;border-top:1px solid var(--border);display:flex;gap:8px">
            <input ref=${inputRef} value=${question} onInput=${e => setQuestion(e.target.value)}
              onKeyDown=${e => e.key === 'Enter' && askQuestion()}
              placeholder="Hỏi AI về cơ sở dữ liệu (Vd: Khách sạn A chốt bao nhiêu đơn?)"
              style="flex:1;padding:10px 14px;border-radius:10px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px" />
            <button class="btn" onClick=${askQuestion} disabled=${sqlLoading || !selectedConn || !isIndexed}
              style="background:var(--accent);color:#fff;padding:10px 20px;border-radius:10px;font-weight:600">
              ${sqlLoading ? '⏳' : '🧠'} Check DB
            </button>
          </div>
        </div>
      ` : activeSqlTab === 'rules' ? html`
        <div class="card" style="padding:20px">
          <h3 style="font-size:15px;margin:0 0 12px;display:flex;align-items:center;gap:8px">
            📏 Business Rules (Tri thức nghiệp vụ SQL)
            <span class="badge badge-outline">${selectedConn}</span>
          </h3>
          <p style="font-size:12px;color:var(--text2);margin:0 0 16px">
            Dạy Agent biết cấu trúc dữ liệu của công ty. (Vd: "Đơn bị Boom hàng là status=CANCELLED, đừng cộng vào Lợi Nhuận")
          </p>
          <div style="display:flex;gap:8px;margin-bottom:16px">
            <input value=${newRule} onInput=${e => setNewRule(e.target.value)}
              onKeyDown=${e => e.key === 'Enter' && addRule()}
              placeholder="Ví dụ: Chỉ đếm khách hàng có is_active=true..."
              style="flex:1;padding:8px 12px;border-radius:8px;border:1px solid var(--border);background:var(--bg);color:var(--text);font-size:13px" />
            <button class="btn btn-sm" style="background:var(--accent);color:#fff" onClick=${addRule}>+ Dạy AI</button>
          </div>
          ${rules.length === 0 ? html`
            <div style="text-align:center;padding:40px;color:var(--text2)">
              📏 Chưa có rules. Thêm rules để AI viết SQL chính xác không bị trừ tiền ảo.
            </div>
          ` : html`
            ${rules.map((r, i) => html`
              <div key=${i} style="padding:10px 14px;margin-bottom:6px;background:var(--bg);border-radius:8px;border:1px solid var(--border);display:flex;align-items:center;gap:10px;font-size:13px">
                <span style="color:var(--accent);font-weight:600">${i + 1}.</span>
                <span style="flex:1">${r.rule}</span>
                <span class="badge badge-outline" style="font-size:10px">${r.connection_id}</span>
              </div>
            `)}
          `}
        </div>
      ` : html`
        <div class="card" style="padding:20px">
          <h3 style="font-size:15px;margin:0 0 12px;display:flex;align-items:center;gap:8px">
            📝 Learned Q&A Pairs (Bộ Nhớ Code SQL)
            <span class="badge badge-green">${examples.length} câu lưu</span>
          </h3>
          <p style="font-size:12px;color:var(--text2);margin:0 0 16px">
            Những câu lệnh SQL mẫu đã được xác nhận đúng 100%. AI sẽ học chép lại khi gặp pattern này ở tương lai.
          </p>
          ${examples.length === 0 ? html`
            <div style="text-align:center;padding:40px;color:var(--text2)">
              📝 Chưa có examples lưu.
            </div>
          ` : html`
            ${examples.map((e, i) => html`
              <div key=${i} style="padding:12px 16px;margin-bottom:8px;background:var(--bg);border-radius:10px;border:1px solid var(--border)">
                <div style="font-size:13px;font-weight:500;margin-bottom:6px">💬 ${e.question}</div>
                <pre style="font-size:12px;font-family:var(--mono);color:var(--cyan);background:var(--bg2);padding:8px 12px;border-radius:6px;margin:0;overflow-x:auto">${e.sql}</pre>
              </div>
            `)}
          `}
        </div>
      `}
    `}
  </div>`;
}

export { KnowledgePage };
