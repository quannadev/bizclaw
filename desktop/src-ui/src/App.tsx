import { useState, useEffect } from 'preact/hooks'
import { invoke } from '@tauri-apps/api/core'
import { useStore } from './stores/appStore'
import { Sidebar } from './components/Sidebar'
import { ChatView } from './components/ChatView'
import { SettingsView } from './components/SettingsView'
import { ChannelsView } from './components/ChannelsView'
import { SkillsView } from './components/SkillsView'

type View = 'chat' | 'settings' | 'channels' | 'skills'

export function App() {
  const [currentView, setCurrentView] = useState<View>('chat')
  const { status, connect } = useStore()

  useEffect(() => {
    connect()
  }, [])

  return (
    <div class="app">
      <Sidebar 
        currentView={currentView} 
        onViewChange={setCurrentView}
      />
      <main class="main-content">
        {currentView === 'chat' && <ChatView />}
        {currentView === 'settings' && <SettingsView />}
        {currentView === 'channels' && <ChannelsView />}
        {currentView === 'skills' && <SkillsView />}
      </main>
    </div>
  )
}
