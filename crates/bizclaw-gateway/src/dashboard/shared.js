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

// Make shared functions available globally (for backward compat with page modules)
window.authFetch = authFetch;
window.authHeaders = authHeaders;
window.t = t;
