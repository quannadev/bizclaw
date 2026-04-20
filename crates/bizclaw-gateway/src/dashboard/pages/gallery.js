// BizClaw Agent Templates Gallery — Pre-built workflows for popular use cases
const { html, useState, useEffect, useCallback } = window;
import { authFetch, t } from '/static/dashboard/shared.js';

const TEMPLATES = [
  {
    id: 'zalo-support',
    icon: '💬',
    name: 'Zalo Support Agent',
    description: 'Trả lời tin nhắn Zalo tự động 24/7 với khả năng chuyển human khi cần',
    category: 'customer-service',
    tags: ['Zalo', 'Support', '24/7', 'Vietnam'],
    setup: '5 phút',
    difficulty: 'easy',
    agents: [
      { name: 'Support Agent', role: 'primary', model: 'gemini/gemini-2.0-flash' }
    ],
    channels: ['zalo'],
    tools: ['zalo_tool', 'knowledge_search', 'product_lookup'],
    workflow: 'sequential',
    config: {
      system_prompt: 'Bạn là nhân viên chăm sóc khách hàng của cửa hàng. Trả lời lịch sự, nhanh chóng. Nếu câu hỏi phức tạp hoặc khách hàng yêu cầu nói chuyện với người, chuyển sang human handoff.',
      response_tone: 'friendly',
      max_tokens: 500
    },
    featured: true
  },
  {
    id: 'facebook-sales',
    icon: '📘',
    name: 'Facebook Sales Bot',
    description: 'Tự động trả lời tin nhắn Facebook, trả lời câu hỏi về sản phẩm và giá cả',
    category: 'sales',
    tags: ['Facebook', 'Sales', 'E-commerce', 'Products'],
    setup: '10 phút',
    difficulty: 'easy',
    agents: [
      { name: 'Sales Agent', role: 'primary', model: 'openai/gpt-4o-mini' }
    ],
    channels: ['facebook'],
    tools: ['product_lookup', 'price_check', 'inventory_status'],
    workflow: 'sequential',
    config: {
      system_prompt: 'Bạn là nhân viên bán hàng chuyên nghiệp. Tư vấn sản phẩm, báo giá, và hướng dẫn đặt hàng.',
      auto_replies: ['Chào bạn!', 'Cảm ơn đã liên hệ', 'Sản phẩm này đang có khuyến mãi']
    },
    featured: true
  },
  {
    id: 'restaurant-booking',
    icon: '🍜',
    name: 'Restaurant Booking Agent',
    description: 'Đặt bàn tự động, gửi menu hàng ngày, xác nhận đơn hàng',
    category: 'booking',
    tags: ['Restaurant', 'Booking', 'F&B', 'Zalo'],
    setup: '15 phút',
    difficulty: 'medium',
    agents: [
      { name: 'Booking Agent', role: 'primary', model: 'gemini/gemini-2.0-flash' },
      { name: 'Reminder Agent', role: 'scheduler', model: 'gemini/gemini-2.0-flash' }
    ],
    channels: ['zalo'],
    tools: ['calendar', 'menu_lookup', 'booking_create', 'notification_send'],
    workflow: 'conditional',
    config: {
      system_prompt: 'Bạn là lễ tân nhà hàng. Tiếp nhận đặt bàn, xác nhận thông tin, gửi nhắc nhở trước giờ đến.',
      working_hours: '09:00-22:00',
      languages: ['vi', 'en']
    },
    featured: false
  },
  {
    id: 'hotel-booking',
    icon: '🏨',
    name: 'Hotel Concierge Agent',
    description: 'Xác nhận booking từ Booking.com, Agoda, chăm sóc khách 24/7',
    category: 'booking',
    tags: ['Hotel', 'Concierge', 'Booking', 'Multi-channel'],
    setup: '20 phút',
    difficulty: 'medium',
    agents: [
      { name: 'Concierge', role: 'primary', model: 'gemini/gemini-2.0-flash' },
      { name: 'Review Manager', role: 'secondary', model: 'openai/gpt-4o-mini' }
    ],
    channels: ['zalo', 'telegram', 'webhook'],
    tools: ['booking_sync', 'room_availability', 'checkout_reminder', 'review_request'],
    workflow: 'fan-out',
    config: {
      system_prompt: 'Bạn là quản gia khách sạn cao cấp. Chào đón khách, giải đáp thắc mắc, hỗ trợ 24/7.',
      sync_platforms: ['booking.com', 'agoda', 'airbnb']
    },
    featured: false
  },
  {
    id: 'lead-qualification',
    icon: '🎯',
    name: 'Lead Qualification Pipeline',
    description: 'Sàng lọc và phân loại leads tự động từ nhiều nguồn',
    category: 'marketing',
    tags: ['Lead', 'CRM', 'Qualification', 'Automation'],
    setup: '15 phút',
    difficulty: 'medium',
    agents: [
      { name: 'Lead Collector', role: 'collector', model: 'gemini/gemini-2.0-flash' },
      { name: 'Lead Qualifier', role: 'processor', model: 'openai/gpt-4o' },
      { name: 'Lead Router', role: 'router', model: 'gemini/gemini-2.0-flash' }
    ],
    channels: ['zalo', 'facebook', 'webhook'],
    tools: ['lead_scoring', 'crm_update', 'email_notification', 'slack_alert'],
    workflow: 'sequential',
    config: {
      qualification_questions: [
        'Bạn đang quan tâm đến sản phẩm gì?',
        'Ngân sách dự kiến là bao nhiêu?',
        'Thời gian triển khai mong muốn?'
      ],
      score_thresholds: { hot: 80, warm: 50, cold: 20 }
    },
    featured: true
  },
  {
    id: 'content-creator',
    icon: '✍️',
    name: 'Content Creation Pipeline',
    description: 'Tạo nội dung đa nền tảng: Facebook, Zalo, Email từ một brief',
    category: 'marketing',
    tags: ['Content', 'Social Media', 'Multi-platform', 'Automation'],
    setup: '10 phút',
    difficulty: 'easy',
    agents: [
      { name: 'Content Strategist', role: 'planner', model: 'openai/gpt-4o' },
      { name: 'Copywriter', role: 'writer', model: 'gemini/gemini-2.0-flash' },
      { name: 'Image Selector', role: 'asset', model: 'gemini/gemini-2.0-flash' }
    ],
    channels: ['zalo', 'facebook', 'email'],
    tools: ['content_generate', 'image_search', 'hashtag_suggest', 'schedule_post'],
    workflow: 'sequential',
    config: {
      platforms: ['facebook', 'zalo', 'email'],
      content_tones: ['formal', 'friendly', 'casual'],
      auto_schedule: true
    },
    featured: true
  },
  {
    id: 'ecommerce-support',
    icon: '🛒',
    name: 'E-commerce Support Agent',
    description: 'Hỗ trợ đơn hàng, tracking vận chuyển, xử lý khiếu nại',
    category: 'customer-service',
    tags: ['E-commerce', 'Order', 'Tracking', 'Support'],
    setup: '15 phút',
    difficulty: 'medium',
    agents: [
      { name: 'Order Support', role: 'primary', model: 'gemini/gemini-2.0-flash' },
      { name: 'Complaint Handler', role: 'escalation', model: 'openai/gpt-4o' }
    ],
    channels: ['zalo', 'facebook', 'shopee'],
    tools: ['order_lookup', 'tracking_status', 'refund_request', 'rating_response'],
    workflow: 'conditional',
    config: {
      auto_tracking: true,
      refund_threshold: 500000,
      escalation_keywords: ['khiếu nại', 'hoàn tiền', 'không hài lòng']
    },
    featured: false
  },
  {
    id: 'appointment-reminder',
    icon: '📅',
    name: 'Appointment & Reminder System',
    description: 'Đặt lịch hẹn, gửi nhắc nhở tự động qua Zalo, SMS, Email',
    category: 'scheduling',
    tags: ['Appointment', 'Reminder', 'Scheduling', 'Spa', 'Clinic'],
    setup: '10 phút',
    difficulty: 'easy',
    agents: [
      { name: 'Appointment Agent', role: 'primary', model: 'gemini/gemini-2.0-flash' }
    ],
    channels: ['zalo', 'sms', 'email'],
    tools: ['calendar_check', 'appointment_create', 'reminder_send', 'confirmation_request'],
    workflow: 'sequential',
    config: {
      reminder_times: ['24h', '2h', '30min'],
      working_hours: '08:00-20:00',
      services: ['massage', 'spa', 'clinic', 'salon']
    },
    featured: false
  },
  {
    id: 'market-research',
    icon: '🔍',
    name: 'Market Research Agent',
    description: 'Thu thập và phân tích thông tin thị trường, đối thủ cạnh tranh',
    category: 'research',
    tags: ['Research', 'Market', 'Analysis', 'Competitor'],
    setup: '5 phút',
    difficulty: 'easy',
    agents: [
      { name: 'Research Agent', role: 'primary', model: 'openai/gpt-4o' }
    ],
    channels: ['internal'],
    tools: ['web_search', 'web_fetch', 'competitor_analysis', 'report_generate'],
    workflow: 'sequential',
    config: {
      research_topics: ['pricing', 'features', 'reviews', 'trends'],
      report_format: 'markdown',
      schedule: 'weekly'
    },
    featured: false
  },
  {
    id: 'social-media-manager',
    icon: '📱',
    name: 'Social Media Manager',
    description: 'Quản lý đăng bài, phản hồi comments, inbox messages đa nền tảng',
    category: 'marketing',
    tags: ['Social Media', 'Multi-platform', 'Auto-reply', 'Content'],
    setup: '20 phút',
    difficulty: 'medium',
    agents: [
      { name: 'Content Publisher', role: 'publisher', model: 'gemini/gemini-2.0-flash' },
      { name: 'Inbox Manager', role: 'inbox', model: 'openai/gpt-4o-mini' },
      { name: 'Comment Responder', role: 'comments', model: 'gemini/gemini-2.0-flash' }
    ],
    channels: ['facebook', 'instagram', 'zalo'],
    tools: ['post_schedule', 'inbox_aggregate', 'comment_reply', 'dm_respond'],
    workflow: 'parallel',
    config: {
      platforms: ['facebook', 'instagram', 'zalo'],
      response_tone: 'friendly_professional',
      auto_posting: true,
      content_library: 'shared'
    },
    featured: false
  }
];

