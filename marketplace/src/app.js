const API_BASE = 'http://localhost:18789/api/marketplace';

let currentPage = 1;
let currentFilters = {
  category: '',
  sort: 'popular',
  query: ''
};

document.addEventListener('DOMContentLoaded', () => {
  loadFeatured();
  loadCategories();
  loadAllSkills();
  setupEventListeners();
});

function setupEventListeners() {
  document.getElementById('searchInput').addEventListener('input', debounce((e) => {
    currentFilters.query = e.target.value;
    loadAllSkills();
  }, 300));

  document.getElementById('categoryFilter').addEventListener('change', (e) => {
    currentFilters.category = e.target.value;
    loadAllSkills();
  });

  document.getElementById('sortFilter').addEventListener('change', (e) => {
    currentFilters.sort = e.target.value;
    loadAllSkills();
  });

  document.getElementById('prevPage').addEventListener('click', () => {
    if (currentPage > 1) {
      currentPage--;
      loadAllSkills();
    }
  });

  document.getElementById('nextPage').addEventListener('click', () => {
    currentPage++;
    loadAllSkills();
  });

  document.querySelectorAll('.tag').forEach(tag => {
    tag.addEventListener('click', () => {
      document.querySelectorAll('.tag').forEach(t => t.classList.remove('active'));
      tag.classList.add('active');
    });
  });

  document.getElementById('loginBtn').addEventListener('click', () => {
    alert('Login functionality coming soon!');
  });

  document.getElementById('signupBtn').addEventListener('click', () => {
    alert('Sign up functionality coming soon!');
  });
}

async function loadFeatured() {
  try {
    const response = await fetch(`${API_BASE}/featured`);
    const data = await response.json();
    
    if (data.success) {
      renderTrendingSkills(data.data.trending);
    }
  } catch (error) {
    console.error('Failed to load featured:', error);
    renderTrendingSkills(getMockTrending());
  }
}

async function loadCategories() {
  try {
    const response = await fetch(`${API_BASE}/categories`);
    const data = await response.json();
    
    if (data.success) {
      renderCategories(data.data);
    }
  } catch (error) {
    console.error('Failed to load categories:', error);
    renderCategories(getMockCategories());
  }
}

async function loadAllSkills() {
  try {
    let url = `${API_BASE}/skills?page=${currentPage}&per_page=12`;
    if (currentFilters.category) url += `&category=${currentFilters.category}`;
    if (currentFilters.query) url += `&q=${encodeURIComponent(currentFilters.query)}`;
    if (currentFilters.sort) url += `&sort=${currentFilters.sort}`;
    
    const response = await fetch(url);
    const data = await response.json();
    
    if (data.success) {
      renderAllSkills(data.data.skills);
      updatePagination(data.data);
    }
  } catch (error) {
    console.error('Failed to load skills:', error);
    renderAllSkills(getMockSkills());
    updatePagination({ page: 1, total_pages: 3 });
  }
}

function renderTrendingSkills(skills) {
  const container = document.getElementById('trendingSkills');
  container.innerHTML = skills.map(skill => createSkillCard(skill)).join('');
  addSkillCardListeners(container);
}

function renderCategories(categories) {
  const container = document.getElementById('categoryGrid');
  container.innerHTML = categories.map(cat => `
    <div class="category-card" data-category="${cat.id}">
      <div class="category-icon">${cat.icon}</div>
      <h3>${cat.name}</h3>
      <p>${cat.description}</p>
      <span class="category-count">${cat.skill_count} skills</span>
    </div>
  `).join('');
  
  container.querySelectorAll('.category-card').forEach(card => {
    card.addEventListener('click', () => {
      document.getElementById('categoryFilter').value = card.dataset.category;
      currentFilters.category = card.dataset.category;
      loadAllSkills();
    });
  });
}

function renderAllSkills(skills) {
  const container = document.getElementById('allSkills');
  container.innerHTML = skills.map(skill => createSkillCard(skill)).join('');
  addSkillCardListeners(container);
}

function createSkillCard(skill) {
  return `
    <div class="skill-card" data-skill="${skill.slug || skill.id}">
      <div class="skill-header">
        <span class="skill-icon">${skill.icon || '🛠️'}</span>
        <div class="skill-info">
          <h3>${skill.name}</h3>
          <span class="skill-author">
            by ${skill.author_name || skill.author?.name || 'Unknown'}
            ${skill.author_verified || skill.author?.verified ? '<span class="verified">✓</span>' : ''}
          </span>
        </div>
      </div>
      <p class="skill-description">${skill.description}</p>
      <div class="skill-meta">
        <div class="skill-stats">
          <span class="skill-stat">⬇️ ${skill.downloads?.toLocaleString() || 0}</span>
          <span class="skill-stat">⭐ ${skill.rating?.toFixed(1) || '0.0'}</span>
        </div>
        <span class="skill-category">${skill.category || 'general'}</span>
      </div>
    </div>
  `;
}

function addSkillCardListeners(container) {
  container.querySelectorAll('.skill-card').forEach(card => {
    card.addEventListener('click', () => {
      openSkillModal(card.dataset.skill);
    });
  });
}

