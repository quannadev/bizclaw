// ActivityPage — Enhanced visual timeline with filters, stats, and auto-refresh
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

const EVENT_CONFIG = {
  'llm_call_started':    { icon: '🤖', color: 'var(--accent2)', label: 'LLM Started' },
  'llm_call_completed':  { icon: '✅', color: 'var(--green)',   label: 'LLM Done' },
  'llm_call_error':      { icon: '❌', color: 'var(--red)',     label: 'LLM Error' },
  'tool_call_started':   { icon: '🛠️', color: 'var(--accent)',  label: 'Tool Started' },
  'tool_call_completed': { icon: '🔧', color: 'var(--green)',   label: 'Tool Done' },
  'tool_call_error':     { icon: '⚠️', color: 'var(--red)',     label: 'Tool Error' },
  'scheduler_run':       { icon: '⏰', color: 'var(--accent2)', label: 'Scheduler' },
  'channel_message':     { icon: '📨', color: 'var(--blue)',    label: 'Channel' },
  'ws_connected':        { icon: '🔗', color: 'var(--green)',   label: 'WS Connected' },
  'ws_disconnected':     { icon: '🔌', color: 'var(--red)',     label: 'WS Disconnected' },
};

function getEventConfig(type) {
  for (const [key, cfg] of Object.entries(EVENT_CONFIG)) {
    if (type?.includes(key) || type === key) return cfg;
  }
  if (type?.includes('error')) return { icon: '❌', color: 'var(--red)', label: type };
  if (type?.includes('completed')) return { icon: '✅', color: 'var(--green)', label: type };
  return { icon: '⚡', color: 'var(--accent)', label: type || 'Event' };
}

