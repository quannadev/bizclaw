// ProductsPage — Product Catalog Manager for SME
// Structured product data → auto-sync to RAG for AI agents
const { h, html, useState, useEffect, useContext, useCallback, useRef, useMemo } = window;
import { t, authFetch, authHeaders, StatsCard } from '/static/dashboard/shared.js';

const CATEGORIES = ['Tất cả', 'Thời trang', 'Thực phẩm', 'Điện tử', 'Dịch vụ', 'Mỹ phẩm', 'Nội thất', 'Khác'];
const STATUS_MAP = { active: { label: 'Đang bán', badge: 'badge-green' }, draft: { label: 'Nháp', badge: 'badge-outline' }, outofstock: { label: 'Hết hàng', badge: 'badge-red' } };

function ProductsPage({ lang }) {
  const { showToast } = useContext(AppContext);
  const [products, setProducts] = useState([
    // Demo seed data
    { id: 'p1', name: 'Áo Khoác Gió Premium', sku: 'AKG-001', price: 450000, stock: 124, category: 'Thời trang', status: 'active', image: '', desc: 'Áo khoác gió chống nước, thoáng khí, freesize M-XL' },
    { id: 'p2', name: 'Túi Xách Da Bò Handmade', sku: 'TXD-015', price: 1200000, stock: 23, category: 'Thời trang', status: 'active', image: '', desc: 'Túi xách da bò thật 100%, khâu tay thủ công' },
    { id: 'p3', name: 'Set Skincare Hàn Quốc 5 Bước', sku: 'SKC-088', price: 890000, stock: 0, category: 'Mỹ phẩm', status: 'outofstock', image: '', desc: 'Bộ skincare 5 bước: tẩy trang, sữa rửa, toner, serum, kem dưỡng' },
    { id: 'p4', name: 'Combo Cà Phê Rang Xay 1kg', sku: 'CF-201', price: 280000, stock: 56, category: 'Thực phẩm', status: 'active', image: '', desc: 'Cà phê Arabica Đà Lạt rang medium, xay sẵn' },
  ]);
  const [showForm, setShowForm] = useState(false);
  const [editProduct, setEditProduct] = useState(null);
  const [form, setForm] = useState({ name: '', sku: '', price: 0, stock: 0, category: 'Khác', status: 'active', image: '', desc: '' });
  const [filterCat, setFilterCat] = useState('Tất cả');
  const [search, setSearch] = useState('');
  const [syncing, setSyncing] = useState(false);

  const filtered = useMemo(() => {
    let list = products;
    if (filterCat !== 'Tất cả') list = list.filter(p => p.category === filterCat);
    if (search.trim()) list = list.filter(p => p.name.toLowerCase().includes(search.toLowerCase()) || p.sku.toLowerCase().includes(search.toLowerCase()));
    return list;
  }, [products, filterCat, search]);

  const totalValue = useMemo(() => products.reduce((s, p) => s + p.price * p.stock, 0), [products]);
  const activeCount = products.filter(p => p.status === 'active').length;
  const oosCount = products.filter(p => p.status === 'outofstock' || p.stock === 0).length;

  const openCreate = () => {
    setEditProduct(null);
    setForm({ name: '', sku: '', price: 0, stock: 0, category: 'Khác', status: 'active', image: '', desc: '' });
    setShowForm(true);
  };
  const openEdit = (p) => {
    setEditProduct(p);
    setForm({ ...p });
    setShowForm(true);
  };
  const saveProduct = () => {
    if (!form.name.trim()) { showToast('⚠️ Nhập tên sản phẩm', 'error'); return; }
    if (editProduct) {
      setProducts(prev => prev.map(p => p.id === editProduct.id ? { ...form, id: editProduct.id } : p));
      showToast('✅ Đã cập nhật: ' + form.name, 'success');
    } else {
      const newId = 'p' + Date.now();
      setProducts(prev => [...prev, { ...form, id: newId }]);
      showToast('✅ Đã thêm: ' + form.name, 'success');
    }
    setShowForm(false);
  };
  const deleteProduct = (p) => {
    if (!confirm('Xoá "' + p.name + '"?')) return;
    setProducts(prev => prev.filter(x => x.id !== p.id));
    showToast('🗑️ Đã xoá: ' + p.name, 'success');
  };
  const syncToRag = async () => {
    setSyncing(true);
    // Build a markdown document from product catalog and push to RAG
    const content = products.filter(p => p.status === 'active').map(p =>
      `## ${p.name} (${p.sku})\n- Giá: ${p.price.toLocaleString('vi-VN')} VNĐ\n- Tồn kho: ${p.stock} cái\n- Phân loại: ${p.category}\n- Mô tả: ${p.desc}\n`
    ).join('\n');
    try {
      const r = await authFetch('/api/v1/knowledge/documents', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name: 'product_catalog_auto.md', content, source: 'product_catalog', owner: '', data_zone: 'transactional' })
      });
      const d = await r.json();
      if (d.ok) showToast('✅ Đã đồng bộ ' + products.filter(p => p.status === 'active').length + ' sản phẩm → RAG (' + d.chunks + ' chunks)', 'success');
      else showToast('❌ ' + (d.error || 'Lỗi'), 'error');
    } catch (e) { showToast('❌ ' + e.message, 'error'); }
    setSyncing(false);
  };

  const fmtPrice = (v) => v.toLocaleString('vi-VN') + ' ₫';
  const inp = 'width:100%;padding:8px;margin-top:4px;background:var(--bg2);border:1px solid var(--border);border-radius:6px;color:var(--text);font-size:13px';

  return html`<div>
    <div class="page-header"><div>
      <h1>🛍️ Quản Lý Sản Phẩm (Product Catalog)</h1>
      <div class="sub">Bảng giá sản phẩm cấu trúc — AI Agent sẽ tự tra giá và tồn kho khi khách hỏi</div>
    </div>
      <div style="display:flex;gap:8px">
        <button class="btn btn-outline" onClick=${syncToRag} disabled=${syncing} style="display:flex;align-items:center;gap:6px">
          ${syncing ? '⏳' : '🔄'} Đồng bộ → RAG
        </button>
        <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 18px" onClick=${openCreate}>+ Thêm Sản Phẩm</button>
      </div>
    </div>

    <div class="stats">
      <${StatsCard} label="Tổng Sản Phẩm" value=${products.length} color="accent" icon="📦" />
      <${StatsCard} label="Đang Bán" value=${activeCount} color="green" icon="✅" />
      <${StatsCard} label="Hết Hàng" value=${oosCount} color="red" icon="⚠️" />
      <${StatsCard} label="Giá Trị Kho" value=${fmtPrice(totalValue)} color="blue" icon="💰" />
    </div>

    <!-- Filters -->
    <div style="display:flex;gap:8px;margin-bottom:14px;flex-wrap:wrap;align-items:center">
      <input value=${search} onInput=${e => setSearch(e.target.value)} placeholder="🔍 Tìm tên / mã SKU..."
        style="padding:8px 14px;border-radius:8px;border:1px solid var(--border);background:var(--bg2);color:var(--text);font-size:13px;min-width:220px" />
      <div style="display:flex;gap:4px;flex-wrap:wrap">
        ${CATEGORIES.map(c => html`
          <button key=${c} class="btn btn-sm" onClick=${() => setFilterCat(c)}
            style="padding:4px 12px;border-radius:6px;font-size:11px;
              border:1px solid ${filterCat === c ? 'var(--accent)' : 'var(--border)'};
              background:${filterCat === c ? 'var(--accent)' : 'transparent'};
              color:${filterCat === c ? '#fff' : 'var(--text2)'}">${c}</button>
        `)}
      </div>
    </div>

    ${showForm && html`
      <div class="card" style="margin-bottom:14px;border:1px solid var(--accent)">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:12px">
          <h3>${editProduct ? '✏️ Sửa: ' + editProduct.name : '➕ Thêm Sản Phẩm Mới'}</h3>
          <button class="btn btn-outline btn-sm" onClick=${() => setShowForm(false)}>✕</button>
        </div>
        <div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:10px;font-size:13px">
          <label style="grid-column:span 2">Tên sản phẩm *<input style="${inp}" value=${form.name} onInput=${e => setForm(f => ({ ...f, name: e.target.value }))} placeholder="Áo Khoác Gió Premium" /></label>
          <label>Mã SKU<input style="${inp}" value=${form.sku} onInput=${e => setForm(f => ({ ...f, sku: e.target.value }))} placeholder="AKG-001" /></label>
          <label>Giá bán (VNĐ) *<input type="number" style="${inp}" value=${form.price} onInput=${e => setForm(f => ({ ...f, price: +e.target.value || 0 }))} /></label>
          <label>Tồn kho<input type="number" style="${inp}" value=${form.stock} onInput=${e => setForm(f => ({ ...f, stock: +e.target.value || 0 }))} /></label>
          <label>Phân loại
            <select style="${inp}" value=${form.category} onChange=${e => setForm(f => ({ ...f, category: e.target.value }))}>
              ${CATEGORIES.filter(c => c !== 'Tất cả').map(c => html`<option key=${c} value=${c}>${c}</option>`)}
            </select>
          </label>
          <label>Trạng thái
            <select style="${inp}" value=${form.status} onChange=${e => setForm(f => ({ ...f, status: e.target.value }))}>
              <option value="active">✅ Đang bán</option>
              <option value="draft">📝 Nháp</option>
              <option value="outofstock">⚠️ Hết hàng</option>
            </select>
          </label>
          <label>URL Ảnh sản phẩm<input style="${inp}" value=${form.image} onInput=${e => setForm(f => ({ ...f, image: e.target.value }))} placeholder="https://..." /></label>
          <label style="grid-column:span 3">Mô tả ngắn (AI sẽ đọc mô tả này để trả lời khách)<input style="${inp}" value=${form.desc} onInput=${e => setForm(f => ({ ...f, desc: e.target.value }))} placeholder="Mô tả chi tiết sản phẩm..." /></label>
        </div>
        <div style="margin-top:14px;display:flex;gap:8px;justify-content:flex-end">
          <button class="btn btn-outline" onClick=${() => setShowForm(false)}>Huỷ</button>
          <button class="btn" style="background:var(--grad1);color:#fff;padding:8px 20px" onClick=${saveProduct}>💾 ${editProduct ? 'Cập nhật' : 'Thêm'}</button>
        </div>
      </div>
    `}

    <!-- Product Table -->
    <div class="card">
      ${filtered.length === 0 ? html`
        <div style="text-align:center;padding:40px;color:var(--text2)">
          <div style="font-size:48px;margin-bottom:12px">🛍️</div>
          <p>Chưa có sản phẩm${filterCat !== 'Tất cả' ? ' trong mục "' + filterCat + '"' : ''}. Bấm <strong>+ Thêm Sản Phẩm</strong> để bắt đầu!</p>
        </div>
      ` : html`
        <table><thead><tr>
          <th style="width:35%">Sản phẩm</th>
          <th>SKU</th>
          <th style="text-align:right">Giá</th>
          <th style="text-align:center">Kho</th>
          <th>Phân loại</th>
          <th>Trạng thái</th>
          <th style="text-align:right">Thao tác</th>
        </tr></thead><tbody>
          ${filtered.map(p => html`<tr key=${p.id} style="cursor:pointer" onClick=${() => openEdit(p)}>
            <td>
              <div style="display:flex;align-items:center;gap:10px">
                <div style="width:40px;height:40px;border-radius:8px;background:var(--bg2);border:1px solid var(--border);display:flex;align-items:center;justify-content:center;font-size:20px;flex-shrink:0">
                  ${p.image ? html`<img src=${p.image} style="width:100%;height:100%;object-fit:cover;border-radius:8px" />` : '📦'}
                </div>
                <div>
                  <strong style="font-size:13px">${p.name}</strong>
                  <div style="font-size:11px;color:var(--text2);max-width:250px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${p.desc}</div>
                </div>
              </div>
            </td>
            <td style="font-family:var(--mono);font-size:12px;color:var(--accent)">${p.sku}</td>
            <td style="text-align:right;font-weight:700;font-size:13px">${fmtPrice(p.price)}</td>
            <td style="text-align:center">
              <span style="font-weight:600;color:${p.stock === 0 ? 'var(--red)' : p.stock < 10 ? '#f59e0b' : 'var(--green)'}">${p.stock}</span>
            </td>
            <td><span class="badge" style="font-size:10px">${p.category}</span></td>
            <td><span class="badge ${STATUS_MAP[p.status]?.badge || ''}" style="font-size:10px">${STATUS_MAP[p.status]?.label || p.status}</span></td>
            <td style="text-align:right" onClick=${e => e.stopPropagation()}>
              <button class="btn btn-outline btn-sm" onClick=${() => openEdit(p)} title="Sửa">✏️</button>
              <button class="btn btn-outline btn-sm" style="color:var(--red);margin-left:4px" onClick=${() => deleteProduct(p)} title="Xoá">🗑️</button>
            </td>
          </tr>`)}
        </tbody></table>
      `}
    </div>

    <!-- RAG Sync Info -->
    <div style="margin-top:14px;padding:14px 18px;background:var(--bg2);border-radius:10px;border:1px solid var(--border);display:flex;align-items:center;gap:12px;font-size:12px">
      <span style="font-size:20px">💡</span>
      <div style="flex:1;color:var(--text2)">
        <strong style="color:var(--text)">Auto-Sync RAG:</strong> Bấm "🔄 Đồng bộ → RAG" để cập nhật bảng giá vào Knowledge Base.
        AI Agent sẽ tự động tra giá + tồn kho khi khách hỏi qua Zalo/Messenger/Telegram.
      </div>
    </div>
  </div>`;
}

export { ProductsPage };
