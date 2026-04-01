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

// ═══ PAGE DEFINITIONS ═══
export const PAGES = [
  { id: 'dashboard', icon: '📊', label: 'nav.dashboard' },
  { id: 'chat', icon: '💬', label: 'nav.webchat' },
  { id: 'sep1', sep: true },
  // AI & Content
  { id: 'agents', icon: '🤖', label: 'nav.agents' },
  { id: 'knowledge', icon: '📚', label: 'nav.knowledge' },
  { id: 'channels', icon: '📱', label: 'nav.channels' },
  { id: 'settings', icon: '⚙️', label: 'nav.settings' },
  { id: 'providers', icon: '🔌', label: 'nav.providers' },
  { id: 'tools', icon: '🛠️', label: 'nav.tools' },
  { id: 'mcp', icon: '🔗', label: 'nav.mcp' },
  { id: 'wiki', icon: '📖', label: 'Wiki & Guide' },
  { id: 'products', icon: '🛍️', label: 'Sản Phẩm' },
  { id: 'handoff', icon: '🤝', label: 'Human Handoff' },
  { id: 'paymentlinks', icon: '💳', label: 'Thanh Toán QR' },
  { id: 'sep2', sep: true },
  // Automation
  { id: 'hands', icon: '🤚', label: 'Autonomous Hands' },
  { id: 'workflows', icon: '🔄', label: 'nav.workflows' },
  { id: 'orchestration', icon: '🔀', label: 'nav.orchestration' },
  { id: 'orgmap', icon: '🗺️', label: 'Org Map' },
  { id: 'kanban', icon: '📋', label: 'Kanban Board' },
  { id: 'gallery', icon: '📦', label: 'nav.gallery' },
  { id: 'scheduler', icon: '⏰', label: 'nav.scheduler' },
  { id: 'campaigns', icon: '📢', label: 'Broadcast' },
  { id: 'sep3', sep: true },
  // Monitoring & Analytics
  { id: 'traces', icon: '🔍', label: 'nav.traces' },
  { id: 'cost', icon: '💰', label: 'nav.cost' },
  { id: 'activity', icon: '⚡', label: 'activity.title' },
  { id: 'usage', icon: '📊', label: 'Usage & Quotas' },
  { id: 'analytics', icon: '📈', label: 'Analytics' },
  { id: 'sep4', sep: true },
  // Enterprise
  { id: 'plugins', icon: '🛒', label: 'Plugin Marketplace' },
  { id: 'sso', icon: '🔐', label: 'Enterprise SSO' },
  { id: 'finetuning', icon: '🧪', label: 'Fine-Tuning' },
  { id: 'edgegateway', icon: '📡', label: 'Edge IoT Gateway' },
  { id: 'sep5', sep: true },
  // System
  { id: 'apikeys', icon: '🔑', label: 'API Keys' },
  { id: 'configfile', icon: '📄', label: 'nav.config' },
];

// Make shared functions available globally (for backward compat with page modules)
window.authFetch = authFetch;
window.authHeaders = authHeaders;
window.t = t;
