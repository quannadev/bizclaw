import { useState } from 'preact/hooks'
import { useStore } from '../stores/appStore'

export function ChatView() {
  const [input, setInput] = useState('')
  const { currentConversation, sendMessage, status } = useStore()

  const handleSubmit = async (e: Event) => {
    e.preventDefault()
    if (!input.trim() || status !== 'connected') return
    
    await sendMessage(input)
    setInput('')
  }

  return (
    <div class="chat-view">
      <header class="chat-header">
        <h2>{currentConversation?.title || 'Select a conversation'}</h2>
      </header>

      <div class="messages">
        {currentConversation?.messages.length === 0 && (
          <div class="welcome-message">
            <h3>Welcome to BizClaw! 👋</h3>
            <p>I'm your AI assistant. How can I help you today?</p>
          </div>
        )}
        
        {currentConversation?.messages.map(msg => (
          <div key={msg.id} class={`message ${msg.role}`}>
            <div class="message-content">{msg.content}</div>
            <div class="message-time">
              {new Date(msg.timestamp).toLocaleTimeString()}
            </div>
          </div>
        ))}
      </div>

      <form class="chat-input" onSubmit={handleSubmit}>
        <input
          type="text"
          value={input}
          onInput={(e) => setInput((e.target as HTMLInputElement).value)}
          placeholder="Type your message..."
          disabled={status !== 'connected'}
        />
        <button type="submit" disabled={!input.trim() || status !== 'connected'}>
          ➤
        </button>
      </form>

      <div class="tools-bar">
        🔗 Tools: browser, file, search, memory
      </div>
    </div>
  )
}
