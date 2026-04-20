// BizClaw Analytics Dashboard — Real-time metrics & insights v2.0
const { html, useState, useEffect, useCallback } = window;
import { authFetch, t } from '/static/dashboard/shared.js';

const DEMO_DATA = {
  overview: {
    total_messages: 12847, total_tokens: 4283920, total_conversations: 1562,
    avg_latency_ms: 342, active_channels: 5, active_tools: 12,
    cost_usd: 18.42, uptime_percent: 99.7,
    success_rate: 98.5, avg_response_time: 1250
  },
  hourly: [
    { hour: '00', messages: 120 }, { hour: '01', messages: 85 },
    { hour: '02', messages: 62 }, { hour: '03', messages: 45 },
    { hour: '04', messages: 38 }, { hour: '05', messages: 52 },
    { hour: '06', messages: 95 }, { hour: '07', messages: 210 },
    { hour: '08', messages: 420 }, { hour: '09', messages: 580 },
    { hour: '10', messages: 650 }, { hour: '11', messages: 720 },
    { hour: '12', messages: 580 }, { hour: '13', messages: 520 },
    { hour: '14', messages: 680 }, { hour: '15', messages: 750 },
    { hour: '16', messages: 820 }, { hour: '17', messages: 780 },
    { hour: '18', messages: 620 }, { hour: '19', messages: 480 },
    { hour: '20', messages: 380 }, { hour: '21', messages: 290 },
    { hour: '22', messages: 180 }, { hour: '23', messages: 132 }
  ],
  daily: [
    { date: '03/07', messages: 1520, tokens: 512000, cost: 2.1, conversations: 210 },
    { date: '03/08', messages: 1830, tokens: 624000, cost: 2.6, conversations: 245 },
    { date: '03/09', messages: 1640, tokens: 558000, cost: 2.3, conversations: 198 },
    { date: '03/10', messages: 2100, tokens: 715000, cost: 3.0, conversations: 278 },
    { date: '03/11', messages: 1950, tokens: 664000, cost: 2.8, conversations: 256 },
    { date: '03/12', messages: 2210, tokens: 752000, cost: 3.1, conversations: 298 },
    { date: '03/13', messages: 1597, tokens: 458920, cost: 2.5, conversations: 210 }
  ],
  top_tools: [
    { name: 'web_search', calls: 892, avg_ms: 1200, success: 98.2 },
    { name: 'db_query', calls: 645, avg_ms: 85, success: 99.8 },
    { name: 'file', calls: 534, avg_ms: 12, success: 100 },
    { name: 'zalo_tool', calls: 423, avg_ms: 340, success: 97.8 },
    { name: 'http_request', calls: 312, avg_ms: 780, success: 96.5 },
    { name: 'shell', calls: 256, avg_ms: 450, success: 95.2 }
  ],
  channel_stats: [
    { name: 'Zalo Personal', messages: 4520, active_users: 128, satisfaction: 94 },
    { name: 'Telegram', messages: 3210, active_users: 85, satisfaction: 92 },
    { name: 'Discord', messages: 2840, active_users: 62, satisfaction: 89 },
    { name: 'Web Chat', messages: 1680, active_users: 245, satisfaction: 96 },
    { name: 'Webhook', messages: 597, active_users: 12, satisfaction: 98 }
  ],
  provider_usage: [
    { name: 'DeepSeek', tokens: 2100000, cost: 8.4, requests: 3200, latency: 890 },
    { name: 'Gemini', tokens: 1200000, cost: 4.8, requests: 1800, latency: 680 },
    { name: 'OpenAI', tokens: 680000, cost: 3.4, requests: 420, latency: 520 },
    { name: 'Ollama', tokens: 303920, cost: 0, requests: 142, latency: 120 }
  ],
  sentiment: { positive: 72, neutral: 18, negative: 10 }
};

