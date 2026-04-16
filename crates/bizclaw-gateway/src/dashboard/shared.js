// BizClaw Dashboard — Shared utilities for page modules
// Exports: authFetch, authHeaders, t (i18n), StatsCard, Toast

const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;

import { vi } from '/static/dashboard/i18n/vi.js';
import { en } from '/static/dashboard/i18n/en.js';

const I18N = { vi, en };

// ═══ I18N ═══
export function t(key, lang) {
  return (I18N[lang] || I18N.vi)[key] || I18N.vi[key] || key;
}

// ═══ AUTH HELPERS ═══
// JWT token management
function getJwtToken() {
  const url = new URL(location.href);
  const tokenParam = url.searchParams.get('token');
  if (tokenParam) {
    sessionStorage.setItem('bizclaw_jwt', tokenParam);
    url.searchParams.delete('token');
    history.replaceState(null, '', url.pathname + url.search + url.hash);
    return tokenParam;
  }
  const match = document.cookie.match(/bizclaw_token=([^;]+)/);
  if (match) return match[1];
  return sessionStorage.getItem('bizclaw_jwt') || '';
}

let jwtToken = getJwtToken();

export function authHeaders(extra = {}) {
  return { ...extra, 'Authorization': 'Bearer ' + jwtToken, 'Content-Type': 'application/json' };
}

export async function authFetch(url, opts = {}) {
  if (!opts.headers) opts.headers = {};
  if (jwtToken) {
    opts.headers['Authorization'] = 'Bearer ' + jwtToken;
  }
  const res = await fetch(url, opts);
  if (res.status === 401) {
    sessionStorage.removeItem('bizclaw_jwt');
    jwtToken = '';
    throw new Error('Unauthorized');
  }
  return res;
}

export function refreshJwtToken() {
  jwtToken = getJwtToken();
}

export function getToken() {
  return jwtToken;
}

export function setToken(newToken) {
  jwtToken = newToken;
}

// ═══ SHARED COMPONENTS ═══

export function Toast({ message, type }) {
  if (!message) return null;
  const colors = { error: 'var(--red)', success: 'var(--green)', info: 'var(--accent2)' };
  return html`<div class="toast" style="border-left: 3px solid ${colors[type] || colors.info}">
    ${message}
  </div>`;
}

export function StatsCard({ label, value, color = 'accent', sub, icon }) {
  return html`<div class="card stats-card">
    <div style="display:flex;align-items:center;gap:8px;margin-bottom:8px">
      ${icon ? html`<span style="font-size:1.3em">${icon}</span>` : null}
      <span class="stats-label">${label}</span>
    </div>
    <div class="stats-value" style="color:var(--${color})">${value}</div>
    ${sub ? html`<div style="font-size:0.75em;opacity:0.6;margin-top:4px">${sub}</div>` : null}
  </div>`;
}

// ═══ PAGE DEFINITIONS — Clean grouped sidebar for SME-friendly UX ═══
export const PAGES = [
  // ── Trang chủ ──
  { id: 'dashboard', icon: '📊', label: 'nav.dashboard' },

  // ── Vận hành (Operations) ──
  { id: 'sep_ops', sep: true, groupLabel: 'nav.group_ops' },
  { id: 'chat', icon: '💬', label: 'nav.webchat' },
  { id: 'handoff', icon: '🤝', label: 'nav.handoff' },
  { id: 'campaigns', icon: '📢', label: 'nav.campaigns' },
  { id: 'hands', icon: '🤚', label: 'nav.hands' },
  { id: 'workflows', icon: '🔄', label: 'nav.workflows' },
  { id: 'scheduler', icon: '⏰', label: 'nav.scheduler' },

  // ── Kinh doanh (Business) ──
  { id: 'sep_biz', sep: true, groupLabel: 'nav.group_biz' },
  { id: 'products', icon: '🛍️', label: 'nav.products' },
  { id: 'paymentlinks', icon: '💳', label: 'nav.paymentlinks' },
  { id: 'analytics', icon: '📈', label: 'nav.analytics' },

  // ── Trí tuệ (Intelligence) ──
  { id: 'sep_intel', sep: true, groupLabel: 'nav.group_intel' },
  { id: 'agents', icon: '🤖', label: 'nav.agents' },
  { id: 'knowledge', icon: '📚', label: 'nav.knowledge' },
  { id: 'wiki', icon: '📖', label: 'nav.wiki' },
  { id: 'gallery', icon: '📦', label: 'nav.gallery' },
  { id: 'plugins', icon: '🛒', label: 'nav.plugins' },

  // ── Cấu hình (Settings) ──
  { id: 'sep_cfg', sep: true, groupLabel: 'nav.group_cfg' },
  { id: 'channels', icon: '📱', label: 'nav.channels' },
  { id: 'providers', icon: '🔌', label: 'nav.providers' },
  { id: 'tools', icon: '🛠️', label: 'nav.tools' },
  { id: 'mcp', icon: '🔗', label: 'nav.mcp' },
  { id: 'settings', icon: '⚙️', label: 'nav.settings' },
];

