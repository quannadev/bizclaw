import { useState, useEffect } from 'preact/hooks'
import { invoke } from '@tauri-apps/api/core'

interface Settings {
  theme: string
  language: string
  provider: string
  model: string
  max_tokens: number
  temperature: number
}

export function SettingsView() {
  const [settings, setSettings] = useState<Settings>({
    theme: 'dark',
    language: 'en',
    provider: 'openai',
    model: 'gpt-4',
    max_tokens: 4000,
    temperature: 0.7
  })

  useEffect(() => {
    loadSettings()
  }, [])

  const loadSettings = async () => {
    try {
      const s = await invoke('get_settings') as Settings
      setSettings(s)
    } catch (error) {
      console.error('Failed to load settings:', error)
    }
  }

  const updateSetting = async (key: keyof Settings, value: string | number) => {
    const updated = { ...settings, [key]: value }
    setSettings(updated)
    try {
      await invoke('update_settings', { settings: updated })
    } catch (error) {
      console.error('Failed to update settings:', error)
    }
  }

  return (
    <div class="settings-view">
      <h2>Settings</h2>

      <section class="settings-section">
        <h3>Provider Configuration</h3>
        
        <div class="setting-item">
          <label>Primary Provider</label>
          <select 
            value={settings.provider}
            onChange={(e) => updateSetting('provider', (e.target as HTMLSelectElement).value)}
          >
            <option value="openai">OpenAI</option>
            <option value="anthropic">Anthropic</option>
            <option value="gemini">Google Gemini</option>
            <option value="deepseek">DeepSeek</option>
            <option value="ollama">Ollama (Local)</option>
          </select>
        </div>

        <div class="setting-item">
          <label>Model</label>
          <select 
            value={settings.model}
            onChange={(e) => updateSetting('model', (e.target as HTMLSelectElement).value)}
          >
            <option value="gpt-4">GPT-4</option>
            <option value="gpt-4-turbo">GPT-4 Turbo</option>
            <option value="gpt-3.5-turbo">GPT-3.5 Turbo</option>
          </select>
        </div>

        <div class="setting-item">
          <label>Temperature: {settings.temperature}</label>
          <input 
            type="range" 
            min="0" 
            max="2" 
            step="0.1"
            value={settings.temperature}
            onInput={(e) => updateSetting('temperature', parseFloat((e.target as HTMLInputElement).value))}
          />
        </div>

        <div class="setting-item">
          <label>Max Tokens</label>
          <input 
            type="number" 
            value={settings.max_tokens}
            onChange={(e) => updateSetting('max_tokens', parseInt((e.target as HTMLInputElement).value))}
          />
        </div>
      </section>

      <section class="settings-section">
        <h3>Appearance</h3>
        
        <div class="setting-item">
          <label>Theme</label>
          <select 
            value={settings.theme}
            onChange={(e) => updateSetting('theme', (e.target as HTMLSelectElement).value)}
          >
            <option value="dark">Dark</option>
            <option value="light">Light</option>
            <option value="system">System</option>
          </select>
        </div>

        <div class="setting-item">
          <label>Language</label>
          <select 
            value={settings.language}
            onChange={(e) => updateSetting('language', (e.target as HTMLSelectElement).value)}
          >
            <option value="en">English</option>
            <option value="vi">Tiếng Việt</option>
            <option value="zh">中文</option>
            <option value="ja">日本語</option>
          </select>
        </div>
      </section>
    </div>
  )
}