const CATEGORIES = [
  { id: 'all', icon: '🌟', name: 'Tất cả' },
  { id: 'customer-service', icon: '💬', name: 'Chăm sóc khách' },
  { id: 'sales', icon: '💰', name: 'Bán hàng' },
  { id: 'marketing', icon: '📢', name: 'Marketing' },
  { id: 'booking', icon: '📅', name: 'Đặt lịch' },
  { id: 'scheduling', icon: '⏰', name: 'Nhắc nhở' },
  { id: 'research', icon: '🔬', name: 'Nghiên cứu' }
];

export function GalleryPage({ config, lang }) {
  const [selectedCategory, setSelectedCategory] = useState('all');
  const [selectedTemplate, setSelectedTemplate] = useState(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [installing, setInstalling] = useState(null);

  const filteredTemplates = TEMPLATES.filter(t => {
    const matchesCategory = selectedCategory === 'all' || t.category === selectedCategory;
    const matchesSearch = !searchQuery || 
      t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      t.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      t.tags.some(tag => tag.toLowerCase().includes(searchQuery.toLowerCase()));
    return matchesCategory && matchesSearch;
  });

  const handleInstall = async (template) => {
    setInstalling(template.id);
    try {
      const res = await authFetch('/api/v1/templates/install', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(template)
      });
      if (res.ok) {
        window.toastManager?.success(`Đã cài đặt template "${template.name}"!`);
      } else {
        window.toastManager?.error('Cài đặt thất bại. Vui lòng thử lại.');
      }
    } catch (e) {
      window.toastManager?.error('Lỗi kết nối. Vui lòng thử lại.');
    }
    setInstalling(null);
  };

  return html`
    <div>
      <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:20px">
        <h2 style="color:var(--text1);margin:0">📦 Agent Templates</h2>
        <div style="display:flex;gap:8px">
          <input 
            type="search" 
            placeholder="Tìm kiếm template..." 
            value=${searchQuery}
            onInput=${e => setSearchQuery(e.target.value)}
            style="padding:8px 12px;border-radius:8px;border:1px solid var(--border);background:var(--bg2);color:var(--text1)"
          />
        </div>
      </div>

      <!-- Categories -->
      <div style="display:flex;gap:8px;margin-bottom:24px;flex-wrap:wrap">
        ${CATEGORIES.map(cat => html`
          <button 
            onClick=${() => setSelectedCategory(cat.id)}
            style="
              padding:8px 16px;border-radius:20px;border:1px solid ${selectedCategory === cat.id ? 'var(--accent)' : 'var(--border)'};
              background:${selectedCategory === cat.id ? 'var(--accent)' : 'transparent'};
              color:${selectedCategory === cat.id ? '#fff' : 'var(--text2)'};
              cursor:pointer;font-size:13px;display:flex;align-items:center;gap:6px
            "
          >
            ${cat.icon} ${cat.name}
          </button>
        `)}
      </div>

      <!-- Featured Templates -->
      ${selectedCategory === 'all' && !searchQuery && html`
        <div style="margin-bottom:32px">
          <h3 style="margin:0 0 16px;color:var(--text1);font-size:16px">⭐ Được khuyên dùng</h3>
          <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:16px">
            ${TEMPLATES.filter(t => t.featured).slice(0, 3).map(t => TemplateCard({ template: t, onSelect: setSelectedTemplate, onInstall: handleInstall, installing }))}
          </div>
        </div>
      `}

      <!-- All Templates -->
      <div>
        <h3 style="margin:0 0 16px;color:var(--text1);font-size:16px">
          ${selectedCategory === 'all' ? 'Tất cả Templates' : CATEGORIES.find(c => c.id === selectedCategory)?.name}
          <span style="color:var(--text2);font-weight:normal;font-size:14px"> (${filteredTemplates.length})</span>
        </h3>
        <div style="display:grid;grid-template-columns:repeat(auto-fill,minmax(320px,1fr));gap:16px">
          ${filteredTemplates.length === 0 ? html`
            <div style="grid-column:1/-1;text-align:center;padding:60px;color:var(--text2)">
              <div style="font-size:48px;margin-bottom:12px">🔍</div>
              <p>Không tìm thấy template phù hợp</p>
            </div>
          ` : filteredTemplates.map(t => TemplateCard({ template: t, onSelect: setSelectedTemplate, onInstall: handleInstall, installing }))}
        </div>
      </div>

      <!-- Template Detail Modal -->
      ${selectedTemplate ? TemplateModal({ template: selectedTemplate, onClose: () => setSelectedTemplate(null), onInstall: handleInstall, installing }) : null}
    </div>
  `;
}