export function AnalyticsPage({ config, lang }) {
  const [metrics, setMetrics] = useState(null);
  const [period, setPeriod] = useState('7d');
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState('overview');
  const [exporting, setExporting] = useState(false);
  const [realtime, setRealtime] = useState(false);

  const loadMetrics = useCallback(async () => {
    setLoading(true);
    try {
      const res = await authFetch(`/api/v1/analytics?period=${period}`);
      if (res.ok) {
        const data = await res.json();
        setMetrics(data);
      } else {
        setMetrics(DEMO_DATA);
      }
    } catch (e) {
      setMetrics(DEMO_DATA);
    }
    setLoading(false);
  }, [period]);

  useEffect(() => {
    loadMetrics();
  }, [loadMetrics]);

  const handleExport = async (format) => {
    setExporting(true);
    try {
      if (metrics) {
        const blob = format === 'json'
          ? new Blob([JSON.stringify(metrics, null, 2)], { type: 'application/json' })
          : new Blob([convertToCSV(metrics)], { type: 'text/csv' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `bizclaw-analytics-${period}.${format}`;
        a.click();
        URL.revokeObjectURL(url);
        window.toastManager?.success(`Đã tải ${format.toUpperCase()} thành công!`);
      }
    } catch (e) {
      window.toastManager?.error('Export thất bại');
    }
    setExporting(false);
  };

  if (loading) return html`<div style="padding:40px;text-align:center;color:var(--text2)">⏳ Đang tải analytics...</div>`;

  const m = metrics?.overview || {};
  const maxMsg = Math.max(...(metrics?.daily || []).map(d => d.messages), 1);
  const maxHour = Math.max(...(metrics?.hourly || []).map(h => h.messages), 1);

  return html`<div>
    <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:20px">
      <div style="display:flex;align-items:center;gap:12px">
        <h2 style="color:var(--text1);margin:0">📊 Analytics Dashboard</h2>
        <span style="
          display:inline-flex;align-items:center;gap:4px;
          padding:4px 10px;background:${realtime ? 'var(--green)' : 'var(--bg2)'};
          color:${realtime ? '#fff' : 'var(--text2)'};border-radius:12px;font-size:11px
        ">
          ${realtime ? '● Live' : '○ Static'}
        </span>
      </div>
      <div style="display:flex;gap:8px;align-items:center">
        <button onClick=${()=>setRealtime(!realtime)} style="
          padding:6px 12px;border-radius:6px;border:1px solid var(--border);
          background:${realtime ? 'var(--green)' : 'transparent'};
          color:${realtime ? '#fff' : 'var(--text2)'};cursor:pointer;font-size:12px
        ">
          ${realtime ? '🔴 Stop' : '🟢 Start Live'}
        </button>
        <select onChange=${e => setPeriod(e.target.value)} value=${period} style="
          padding:8px 12px;border-radius:6px;border:1px solid var(--border);
          background:var(--bg2);color:var(--text1);font-size:12px
        ">
          <option value="24h">24 giờ</option>
          <option value="7d">7 ngày</option>
          <option value="30d">30 ngày</option>
          <option value="90d">90 ngày</option>
        </select>
      </div>
    </div>

    <!-- Tabs -->
    <div style="display:flex;gap:4px;margin-bottom:20px;border-bottom:1px solid var(--border);padding-bottom:12px">
      ${['overview', 'messages', 'performance', 'channels'].map(tab => html`
        <button onClick=${()=>setActiveTab(tab)} style="
          padding:8px 16px;border:none;border-radius:6px 6px 0 0;
          background:${activeTab===tab?'var(--accent)':'transparent'};
          color:${activeTab===tab?'#fff':'var(--text2)'};cursor:pointer;font-size:13px
        ">
          ${tab==='overview'?'📈 Tổng quan':tab==='messages'?'💬 Tin nhắn':tab==='performance'?'⚡ Hiệu năng':'📡 Kênh'}
        </button>
      `)}
    </div>

    ${activeTab === 'overview' && html`
      <!-- KPI Cards -->
      <div style="display:grid;grid-template-columns:repeat(4,1fr);gap:12px;margin-bottom:24px">
        ${[
          { icon: '💬', label: 'Tin nhắn', value: (m.total_messages||0).toLocaleString(), sub: (m.total_conversations||0)+' cuộc hội thoại', color: '#6366f1', trend: '+12%' },
          { icon: '🎯', label: 'Tokens', value: ((m.total_tokens||0)/1000000).toFixed(1)+'M', sub: (m.avg_latency_ms||0)+'ms latency TB', color: '#10b981', trend: '-5%' },
          { icon: '💰', label: 'Chi phí', value: '$'+(m.cost_usd||0).toFixed(2), sub: m.active_channels+' kênh hoạt động', color: '#f59e0b', trend: '+3%' },
          { icon: '⚡', label: 'Uptime', value: (m.uptime_percent||0)+'%', sub: (m.success_rate||0)+'% thành công', color: '#ef4444', trend: '+0.2%' }
        ].map(card => html`
          <div class="card" style="padding:16px;position:relative">
            <div style="position:absolute;top:8px;right:8px;font-size:11px;padding:2px 6px;border-radius:4px;background:${card.trend.startsWith('+')?'rgba(16,185,129,0.1)':'rgba(239,68,68,0.1)'};color:${card.trend.startsWith('+')?'var(--green)':'var(--red)'}">
              ${card.trend}
            </div>
            <div style="font-size:12px;color:var(--text2);margin-bottom:4px">${card.icon} ${card.label}</div>
            <div style="font-size:28px;font-weight:700;color:${card.color}">${card.value}</div>
            <div style="font-size:11px;color:var(--text2);margin-top:4px">${card.sub}</div>
          </div>
        `)}
      </div>

      <div style="display:grid;grid-template-columns:2fr 1fr;gap:16px;margin-bottom:20px">
        <!-- Daily Chart -->
        <div class="card" style="padding:16px">
          <h3 style="margin:0 0 12px;font-size:14px;color:var(--text1)">📈 Tin nhắn theo ngày</h3>
          <div style="display:flex;align-items:flex-end;gap:8px;height:180px;padding-bottom:24px">
            ${(metrics?.daily||[]).map((d, i) => html`
              <div style="flex:1;display:flex;flex-direction:column;align-items:center;height:100%;justify-content:flex-end">
                <div style="font-size:10px;color:var(--text2);margin-bottom:4px">${d.messages}</div>
                <div style="
                  width:100%;background:linear-gradient(to top,#6366f1,#818cf8);
                  border-radius:6px 6px 0 0;height:${(d.messages/maxMsg*100).toFixed(0)}%;
                  min-height:8px;transition:height 0.5s
                "></div>
                <div style="font-size:10px;color:var(--text2);margin-top:6px">${d.date}</div>
              </div>
            `)}
          </div>
          <div style="display:flex;gap:16px;margin-top:12px;padding-top:12px;border-top:1px solid var(--border)">
            <div style="font-size:11px;color:var(--text2)">💬 Trung bình: <strong style="color:var(--text1)">${Math.round((metrics?.daily||[]).reduce((a,b)=>a+b.messages,0)/(metrics?.daily||[]).length||0)}</strong> tin nhắn/ngày</div>
            <div style="font-size:11px;color:var(--text2)">💵 Chi phí TB: <strong style="color:var(--text1)">$${((metrics?.daily||[]).reduce((a,b)=>a+b.cost,0)/(metrics?.daily||[]).length||0).toFixed(2)}</strong>/ngày</div>
          </div>
        </div>

        <!-- Sentiment -->
        <div class="card" style="padding:16px">
          <h3 style="margin:0 0 12px;font-size:14px;color:var(--text1)">😊 Phản hồi khách hàng</h3>
          <div style="display:flex;align-items:center;justify-content:center;margin-bottom:16px">
            ${renderPieChart(metrics?.sentiment||{positive:72,neutral:18,negative:10})}
          </div>
          <div style="display:flex;flex-direction:column;gap:8px">
            ${[['positive', 'Tích cực', 'var(--green)'],['neutral', 'Trung lập', 'var(--orange)'],['negative', 'Tiêu cực', 'var(--red)']].map(([key, label, color]) => html`
              <div style="display:flex;align-items:center;gap:8px">
                <div style="width:12px;height:12px;border-radius:50%;background:${color}"></div>
                <span style="flex:1;font-size:12px;color:var(--text2)">${label}</span>
                <span style="font-size:13px;font-weight:600;color:var(--text1)">${(metrics?.sentiment||{})[key]||0}%</span>
              </div>
            `)}
          </div>
        </div>
      </div>
    `}

    ${activeTab === 'messages' && html`
      <div style="display:grid;grid-template-columns:1fr;gap:16px">
        <!-- Hourly Chart -->
        <div class="card" style="padding:16px">
          <h3 style="margin:0 0 12px;font-size:14px;color:var(--text1)">🕐 Tin nhắn theo giờ</h3>
          <div style="display:flex;align-items:flex-end;gap:2px;height:120px;padding-bottom:20px">
            ${(metrics?.hourly||[]).map(h => html`
              <div style="
                flex:1;height:${(h.messages/maxHour*100).toFixed(0)}%;min-height:4px;
                background:${parseInt(h.hour)>=9&&parseInt(h.hour)<=17?'#6366f1':'#818cf8'};
                border-radius:2px 2px 0 0;transition:height 0.3s;cursor:pointer
              " title="${h.hour}:00 - ${h.messages} tin nhắn"></div>
            `)}
          </div>
          <div style="display:flex;justify-content:space-between;font-size:10px;color:var(--text2);margin-top:4px">
            <span>00:00</span><span>06:00</span><span>12:00</span><span>18:00</span><span>23:00</span>
          </div>
          <div style="display:flex;gap:24px;margin-top:12px;padding-top:12px;border-top:1px solid var(--border)">
            <div style="display:flex;align-items:center;gap:6px;font-size:11px">
              <div style="width:12px;height:12px;background:#6366f1;border-radius:2px"></div>
              <span style="color:var(--text2)">Giờ cao điểm (9-17h)</span>
            </div>
            <div style="display:flex;align-items:center;gap:6px;font-size:11px">
              <div style="width:12px;height:12px;background:#818cf8;border-radius:2px"></div>
              <span style="color:var(--text2)">Giờ thấp điểm</span>
            </div>
          </div>
        </div>
      </div>
    `}

    ${activeTab === 'performance' && html`
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px">
        <!-- Provider Usage -->
        <div class="card" style="padding:16px">
          <h3 style="margin:0 0 12px;font-size:14px;color:var(--text1)">🤖 Provider Usage</h3>
          ${(metrics?.provider_usage||[]).map(p => {
            const maxTok = Math.max(...(metrics?.provider_usage||[]).map(x=>x.tokens),1);
            return html`
              <div style="margin-bottom:14px">
                <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:4px">
                  <span style="font-size:13px;color:var(--text1);font-weight:500">${p.name}</span>
                  <span style="font-size:11px;color:var(--text2)">${(p.tokens/1000000).toFixed(1)}M tok · $${p.cost}</span>
                </div>
                <div style="height:8px;background:var(--bg);border-radius:4px;overflow:hidden">
                  <div style="height:100%;width:${(p.tokens/maxTok*100).toFixed(0)}%;background:linear-gradient(90deg,#6366f1,#10b981);border-radius:4px;transition:width 0.5s"></div>
                </div>
                <div style="display:flex;gap:12px;margin-top:4px;font-size:10px;color:var(--text2)">
                  <span>Latency: ${p.latency}ms</span>
                  <span>Requests: ${p.requests}</span>
                </div>
              </div>
            `;
          })}
        </div>

        <!-- Top Tools -->
        <div class="card" style="padding:16px">
          <h3 style="margin:0 0 12px;font-size:14px;color:var(--text1)">🔧 Top Tools</h3>
          <table style="width:100%;font-size:12px">
            <tr style="color:var(--text2)">
              <th style="text-align:left;padding:4px 0;font-weight:500">Tool</th>
              <th style="text-align:right;font-weight:500">Calls</th>
              <th style="text-align:right;font-weight:500">Latency</th>
              <th style="text-align:right;font-weight:500">Success</th>
            </tr>
            ${(metrics?.top_tools||[]).map((tool, i) => html`
              <tr style="border-top:1px solid var(--border)">
                <td style="padding:8px 0">
                  <span style="color:var(--text2);margin-right:6px">${i+1}.</span>
                  <span style="color:var(--text1)">${tool.name}</span>
                </td>
                <td style="text-align:right;color:var(--accent);font-weight:600">${tool.calls}</td>
                <td style="text-align:right;color:${tool.avg_ms>500?'var(--red)':'var(--green)'}">${tool.avg_ms}ms</td>
                <td style="text-align:right;color:${tool.success>=99?'var(--green)':'var(--orange)'}">${tool.success}%</td>
              </tr>
            `)}
          </table>
        </div>
      </div>
    `}

    ${activeTab === 'channels' && html`
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:16px">
        <!-- Channel Stats -->
        <div class="card" style="padding:16px">
          <h3 style="margin:0 0 12px;font-size:14px;color:var(--text1)">📡 Hoạt động kênh</h3>
          ${(metrics?.channel_stats||[]).map(ch => {
            const maxMsg = Math.max(...(metrics?.channel_stats||[]).map(x=>x.messages),1);
            return html`
              <div style="margin-bottom:14px">
                <div style="display:flex;justify-content:space-between;margin-bottom:4px">
                  <span style="font-size:13px;color:var(--text1)">${ch.name}</span>
                  <span style="font-size:11px;color:var(--text2)">${ch.messages.toLocaleString()} tin · ${ch.active_users} users</span>
                </div>
                <div style="height:6px;background:var(--bg);border-radius:3px;overflow:hidden">
                  <div style="height:100%;width:${(ch.messages/maxMsg*100).toFixed(0)}%;background:var(--accent);border-radius:3px"></div>
                </div>
              </div>
            `;
          })}
        </div>

        <!-- Channel Satisfaction -->
        <div class="card" style="padding:16px">
          <h3 style="margin:0 0 12px;font-size:14px;color:var(--text1)">⭐ Mức độ hài lòng</h3>
          ${(metrics?.channel_stats||[]).map(ch => {
            const maxSat = 100;
            return html`
              <div style="margin-bottom:12px">
                <div style="display:flex;justify-content:space-between;margin-bottom:4px">
                  <span style="font-size:12px;color:var(--text1)">${ch.name}</span>
                  <span style="font-size:12px;font-weight:600;color:${ch.satisfaction>=95?'var(--green)':ch.satisfaction>=90?'var(--orange)':'var(--red)'}">${ch.satisfaction}%</span>
                </div>
                <div style="height:6px;background:var(--bg);border-radius:3px;overflow:hidden">
                  <div style="
                    height:100%;width:${ch.satisfaction}%;
                    background:${ch.satisfaction>=95?'var(--green)':ch.satisfaction>=90?'var(--orange)':'var(--red)'};
                    border-radius:3px
                  "></div>
                </div>
              </div>
            `;
          })}
        </div>
      </div>
    `}

    <!-- Export Section -->
    <div class="card" style="padding:16px;margin-top:16px;display:flex;justify-content:space-between;align-items:center">
      <div>
        <span style="font-size:13px;color:var(--text1);font-weight:500">📥 Xuất dữ liệu</span>
        <span style="font-size:11px;color:var(--text2);margin-left:12px">Kỳ: ${period}</span>
      </div>
      <div style="display:flex;gap:8px">
        <button 
          onClick=${()=>handleExport('csv')}
          disabled=${exporting}
          style="
            padding:8px 16px;border-radius:6px;border:1px solid var(--border);
            background:transparent;color:var(--text1);cursor:pointer;font-size:12px;
            display:flex;align-items:center;gap:6px
          "
        >
          📊 CSV
        </button>
        <button 
          onClick=${()=>handleExport('json')}
          disabled=${exporting}
          style="
            padding:8px 16px;border-radius:6px;border:1px solid var(--border);
            background:transparent;color:var(--text1);cursor:pointer;font-size:12px;
            display:flex;align-items:center;gap:6px
          "
        >
          📋 JSON
        </button>
        <button 
          style="
            padding:8px 16px;border-radius:6px;border:1px solid var(--accent);
            background:var(--accent);color:#fff;cursor:pointer;font-size:12px;
            display:flex;align-items:center;gap:6px
          "
        >
          📧 Email Report
        </button>
      </div>
    </div>
  </div>`;
}

function renderPieChart(sentiment) {
  const total = sentiment.positive + sentiment.neutral + sentiment.negative;
  const pos = (sentiment.positive / total * 100).toFixed(0);
  const neu = (sentiment.neutral / total * 100).toFixed(0);
  const neg = (sentiment.negative / total * 100).toFixed(0);

  return html`
    <div style="position:relative;width:120px;height:120px">
      <svg viewBox="0 0 36 36" style="width:100%;height:100%;transform:rotate(-90deg)">
        <circle cx="18" cy="18" r="14" fill="none" stroke="var(--bg)" stroke-width="3"></circle>
        <circle cx="18" cy="18" r="14" fill="none" stroke="var(--green)" stroke-width="3"
          stroke-dasharray="${pos} ${100-parseInt(pos)}" stroke-dashoffset="0"></circle>
        <circle cx="18" cy="18" r="14" fill="none" stroke="var(--orange)" stroke-width="3"
          stroke-dasharray="${neu} ${100-parseInt(neu)}" stroke-dashoffset="-${pos}"></circle>
        <circle cx="18" cy="18" r="14" fill="none" stroke="var(--red)" stroke-width="3"
          stroke-dasharray="${neg} ${100-parseInt(neg)}" stroke-dashoffset="-${parseInt(pos)+parseInt(neu)}"></circle>
      </svg>
      <div style="
        position:absolute;inset:0;display:flex;flex-direction:column;align-items:center;justify-content:center;
        text-align:center
      ">
        <div style="font-size:22px;font-weight:700;color:var(--text1)">${pos}%</div>
        <div style="font-size:9px;color:var(--text2)">Tích cực</div>
      </div>
    </div>
  `;
}

function convertToCSV(data) {
  const headers = ['Metric', 'Value'];
  const rows = [
    ['Total Messages', data.overview?.total_messages],
    ['Total Tokens', data.overview?.total_tokens],
    ['Total Cost', '$' + data.overview?.cost_usd],
    ['Uptime', data.overview?.uptime_percent + '%'],
    ['Success Rate', data.overview?.success_rate + '%'],
    [''],
    ['Date', 'Messages', 'Tokens', 'Cost'],
    ...(data.daily || []).map(d => [d.date, d.messages, d.tokens, '$' + d.cost])
  ];

  return [headers, ...rows].map(row => row.join(',')).join('\n');
}