// ═══ NEW HORIZONTAL MENU — Feature Groups (Full Vietnamese) ═══
export const PAGES_NEW = [
  // ── Trang chủ ──
  {
    id: 'home',
    label: 'Trang chủ',
    icon: '🏠',
    children: [
      { id: 'dashboard', icon: '📊', label: 'Bảng điều khiển' },
      { id: 'analytics', icon: '📈', label: 'Phân tích' },
      { id: 'activity', icon: '⚡', label: 'Hoạt động' },
    ]
  },
  // ── Nhân sự AI ──
  {
    id: 'ai-team',
    label: 'Nhân sự AI',
    icon: '🤖',
    children: [
      { id: 'agents', icon: '🤖', label: 'Đội nhóm' },
      { id: 'teams', icon: '👥', label: 'Nhóm Agent' },
      { id: 'skills', icon: '💼', label: 'Kỹ năng' },
      { id: 'brain', icon: '🧠', label: 'Bộ nhớ AI' },
      { id: 'gallery', icon: '📦', label: 'Mẫu Agent' },
    ]
  },
  // ── Kênh liên lạc ──
  {
    id: 'channels',
    label: 'Kênh liên lạc',
    icon: '📱',
    children: [
      { id: 'channels', icon: '📱', label: 'Tất cả kênh' },
      { id: 'webhooks', icon: '🪝', label: 'Webhook' },
      { id: 'mcp', icon: '🔗', label: 'Máy chủ MCP' },
    ]
  },
  // ── Tự động hóa ──
  {
    id: 'automation',
    label: 'Tự động hóa',
    icon: '🔄',
    children: [
      { id: 'workflows', icon: '🔄', label: 'Quy trình' },
      { id: 'scheduler', icon: '⏰', label: 'Lịch trình' },
      { id: 'hands', icon: '🤚', label: 'Tay Robot' },
    ]
  },
  // ── Vận hành ──
  {
    id: 'operations',
    label: 'Vận hành',
    icon: '💬',
    children: [
      { id: 'chat', icon: '💬', label: 'Trò chuyện' },
      { id: 'handoff', icon: '🤝', label: 'Chuyển giao' },
      { id: 'campaigns', icon: '📢', label: 'Chiến dịch' },
      { id: 'knowledge', icon: '📚', label: 'Tri thức' },
      { id: 'products', icon: '📦', label: 'Sản phẩm' },
      { id: 'tiktok', icon: '🎵', label: 'TikTok' },
      { id: 'shopee', icon: '🛒', label: 'Shopee' },
    ]
  },
  // ── Cấu hình ──
  {
    id: 'system',
    label: 'Cấu hình',
    icon: '🔧',
    children: [
      { id: 'settings', icon: '⚙️', label: 'Cài đặt' },
      { id: 'providers', icon: '🔌', label: 'Nhà cung cấp' },
      { id: 'tools', icon: '🛠️', label: 'Công cụ' },
      { id: 'apikeys', icon: '🔑', label: 'API Keys' },
      { id: 'plugins', icon: '🧩', label: 'Plugin' },
      { id: 'configfile', icon: '📝', label: 'Tệp cấu hình' },
    ]
  },
  // ── Trợ giúp ──
  {
    id: 'help',
    label: 'Trợ giúp',
    icon: '❓',
    children: [
      { id: 'wiki', icon: '📖', label: 'Tài liệu' },
      { id: 'usage', icon: '📊', label: 'Sử dụng' },
      { id: 'cost', icon: '💰', label: 'Chi phí' },
    ]
  },
];

// Make shared functions available globally (for backward compat with page modules)
window.authFetch = authFetch;
window.authHeaders = authHeaders;
window.t = t;

// ═══ SME MODE PAGES — Simplified navigation for non-technical users ═══
export const SME_PAGES = [
  { id: 'sme', icon: '🏠', label: 'nav.dashboard' },
  { id: 'agents', icon: '🤖', label: 'nav.agents' },
  { id: 'workflows', icon: '🔄', label: 'nav.workflows' },
  { id: 'analytics', icon: '📊', label: 'nav.analytics' },
  { id: 'settings', icon: '⚙️', label: 'nav.settings' },
];

window.SME_PAGES = SME_PAGES;
