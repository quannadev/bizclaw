import { useStore } from '../stores/appStore'

interface SidebarProps {
  currentView: 'chat' | 'settings' | 'channels' | 'skills'
  onViewChange: (view: 'chat' | 'settings' | 'channels' | 'skills') => void
}

export function Sidebar({ currentView, onViewChange }: SidebarProps) {
  const { conversations, currentConversation, selectConversation, createConversation, status } = useStore()

  return (
    <aside class="sidebar">
      <div class="sidebar-header">
        <h1 class="logo">🦞</h1>
        <span class="app-name">BizClaw</span>
      </div>

      <button class="new-chat-btn" onClick={() => createConversation()}>
        + New Chat
      </button>

      <nav class="nav-items">
        <button 
          class={`nav-item ${currentView === 'chat' ? 'active' : ''}`}
          onClick={() => onViewChange('chat')}
        >
          💬 Chat
        </button>
        <button 
          class={`nav-item ${currentView === 'channels' ? 'active' : ''}`}
          onClick={() => onViewChange('channels')}
        >
          📡 Channels
        </button>
        <button 
          class={`nav-item ${currentView === 'skills' ? 'active' : ''}`}
          onClick={() => onViewChange('skills')}
        >
          🧩 Skills
        </button>
        <button 
          class={`nav-item ${currentView === 'settings' ? 'active' : ''}`}
          onClick={() => onViewChange('settings')}
        >
          ⚙️ Settings
        </button>
      </nav>

      <div class="conversations-list">
        <h3>Recent</h3>
        {conversations.map(conv => (
          <button
            key={conv.id}
            class={`conversation-item ${currentConversation?.id === conv.id ? 'active' : ''}`}
            onClick={() => selectConversation(conv.id)}
          >
            {conv.title}
          </button>
        ))}
      </div>

      <div class="sidebar-footer">
        <div class={`status-indicator ${status}`}>
          {status === 'connected' ? '🟢 Connected' : 
           status === 'connecting' ? '🟡 Connecting...' :
           status === 'error' ? '🔴 Error' : '⚪ Disconnected'}
        </div>
      </div>
    </aside>
  )
}