function TemplateCard({ template, onSelect, onInstall, installing }) {
  return html`
    <div 
      class="card" 
      style="cursor:pointer;transition:transform 0.2s;position:relative;overflow:hidden"
      onClick=${() => onSelect(template)}
      onMouseEnter=${e => e.currentTarget.style.transform = 'translateY(-2px)'}
      onMouseLeave=${e => e.currentTarget.style.transform = 'translateY(0)'}
    >
      ${template.featured ? html`
        <div style="position:absolute;top:8px;right:8px;background:var(--accent);color:#fff;padding:2px 8px;border-radius:4px;font-size:11px;font-weight:600">
          ⭐ Featured
        </div>
      ` : null}
      
      <div style="display:flex;align-items:center;gap:12px;margin-bottom:12px">
        <div style="width:48px;height:48px;background:var(--bg2);border-radius:12px;display:flex;align-items:center;justify-content:center;font-size:24px">
          ${template.icon}
        </div>
        <div>
          <h4 style="margin:0;font-size:15px;color:var(--text1)">${template.name}</h4>
          <div style="display:flex;align-items:center;gap:8px;margin-top:2px">
            <span style="font-size:11px;color:var(--text2)">⏱️ ${template.setup}</span>
            <span style="font-size:11px;padding:2px 6px;border-radius:4px;background:${template.difficulty === 'easy' ? 'var(--green)' : 'var(--orange)'};color:#fff">
              ${template.difficulty === 'easy' ? 'Dễ' : 'Trung bình'}
            </span>
          </div>
        </div>
      </div>
      
      <p style="margin:0 0 12px;font-size:13px;color:var(--text2);line-height:1.5">
        ${template.description}
      </p>
      
      <div style="display:flex;flex-wrap:wrap;gap:4px;margin-bottom:12px">
        ${template.tags.slice(0, 3).map(tag => html`
          <span style="font-size:11px;padding:2px 8px;background:var(--bg2);border-radius:4px;color:var(--text2)">
            ${tag}
          </span>
        `)}
        ${template.tags.length > 3 ? html`<span style="font-size:11px;color:var(--text2)">+${template.tags.length - 3}</span>` : null}
      </div>

      <div style="display:flex;gap:8px;margin-top:auto;padding-top:12px;border-top:1px solid var(--border)">
        <button 
          class="btn-secondary" 
          onClick=${e => { e.stopPropagation(); onSelect(template); }}
          style="flex:1;padding:8px"
        >
          Xem chi tiết
        </button>
        <button 
          class="btn-primary" 
          onClick=${e => { e.stopPropagation(); onInstall(template); }}
          disabled=${installing === template.id}
          style="flex:1;padding:8px"
        >
          ${installing === template.id ? 'Đang cài...' : 'Cài đặt'}
        </button>
      </div>
    </div>
  `;
}

