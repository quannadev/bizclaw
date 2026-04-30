import { useStore } from '../stores/appStore'

export function ChannelsView() {
  const { channels, connectChannel, disconnectChannel } = useStore()

  return (
    <div class="channels-view">
      <h2>Channels</h2>
      <p class="subtitle">Connect messaging channels to your AI agent</p>

      <div class="channels-grid">
        {channels.map(channel => (
          <div key={channel.id} class="channel-card">
            <div class="channel-icon">
              {channel.type === 'telegram' && '📱'}
              {channel.type === 'discord' && '🎮'}
              {channel.type === 'slack' && '💬'}
              {channel.type === 'zalo' && '💚'}
              {channel.type === 'whatsapp' && '📞'}
            </div>
            <h3>{channel.name}</h3>
            <p class="channel-status">
              {channel.connected ? '🟢 Connected' : '⚪ Not connected'}
            </p>
            <button 
              class={channel.connected ? 'btn-disconnect' : 'btn-connect'}
              onClick={() => channel.connected 
                ? disconnectChannel(channel.id) 
                : connectChannel(channel.id)
              }
            >
              {channel.connected ? 'Disconnect' : 'Connect'}
            </button>
          </div>
        ))}
      </div>

      <section class="add-channel">
        <h3>Add New Channel</h3>
        <p>More channels coming soon: Signal, LINE, Facebook Messenger, Email</p>
      </section>
    </div>
  )
}
