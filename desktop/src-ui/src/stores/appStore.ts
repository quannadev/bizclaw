import { create } from 'zustand'
import { invoke } from '@tauri-apps/api/core'

interface Message {
  id: string
  role: 'user' | 'assistant'
  content: string
  timestamp: number
}

interface Conversation {
  id: string
  title: string
  messages: Message[]
  createdAt: number
  updatedAt: number
}

interface Channel {
  id: string
  name: string
  type: string
  connected: boolean
}

interface Skill {
  id: string
  name: string
  description: string
  installed: boolean
}

interface AppState {
  status: 'connecting' | 'connected' | 'disconnected' | 'error'
  conversations: Conversation[]
  currentConversation: Conversation | null
  channels: Channel[]
  skills: Skill[]
  
  connect: () => Promise<void>
  disconnect: () => Promise<void>
  sendMessage: (content: string) => Promise<void>
  createConversation: () => Promise<void>
  selectConversation: (id: string) => void
  deleteConversation: (id: string) => Promise<void>
  loadChannels: () => Promise<void>
  connectChannel: (id: string) => Promise<void>
  disconnectChannel: (id: string) => Promise<void>
  loadSkills: () => Promise<void>
  installSkill: (id: string) => Promise<void>
  uninstallSkill: (id: string) => Promise<void>
}

export const useStore = create<AppState>((set, get) => ({
  status: 'disconnected',
  conversations: [],
  currentConversation: null,
  channels: [],
  skills: [],

  connect: async () => {
    set({ status: 'connecting' })
    try {
      const status = await invoke('get_status')
      set({ status: 'connected' })
      await get().loadChannels()
      await get().loadSkills()
    } catch (error) {
      console.error('Failed to connect:', error)
      set({ status: 'error' })
    }
  },

  disconnect: async () => {
    set({ status: 'disconnected' })
  },

  sendMessage: async (content: string) => {
    const { currentConversation, conversations } = get()
    
    if (!currentConversation) {
      await get().createConversation()
    }
    
    const conv = get().currentConversation
    if (!conv) return

    const userMessage: Message = {
      id: crypto.randomUUID(),
      role: 'user',
      content,
      timestamp: Date.now()
    }

    set({
      currentConversation: {
        ...conv,
        messages: [...conv.messages, userMessage],
        updatedAt: Date.now()
      }
    })

    try {
      const response = await invoke('send_message', {
        conversationId: conv.id,
        message: content
      }) as string

      const assistantMessage: Message = {
        id: crypto.randomUUID(),
        role: 'assistant',
        content: response,
        timestamp: Date.now()
      }

      const updatedConv = get().currentConversation
      if (updatedConv) {
        set({
          currentConversation: {
            ...updatedConv,
            messages: [...updatedConv.messages, assistantMessage]
          }
        })
      }
    } catch (error) {
      console.error('Failed to send message:', error)
    }
  },

  createConversation: async () => {
    const newConv: Conversation = {
      id: crypto.randomUUID(),
      title: 'New Chat',
      messages: [],
      createdAt: Date.now(),
      updatedAt: Date.now()
    }
    
    set({
      conversations: [newConv, ...get().conversations],
      currentConversation: newConv
    })
  },

  selectConversation: (id: string) => {
    const conv = get().conversations.find(c => c.id === id)
    if (conv) {
      set({ currentConversation: conv })
    }
  },

  deleteConversation: async (id: string) => {
    try {
      await invoke('clear_conversation', { conversationId: id })
      const conversations = get().conversations.filter(c => c.id !== id)
      set({ conversations })
      if (get().currentConversation?.id === id) {
        set({ currentConversation: conversations[0] || null })
      }
    } catch (error) {
      console.error('Failed to delete conversation:', error)
    }
  },

  loadChannels: async () => {
    try {
      const channels = await invoke('get_channels') as Channel[]
      set({ channels })
    } catch (error) {
      console.error('Failed to load channels:', error)
    }
  },

  connectChannel: async (id: string) => {
    try {
      await invoke('connect_channel', { channelId: id })
      const channels = get().channels.map(c => 
        c.id === id ? { ...c, connected: true } : c
      )
      set({ channels })
    } catch (error) {
      console.error('Failed to connect channel:', error)
    }
  },

  disconnectChannel: async (id: string) => {
    try {
      await invoke('disconnect_channel', { channelId: id })
      const channels = get().channels.map(c => 
        c.id === id ? { ...c, connected: false } : c
      )
      set({ channels })
    } catch (error) {
      console.error('Failed to disconnect channel:', error)
    }
  },

  loadSkills: async () => {
    try {
      const skills = await invoke('get_skills') as Skill[]
      set({ skills })
    } catch (error) {
      console.error('Failed to load skills:', error)
    }
  },

  installSkill: async (id: string) => {
    try {
      await invoke('install_skill', { skillId: id })
      const skills = get().skills.map(s => 
        s.id === id ? { ...s, installed: true } : s
      )
      set({ skills })
    } catch (error) {
      console.error('Failed to install skill:', error)
    }
  },

  uninstallSkill: async (id: string) => {
    try {
      await invoke('uninstall_skill', { skillId: id })
      const skills = get().skills.map(s => 
        s.id === id ? { ...s, installed: false } : s
      )
      set({ skills })
    } catch (error) {
      console.error('Failed to uninstall skill:', error)
    }
  }
}))