async function openSkillModal(slug) {
  const modal = document.getElementById('skillModal');
  const detailContainer = document.getElementById('skillDetail');
  
  try {
    const response = await fetch(`${API_BASE}/skills/${slug}`);
    const data = await response.json();
    
    if (data.success) {
      detailContainer.innerHTML = renderSkillDetail(data.data);
    }
  } catch (error) {
    console.error('Failed to load skill detail:', error);
    detailContainer.innerHTML = renderSkillDetail(getMockSkillDetail(slug));
  }
  
  modal.classList.add('show');
  
  modal.querySelector('.modal-close').onclick = () => modal.classList.remove('show');
  modal.onclick = (e) => { if (e.target === modal) modal.classList.remove('show'); };
}

function renderSkillDetail(skill) {
  return `
    <div class="skill-detail-header">
      <span class="skill-detail-icon">${skill.icon || '🛠️'}</span>
      <div class="skill-detail-title">
        <h1>${skill.name}</h1>
        <div class="skill-detail-meta">
          <span>by ${skill.author?.name || skill.author_name}</span>
          ${skill.verified ? '<span class="verified">✓ Verified</span>' : ''}
        </div>
        <div class="skill-detail-stats">
          <span>⬇️ ${skill.downloads?.toLocaleString()}</span>
          <span>⭐ ${skill.rating} (${skill.rating_count} reviews)</span>
          <span>v${skill.version}</span>
        </div>
      </div>
    </div>
    <p>${skill.description}</p>
    <div class="skill-actions">
      <button class="install-btn" onclick="installSkill('${skill.slug}')">Install Skill</button>
    </div>
    <div class="skill-readme">${skill.readme || skill.description}</div>
    <div class="reviews-section">
      <h3>Reviews (${skill.review_count || 0})</h3>
      <div id="reviewsList">
        <p>Loading reviews...</p>
      </div>
    </div>
  `;
}

async function installSkill(slug) {
  const btn = document.querySelector('.install-btn');
  btn.textContent = 'Installing...';
  btn.disabled = true;
  
  try {
    const response = await fetch(`${API_BASE}/skills/${slug}/install`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({})
    });
    const data = await response.json();
    
    if (data.success) {
      btn.textContent = '✓ Installed';
      btn.classList.add('installed');
    } else {
      btn.textContent = 'Install Failed';
      btn.disabled = false;
    }
  } catch (error) {
    console.error('Install failed:', error);
    btn.textContent = 'Install Failed';
    btn.disabled = false;
  }
}

function updatePagination(data) {
  document.getElementById('pageInfo').textContent = `Page ${data.page} of ${data.total_pages}`;
  document.getElementById('prevPage').disabled = data.page <= 1;
  document.getElementById('nextPage').disabled = data.page >= data.total_pages;
  currentPage = data.page;
}

function debounce(func, wait) {
  let timeout;
  return function executedFunction(...args) {
    const later = () => {
      clearTimeout(timeout);
      func(...args);
    };
    clearTimeout(timeout);
    timeout = setTimeout(later, wait);
  };
}

function getMockTrending() {
  return [
    { id: 'web-developer', name: 'Web Developer', slug: 'web-developer', icon: '💻', description: 'Web development assistance, code review, debugging', author_name: 'BizClaw', author_verified: true, downloads: 1500, rating: 4.8, category: 'developer' },
    { id: 'python-analyst', name: 'Python Analyst', slug: 'python-analyst', icon: '🐍', description: 'Python data analysis, ML, automation', author_name: 'BizClaw', author_verified: true, downloads: 1200, rating: 4.7, category: 'data' },
    { id: 'vietnamese-business', name: 'Vietnamese Business', slug: 'vietnamese-business', icon: '🇻🇳', description: 'Vietnamese business writing and communication', author_name: 'BizClaw', author_verified: true, downloads: 800, rating: 4.9, category: 'business' },
  ];
}

function getMockCategories() {
  return [
    { id: 'developer', name: 'Developer', icon: '💻', description: 'Programming and development', skill_count: 25 },
    { id: 'business', name: 'Business', icon: '💼', description: 'Business writing and communication', skill_count: 15 },
    { id: 'data', name: 'Data', icon: '📊', description: 'Data analysis and processing', skill_count: 12 },
    { id: 'automation', name: 'Automation', icon: '⚡', description: 'Workflow and process automation', skill_count: 8 },
  ];
}

function getMockSkills() {
  return [
    ...getMockTrending(),
    { id: 'rust-expert', name: 'Rust Expert', slug: 'rust-expert', icon: '🦀', description: 'Rust best practices and optimization', author_name: 'BizClaw', author_verified: true, downloads: 600, rating: 4.6, category: 'developer' },
    { id: 'content-writer', name: 'Content Writer', slug: 'content-writer', icon: '✍️', description: 'Marketing copy and blog posts', author_name: 'BizClaw', author_verified: true, downloads: 500, rating: 4.5, category: 'creative' },
  ];
}

function getMockSkillDetail(slug) {
  return {
    name: 'Web Developer',
    slug,
    icon: '💻',
    description: 'Web development assistance, code review, debugging. This skill helps you with HTML, CSS, JavaScript, React, Vue, and more.',
    version: '1.0.0',
    author: { name: 'BizClaw', verified: true },
    downloads: 1500,
    rating: 4.8,
    rating_count: 120,
    verified: true,
    readme: '# Web Developer Skill\n\nA comprehensive skill for web development...',
    review_count: 95,
  };
}
