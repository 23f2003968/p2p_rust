<script setup>
import { ref, computed } from 'vue';

const props = defineProps({
  messages: Array,
  activeContact: Object,
  isGroupChat: Boolean,
});

const messagesContainer = ref(null);

// Generate deterministic color from a string
function colorFromName(name) {
  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  const hue = Math.abs(hash) % 360;
  return `hsl(${hue}, 70%, 65%)`;
}

// Compute display names with collision suffixes
// If two peers have the same username, append "-1", "-2" sorted by peer_id
const displayNameMap = computed(() => {
  if (!props.isGroupChat || !props.messages) return {};
  
  // Group peer_ids by their username (from field)
  const nameToIds = {};
  for (const msg of props.messages) {
    if (msg.is_self) continue;
    const name = msg.from || 'unknown';
    const pid = msg.peer_id || name;
    if (!nameToIds[name]) nameToIds[name] = new Set();
    nameToIds[name].add(pid);
  }
  
  // Build map: peer_id -> display name
  const result = {};
  for (const [name, ids] of Object.entries(nameToIds)) {
    const sortedIds = [...ids].sort();
    if (sortedIds.length > 1) {
      sortedIds.forEach((id, idx) => {
        result[id] = `${name}-${idx + 1}`;
      });
    } else {
      result[sortedIds[0]] = name;
    }
  }
  return result;
});

function getDisplayName(msg) {
  if (msg.is_self) return 'You';
  const pid = msg.peer_id || msg.from;
  return displayNameMap.value[pid] || msg.from || 'unknown';
}

const formatTime = (timestamp) => {
  const date = new Date(timestamp);
  return date.toLocaleTimeString('en-US', { 
    hour: '2-digit', 
    minute: '2-digit',
  });
};

const scrollToBottom = () => {
  if (messagesContainer.value) {
    setTimeout(() => {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
    }, 0);
  }
};

defineExpose({ scrollToBottom });
</script>

<template>
  <div class="messages-container" ref="messagesContainer">
    <div v-if="!activeContact" class="no-contact-selected">
      <p>No chat selected</p>
      <span>Select a contact to start chatting</span>
    </div>

    <div v-else-if="!messages || messages.length === 0" class="no-messages">
      <p>No messages yet</p>
      <span>Start a conversation with {{ activeContact.nickname }}</span>
    </div>

    <div v-else class="messages-list">
      <div
        v-for="(msg, index) in messages"
        :key="index"
        :class="['message', msg.is_self ? 'self' : 'peer']"
      >
        <!-- Show sender name only in group chats, not for own messages -->
        <div
          v-if="isGroupChat && !msg.is_self"
          class="message-sender"
          :style="{ color: colorFromName(getDisplayName(msg)) }"
        >{{ getDisplayName(msg) }}</div>
        <div class="message-content">{{ msg.content }}</div>
        <div class="message-meta">
          <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.messages-container {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem 0.75rem;
  display: flex;
  flex-direction: column;
  background: #0a0a0a;
}

.no-contact-selected,
.no-messages {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  color: #444;
}

.no-contact-selected p,
.no-messages p {
  margin: 0 0 0.25rem 0;
  font-size: 0.75rem;
  color: #666;
}

.no-contact-selected span,
.no-messages span {
  margin: 0;
  font-size: 0.6875rem;
  color: #444;
}

.messages-container::-webkit-scrollbar {
  width: 4px;
}

.messages-container::-webkit-scrollbar-track {
  background: transparent;
}

.messages-container::-webkit-scrollbar-thumb {
  background: #222;
  border-radius: 2px;
}

.messages-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin-top: auto;
}

.message {
  padding: 0.375rem 0.5rem;
  border-radius: 2px;
  max-width: 75%;
  border: 1px solid transparent;
  font-size: 0.8125rem;
}

.message.self {
  align-self: flex-end;
  background: #002200;
  color: #00ff41;
  border-color: #003300;
  text-align: right;
}

.message.peer {
  align-self: flex-start;
  background: #111;
  color: #c0c0c0;
  border-color: #1a1a1a;
}

.message-sender {
  font-size: 0.5625rem;
  font-weight: 700;
  margin-bottom: 0.125rem;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.message-meta {
  display: flex;
  justify-content: flex-end;
  margin-top: 0.125rem;
}

.message-time {
  font-size: 0.5625rem;
  opacity: 0.4;
  font-family: inherit;
}

.message-content {
  font-size: 0.8125rem;
  line-height: 1.4;
  word-wrap: break-word;
}
</style>