function TimelineEvent({ event, index }) {
  const cfg = getEventConfig(event.event_type);
  const time = new Date(event.timestamp);
  const timeStr = time.toLocaleTimeString('en-US', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
  const isError = event.event_type?.includes('error');

  return html`<div style="display:flex;gap:12px;position:relative;animation:slideIn 0.3s ease ${Math.min(index * 0.03, 0.5)}s both">
    <!-- Timeline line -->
    <div style="display:flex;flex-direction:column;align-items:center;width:32px;flex-shrink:0">
      <div style="width:32px;height:32px;border-radius:10px;background:${cfg.color}22;border:2px solid ${cfg.color};display:flex;align-items:center;justify-content:center;font-size:14px;z-index:1">${cfg.icon}</div>
      <div style="width:2px;flex:1;background:var(--border);margin-top:4px"></div>
    </div>
    <!-- Content -->
    <div style="flex:1;padding-bottom:16px;min-width:0">
      <div style="display:flex;align-items:center;gap:8px;margin-bottom:4px;flex-wrap:wrap">
        <span class="badge ${isError ? 'badge-red' : 'badge-green'}" style="font-size:10px">${cfg.label}</span>
        ${event.agent ? html`<span style="font-size:11px;color:var(--accent2);font-weight:500">${event.agent}</span>` : null}
        <span style="font-size:10px;color:var(--text2);font-family:var(--mono);margin-left:auto">${timeStr}</span>
      </div>
      ${event.detail ? html`<div style="font-size:12px;color:var(--text1);line-height:1.5;
        padding:6px 10px;background:var(--bg2);border-radius:6px;border:1px solid var(--border);
        max-height:80px;overflow-y:auto;word-break:break-word">${event.detail}</div>` : null}
      ${event.duration_ms ? html`<div style="font-size:10px;color:var(--text2);margin-top:4px">⏱ ${event.duration_ms}ms</div>` : null}
    </div>
  </div>`;
}

function ActivityPage({ lang }) {
  const { showToast } = useContext(window.AppContext);
  const [events, setEvents] = useState([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState('all');
  const [autoRefresh, setAutoRefresh] = useState(true);

  const loadEvents = async () => {
    try {
      const res = await authFetch('/api/v1/activity');
      const data = await res.json();
      setEvents(data.events || []);
    } catch (e) { console.error('Activity load:', e); }
    setLoading(false);
  };

  useEffect(() => {
    loadEvents();
    if (!autoRefresh) return;
    const timer = setInterval(loadEvents, 5000);
    return () => clearInterval(timer);
  }, [autoRefresh]);

  // SSE/WS real-time updates
  useEffect(() => {
    const handler = (e) => {
      const msg = e.detail;
      if (msg?.type === 'activity' && msg.event) {
        setEvents(prev => [msg.event, ...prev].slice(0, 200));
      }
    };
    window.addEventListener('ws-message', handler);
    return () => window.removeEventListener('ws-message', handler);
  }, []);

  const clearActivity = async () => {
    if(!confirm(t('activity.clear_confirm', lang) || 'Xoá tất cả activity?')) return;
    try {
      const r = await authFetch('/api/v1/activity', {method:'DELETE'});
      const d = await r.json();
      if(d.ok) { showToast('🗑️ ' + (t('activity.cleared', lang) || 'Đã xoá') + ' '+(d.cleared||0)+' events','success'); setEvents([]); }
      else showToast('❌ '+(d.error||'Lỗi'),'error');
    } catch(e) { showToast('❌ '+e.message,'error'); }
  };

  const filters = [
    { id: 'all',   label: t('activity.all', lang) || 'Tất cả',   icon: '📋' },
    { id: 'llm',   label: 'LLM',       icon: '🤖' },
    { id: 'tool',  label: 'Tools',     icon: '🛠️' },
    { id: 'error', label: t('activity.errors', lang) || 'Lỗi', icon: '❌' },
    { id: 'channel', label: t('th.channel', lang), icon: '📨' },
  ];

  const filtered = useMemo(() => {
    if (filter === 'all') return events;
    return events.filter(e => e.event_type?.includes(filter));
  }, [events, filter]);

  // Stats
  const errorCount = events.filter(e => e.event_type?.includes('error')).length;
  const llmCount = events.filter(e => e.event_type?.includes('llm')).length;
  const toolCount = events.filter(e => e.event_type?.includes('tool')).length;

  return html`<div>
    <div class="page-header"><div>
      <h1>⚡ ${t('activity.title', lang) || 'Activity Feed'}</h1>
      <div class="sub">${t('activity.subtitle', lang) || 'Dòng thời gian hoạt động hệ thống — cập nhật real-time'}</div>
    </div>
    <div style="display:flex;gap:8px;align-items:center">
      <button class="btn btn-outline btn-sm" onClick=${() => setAutoRefresh(!autoRefresh)}
        style="color:${autoRefresh ? 'var(--green)' : 'var(--text2)'}">
        ${autoRefresh ? '🔄 Auto' : '⏸ Paused'}
      </button>
      <button class="btn btn-outline btn-sm" style="color:var(--red)" onClick=${clearActivity}>🗑️</button>
    </div>
    </div>

    <!-- Stats -->
    <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:12px;margin-bottom:16px">
      <${StatsCard} label=${t('activity.total', lang) || 'Tổng events'} value=${events.length} color="accent" icon="⚡" />
      <${StatsCard} label="LLM Calls" value=${llmCount} color="blue" icon="🤖" />
      <${StatsCard} label="Tool Calls" value=${toolCount} color="accent2" icon="🛠️" />
      <${StatsCard} label=${t('activity.errors', lang) || 'Lỗi'} value=${errorCount} color=${errorCount > 0 ? 'red' : 'green'} icon=${errorCount > 0 ? '❌' : '✅'} />
    </div>

    <!-- Filters -->
    <div style="display:flex;gap:6px;margin-bottom:16px;flex-wrap:wrap">
      ${filters.map(f => html`
        <button key=${f.id} class="btn btn-sm ${filter === f.id ? '' : 'btn-outline'}"
          style="${filter === f.id ? 'background:var(--grad1);color:#fff;border:none' : ''};padding:6px 14px"
          onClick=${() => setFilter(f.id)}>
          ${f.icon} ${f.label} ${f.id !== 'all' ? html`<span style="opacity:0.6;margin-left:4px">(${events.filter(e => f.id === 'all' ? true : e.event_type?.includes(f.id)).length})</span>` : ''}
        </button>
      `)}
    </div>

    <!-- Timeline -->
    <div class="card" style="padding:20px">
      ${loading ? html`<div style="text-align:center;padding:40px;color:var(--text2)">
          <div style="font-size:32px;margin-bottom:8px;animation:pulse 1s infinite">⏳</div>
          <div>${t('activity.loading', lang) || 'Đang tải...'}</div>
        </div>` :
        filtered.length === 0 ? html`<div style="text-align:center;padding:60px;color:var(--text2)">
          <div style="font-size:56px;margin-bottom:16px">🌟</div>
          <h3 style="margin-bottom:8px">${t('activity.empty', lang) || 'Chưa có hoạt động nào'}</h3>
          <p style="font-size:13px">${t('activity.empty_hint', lang) || 'Bắt đầu trò chuyện hoặc chạy lịch trình để xem activity!'}</p>
        </div>` :
        html`<div>${filtered.map((ev, i) => html`<${TimelineEvent} key=${ev.timestamp + '-' + i} event=${ev} index=${i} />`)}</div>`
      }
    </div>
  </div>`;
}


export { ActivityPage };
