<script setup>
import { computed } from 'vue';

const props = defineProps({
  contacts: Map,
  activeContactId: String,
  onSelectContact: Function,
  username: String,
});

const emit = defineEmits(['select-contact', 'connect-peer', 'accept-peer', 'reject-peer', 'show-connect-modal', 'show-join-modal', 'update:username', 'save-username']);

const discoveredPeers = computed(() => {
  if (!props.contacts) return [];
  return Array.from(props.contacts.values())
    .filter(c => c.status === 'discovered')
    .sort((a, b) => new Date(b.lastMessageTime || 0) - new Date(a.lastMessageTime || 0));
});

const requestPeers = computed(() => {
  if (!props.contacts) return [];
  return Array.from(props.contacts.values())
    .filter(c => c.status === 'request')
    .sort((a, b) => new Date(b.lastMessageTime || 0) - new Date(a.lastMessageTime || 0));
});

const connectedContacts = computed(() => {
  if (!props.contacts) return [];
  return Array.from(props.contacts.values())
    .filter(c => c.status !== 'discovered' && c.status !== 'request')
    .sort((a, b) => new Date(b.lastMessageTime || 0) - new Date(a.lastMessageTime || 0));
});

const getInitials = (nickname) => {
  return nickname
    .split('-')
    .map(word => word[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);
};

const getStatusColor = (status) => {
  switch (status) {
    case 'connected':
      return '#10b981';
    case 'connecting':
      return '#f59e0b';
    case 'disconnected':
      return '#ef4444';
    default:
      return '#6b7280';
  }
};

const formatTime = (timestamp) => {
  if (!timestamp) return '';
  const date = new Date(timestamp);
  const now = new Date();
  const diffMs = now - date;
  const diffMins = Math.floor(diffMs / 60000);

  if (diffMins < 1) return 'now';
  if (diffMins < 60) return `${diffMins}m`;
  if (diffMins < 1440) return `${Math.floor(diffMins / 60)}h`;
  if (diffMins < 10080) return `${Math.floor(diffMins / 1440)}d`;

  return date.toLocaleDateString();
};

const handleSelectContact = (peerId) => {
  emit('select-contact', peerId);
};

const handleConnect = (peerId) => {
  emit('connect-peer', peerId);
};

const handleAccept = (peerId) => {
  emit('accept-peer', peerId);
};

const handleReject = (peerId) => {
  emit('reject-peer', peerId);
};
</script>

<template>
  <div class="sidebar">
    <div class="sidebar-header">
      <h2>Chats</h2>
      <div class="header-actions">
        <button class="action-btn primary" @click="emit('show-connect-modal')" title="Connect by address">
          +
        </button>
        <button class="action-btn primary" @click="emit('show-join-modal')" title="Join room">
          #
        </button>
      </div>
    </div>

    <div class="username-bar">
      <span class="username-label">NAME</span>
      <input
        class="username-input"
        :value="username"
        @input="emit('update:username', $event.target.value)"
        @keyup.enter="$event.target.blur()"
        @blur="emit('save-username')"
        placeholder="set your name..."
      />
    </div>

    <div v-if="discoveredPeers.length === 0 && requestPeers.length === 0 && connectedContacts.length === 0" class="no-contacts">
      <p class="no-contacts-text">No peers found</p>
      <p class="hint">Peers on your network will appear here</p>
    </div>

    <!-- Available Peers -->
    <div v-if="discoveredPeers.length > 0" class="section">
      <div class="section-header">Available Peers</div>
      <div class="contacts-list">
        <div
          v-for="peer in discoveredPeers"
          :key="peer.peerId"
          class="contact-item discovered"
        >
          <div class="contact-avatar">
            <div class="avatar-circle discovered-avatar">{{ getInitials(peer.nickname) }}</div>
            <div class="status-dot" :style="{ backgroundColor: '#f59e0b' }"></div>
          </div>

          <div class="contact-info">
            <div class="contact-header">
              <span class="contact-nickname">{{ peer.nickname }}</span>
            </div>
            <p class="last-message">Discovered on network</p>
          </div>

          <button class="connect-btn" @click.stop="handleConnect(peer.peerId)">
            Connect
          </button>
        </div>
      </div>
    </div>

    <!-- Connection Requests -->
    <div v-if="requestPeers.length > 0" class="section">
      <div class="section-header">Requests</div>
      <div class="contacts-list">
        <div
          v-for="peer in requestPeers"
          :key="peer.peerId"
          class="contact-item request"
        >
          <div class="contact-avatar">
            <div class="avatar-circle request-avatar">{{ getInitials(peer.nickname) }}</div>
            <div class="status-dot" :style="{ backgroundColor: '#ef4444' }"></div>
          </div>

          <div class="contact-info">
            <div class="contact-header">
              <span class="contact-nickname">{{ peer.nickname }}</span>
            </div>
            <p class="last-message">Wants to connect</p>
          </div>

          <div class="request-actions">
            <button class="accept-btn" @click.stop="handleAccept(peer.peerId)" title="Accept">O</button>
            <button class="reject-btn" @click.stop="handleReject(peer.peerId)" title="Reject">X</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Connected Chats -->
    <div v-if="connectedContacts.length > 0" class="section">
      <div class="section-header">Chats</div>
      <div class="contacts-list">
        <div
          v-for="contact in connectedContacts"
          :key="contact.peerId"
          :class="['contact-item', { active: contact.peerId === activeContactId }]"
          @click="handleSelectContact(contact.peerId)"
        >
          <div class="contact-avatar">
            <div class="avatar-circle">{{ getInitials(contact.nickname) }}</div>
            <div class="status-dot" :style="{ backgroundColor: getStatusColor(contact.status) }"></div>
          </div>

          <div class="contact-info">
            <div class="contact-header">
              <span class="contact-nickname">{{ contact.nickname }}</span>
              <span v-if="contact.unreadCount > 0" class="unread-badge">{{ contact.unreadCount }}</span>
            </div>
            <p class="last-message">{{ contact.lastMessage || 'No messages yet' }}</p>
            <span class="message-time">{{ formatTime(contact.lastMessageTime) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  width: 260px;
  background: #0d0d0d;
  border-right: 1px solid #1a1a1a;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow-y: auto;
  font-family: inherit;
}

.sidebar-header {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid #1a1a1a;
  background: #111111;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.sidebar-header h2 {
  margin: 0;
  font-size: 0.8125rem;
  font-weight: 700;
  color: #00ff41;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.header-actions {
  display: flex;
  gap: 0.25rem;
}

.action-btn {
  width: 28px;
  height: 28px;
  border: 1px solid #333;
  border-radius: 2px;
  background: transparent;
  cursor: pointer;
  font-size: 0.75rem;
  color: #808080;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.1s ease;
}

.action-btn:hover {
  background: #1a1a1a;
  color: #00ff41;
  border-color: #00ff41;
}

.username-bar {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.75rem;
  border-bottom: 1px solid #1a1a1a;
  background: #0d0d0d;
}

.username-label {
  color: #555;
  font-size: 0.5625rem;
  font-weight: 700;
  letter-spacing: 0.5px;
  flex-shrink: 0;
}

.username-input {
  flex: 1;
  background: transparent;
  border: none;
  border-bottom: 1px solid #222;
  color: #00ff41;
  font-family: inherit;
  font-size: 0.6875rem;
  padding: 0.125rem 0;
  outline: none;
  transition: all 0.1s ease;
}

.username-input:focus {
  border-color: #00ff41;
}

.username-input::placeholder {
  color: #333;
}

.action-btn.primary {
  color: #00ff41;
  border-color: #00ff41;
  font-weight: 700;
}

.action-btn.primary:hover {
  background: #00ff41;
  color: #000;
}

.section {
  flex-shrink: 0;
}

.section-header {
  padding: 0.375rem 0.75rem;
  font-size: 0.625rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: #555;
  border-bottom: 1px solid #1a1a1a;
}

.connect-btn {
  background: transparent;
  color: #00ff41;
  border: 1px solid #00ff41;
  border-radius: 2px;
  padding: 0.25rem 0.5rem;
  font-size: 0.6875rem;
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.1s ease;
  flex-shrink: 0;
  align-self: center;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.connect-btn:hover {
  background: #00ff41;
  color: #000;
}

.discovered-avatar {
  background: #3a3a00 !important;
  color: #ffcc00 !important;
}

.contact-item.discovered {
  cursor: default;
}

.contact-item.request {
  cursor: default;
}

.request-avatar {
  background: #3a0000 !important;
  color: #ff4444 !important;
}

.request-actions {
  display: flex;
  gap: 0.25rem;
  flex-shrink: 0;
  align-self: center;
}

.accept-btn,
.reject-btn {
  width: 24px;
  height: 24px;
  border: 1px solid;
  border-radius: 2px;
  cursor: pointer;
  font-size: 0.6875rem;
  font-weight: 700;
  font-family: inherit;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.1s ease;
}

.accept-btn {
  background: transparent;
  color: #00ff41;
  border-color: #00ff41;
}

.accept-btn:hover {
  background: #00ff41;
  color: #000;
}

.reject-btn {
  background: transparent;
  color: #ff4444;
  border-color: #ff4444;
}

.reject-btn:hover {
  background: #ff4444;
  color: #000;
}

.no-contacts {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #444;
  text-align: center;
  padding: 1rem;
}

.no-contacts-text {
  margin: 0.5rem 0;
  font-size: 0.75rem;
  color: #666;
}

.hint {
  margin: 0.25rem 0 0 0;
  font-size: 0.6875rem;
  color: #444;
}

.contacts-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 0.25rem;
}

.contact-item {
  display: flex;
  gap: 0.5rem;
  padding: 0.5rem 0.625rem;
  border-radius: 2px;
  cursor: pointer;
  transition: all 0.1s ease;
  border: 1px solid transparent;
}

.contact-item:hover {
  background: #1a1a1a;
}

.contact-item.active {
  background: #1a1a1a;
  border-color: #00ff41;
}

.contact-avatar {
  position: relative;
  flex-shrink: 0;
}

.avatar-circle {
  width: 32px;
  height: 32px;
  border-radius: 2px;
  background: #1a1a1a;
  border: 1px solid #333;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #808080;
  font-size: 0.625rem;
  font-weight: 700;
  font-family: inherit;
}

.status-dot {
  position: absolute;
  bottom: -1px;
  right: -1px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  border: 2px solid #0d0d0d;
}

@keyframes statusPulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.contact-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.contact-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
}

.contact-nickname {
  color: #c0c0c0;
  font-size: 0.75rem;
  font-weight: 500;
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
}

.unread-badge {
  background: #ff4444;
  color: #000;
  font-size: 0.5625rem;
  font-weight: 700;
  padding: 0.0625rem 0.25rem;
  border-radius: 2px;
  flex-shrink: 0;
  min-width: 16px;
  text-align: center;
}

.last-message {
  margin: 0;
  color: #555;
  font-size: 0.6875rem;
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
}

.message-time {
  color: #444;
  font-size: 0.5625rem;
  margin-top: 0.125rem;
}

.contacts-list::-webkit-scrollbar {
  width: 4px;
}

.contacts-list::-webkit-scrollbar-track {
  background: transparent;
}

.contacts-list::-webkit-scrollbar-thumb {
  background: #222;
  border-radius: 2px;
}

/* Mobile */
@media (max-width: 640px) {
  .sidebar {
    width: 100%;
    max-height: 35vh;
    border-right: none;
    border-bottom: 1px solid #1a1a1a;
  }
}
</style>