function TemplateModal({ template, onClose, onInstall, installing }) {
  return html`
    <div 
      class="modal-overlay" 
      onClick=${onClose}
      style="
        position:fixed;inset:0;background:rgba(0,0,0,0.6);z-index:1000;
        display:flex;align-items:center;justify-content:center;padding:20px
      "
    >
      <div 
        class="modal-content" 
        onClick=${e => e.stopPropagation()}
        style="
          background:var(--surface);border-radius:16px;max-width:700px;width:100%;
          max-height:85vh;overflow-y:auto;padding:24px
        "
      >
        <div style="display:flex;justify-content:space-between;align-items:flex-start;margin-bottom:20px">
          <div style="display:flex;align-items:center;gap:12px">
            <div style="width:56px;height:56px;background:var(--bg2);border-radius:12px;display:flex;align-items:center;justify-content:center;font-size:28px">
              ${template.icon}
            </div>
            <div>
              <h3 style="margin:0;color:var(--text1)">${template.name}</h3>
              <div style="display:flex;gap:8px;margin-top:4px">
                ${template.tags.map(tag => html`
                  <span style="font-size:11px;padding:2px 8px;background:var(--bg2);border-radius:4px;color:var(--text2)">${tag}</span>
                `)}
              </div>
            </div>
          </div>
          <button onClick=${onClose} style="background:none;border:none;font-size:24px;cursor:pointer;color:var(--text2)">✕</button>
        </div>

        <p style="color:var(--text2);line-height:1.6;margin-bottom:24px">${template.description}</p>

        <!-- Setup Info -->
        <div style="display:grid;grid-template-columns:repeat(3,1fr);gap:12px;margin-bottom:24px">
          <div style="background:var(--bg2);padding:12px;border-radius:8px;text-align:center">
            <div style="font-size:20px;margin-bottom:4px">⏱️</div>
            <div style="font-size:11px;color:var(--text2)">Thời gian setup</div>
            <div style="font-weight:600;color:var(--text1)">${template.setup}</div>
          </div>
          <div style="background:var(--bg2);padding:12px;border-radius:8px;text-align:center">
            <div style="font-size:20px;margin-bottom:4px">📊</div>
            <div style="font-size:11px;color:var(--text2)">Độ khó</div>
            <div style="font-weight:600;color:var(--text1)">${template.difficulty === 'easy' ? 'Dễ' : 'Trung bình'}</div>
          </div>
          <div style="background:var(--bg2);padding:12px;border-radius:8px;text-align:center">
            <div style="font-size:20px;margin-bottom:4px">🤖</div>
            <div style="font-size:11px;color:var(--text2)">Số Agent</div>
            <div style="font-weight:600;color:var(--text1)">${template.agents.length}</div>
          </div>
        </div>

        <!-- Agents -->
        <h4 style="margin:0 0 12px;color:var(--text1)">🤖 Agents</h4>
        <div style="margin-bottom:24px">
          ${template.agents.map((agent, i) => html`
            <div style="display:flex;align-items:center;gap:12px;padding:12px;background:var(--bg2);border-radius:8px;margin-bottom:8px">
              <div style="width:32px;height:32px;background:var(--accent);border-radius:50%;display:flex;align-items:center;justify-content:center;color:#fff;font-size:12px;font-weight:600">
                ${i + 1}
              </div>
              <div style="flex:1">
                <div style="font-weight:600;color:var(--text1)">${agent.name}</div>
                <div style="font-size:12px;color:var(--text2)">${agent.role} • ${agent.model}</div>
              </div>
              <span style="font-size:11px;padding:2px 8px;background:var(--accent);color:#fff;border-radius:4px">${agent.role}</span>
            </div>
          `)}
        </div>

        <!-- Channels -->
        <h4 style="margin:0 0 12px;color:var(--text1)">📱 Channels</h4>
        <div style="display:flex;gap:8px;margin-bottom:24px">
          ${template.channels.map(ch => html`
            <span style="font-size:12px;padding:4px 12px;background:var(--bg2);border-radius:4px;color:var(--text1)">${ch}</span>
          `)}
        </div>

        <!-- Tools -->
        <h4 style="margin:0 0 12px;color:var(--text1)">🛠️ Tools</h4>
        <div style="display:flex;flex-wrap:wrap;gap:6px;margin-bottom:24px">
          ${template.tools.map(tool => html`
            <span style="font-size:11px;padding:4px 10px;background:var(--bg2);border-radius:4px;color:var(--text2)">${tool}</span>
          `)}
        </div>

        <!-- Actions -->
        <div style="display:flex;gap:12px;padding-top:20px;border-top:1px solid var(--border)">
          <button class="btn-secondary" onClick=${onClose} style="flex:1;padding:12px">
            Đóng
          </button>
          <button 
            class="btn-primary" 
            onClick=${() => { onInstall(template); onClose(); }}
            disabled=${installing === template.id}
            style="flex:1;padding:12px"
          >
            ${installing === template.id ? 'Đang cài đặt...' : 'Cài đặt Template'}
          </button>
        </div>
      </div>
    </div>
  `;
}
