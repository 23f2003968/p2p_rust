<script setup>
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// State
const messages = ref([]);
const inputMessage = ref('');
const peerID = ref('');
const addresses = ref([]);
const connectedPeers = ref([]);
const isInitialized = ref(false);
const joinRoomMode = ref(false);
const connectPeerMode = ref(false);
const roomInput = ref('');
const peerAddressInput = ref('');
const currentRoom = ref('');
const messagesContainer = ref(null);
const copiedIndex = ref(-1);

// Event listener cleanup
let unlisten = null;

// Initialize P2P node
async function initP2P() {
  try {
    const id = await invoke('init_p2p');
    peerID.value = id;
    isInitialized.value = true;
    
    // Fetch node info periodically
    updateNodeInfo();
    setInterval(updateNodeInfo, 5000);
  } catch (error) {
    console.error('Failed to initialize P2P:', error);
    addSystemMessage('❌ Failed to initialize P2P node: ' + error);
  }
}

// Update node info
async function updateNodeInfo() {
  try {
    const info = await invoke('get_node_info');
    addresses.value = info.addresses;
    connectedPeers.value = info.connected_peers;
  } catch (error) {
    console.error('Failed to get node info:', error);
  }
}

// Send message
async function sendMessage() {
  if (!inputMessage.value.trim()) return;
  
  try {
    await invoke('send_message', { message: inputMessage.value });
    inputMessage.value = '';
  } catch (error) {
    console.error('Failed to send message:', error);
    addSystemMessage('⚠ Failed to send message: ' + error);
  }
}

// Join room
async function joinRoom() {
  if (!roomInput.value.trim()) return;
  
  try {
    await invoke('join_room', { roomName: roomInput.value });
    currentRoom.value = roomInput.value;
    joinRoomMode.value = false;
    roomInput.value = '';
  } catch (error) {
    console.error('Failed to join room:', error);
    addSystemMessage('⚠ Failed to join room: ' + error);
  }
}

// Connect to peer
async function connectToPeer() {
  if (!peerAddressInput.value.trim()) return;
  
  try {
    await invoke('connect_to_peer', { addr: peerAddressInput.value });
    connectPeerMode.value = false;
    peerAddressInput.value = '';
  } catch (error) {
    console.error('Failed to connect to peer:', error);
    addSystemMessage('⚠ Failed to connect to peer: ' + error);
  }
}

// Add system message
function addSystemMessage(content) {
  messages.value.push({
    from: 'System',
    content,
    timestamp: new Date().toISOString(),
    is_self: false,
  });
  scrollToBottom();
}

// Scroll to bottom
async function scrollToBottom() {
  await nextTick();
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
  }
}

// Format timestamp
function formatTime(timestamp) {
  const date = new Date(timestamp);
  return date.toLocaleTimeString('en-US', { 
    hour: '2-digit', 
    minute: '2-digit',
    second: '2-digit'
  });
}

// Short peer ID
function shortPeerID(id) {
  if (id.length > 16) {
    return id.substring(0, 8) + '...' + id.substring(id.length - 6);
  }
  return id;
}

// Handle keyboard shortcuts
function handleKeydown(event) {
  if (event.ctrlKey && event.key === 'j') {
    event.preventDefault();
    joinRoomMode.value = true;
  }
  if (event.ctrlKey && event.key === 'p') {
    event.preventDefault();
    connectPeerMode.value = true;
  }
}

// Copy address to clipboard
async function copyAddress(addr, index) {
  try {
    await navigator.clipboard.writeText(addr);
    copiedIndex.value = index;
    setTimeout(() => {
      copiedIndex.value = -1;
    }, 2000);
  } catch (error) {
    console.error('Failed to copy:', error);
  }
}

// Lifecycle hooks
onMounted(async () => {
  // Listen for chat messages from Rust
  unlisten = await listen('chat-message', (event) => {
    messages.value.push(event.payload);
    scrollToBottom();
  });
  
  // Add keyboard listener
  window.addEventListener('keydown', handleKeydown);
  
  // Initialize P2P
  await initP2P();
});

onUnmounted(() => {
  if (unlisten) unlisten();
  window.removeEventListener('keydown', handleKeydown);
});
</script>

<template>
  <div class="chat-container">
    <!-- Header -->
    <div class="header">
      <h1>P2P Chat</h1>
      <div class="peer-info">
        <div class="peer-id">
          <span class="label">Peer ID:</span>
          <span class="value">{{ shortPeerID(peerID) }}</span>
        </div>
        <div class="peers-count">
          <span class="label">Connected Peers:</span>
          <span class="value">{{ connectedPeers.length }}</span>
        </div>
        <div v-if="currentRoom" class="current-room">
          <span class="label">Room:</span>
          <span class="value">{{ currentRoom }}</span>
        </div>
      </div>
    </div>

    <!-- Addresses Section -->
    <div class="addresses-section" v-if="addresses.length > 0">
      <div class="section-title">Listen Addresses</div>
      <div class="addresses-list">
        <div v-for="(addr, index) in addresses" :key="index" class="address-item">
          <div class="address">{{ addr }}</div>
          <button 
            @click="copyAddress(addr, index)" 
            class="copy-btn"
            :class="{ copied: copiedIndex === index }"
          >
            {{ copiedIndex === index ? '✓ Copied' : 'Copy' }}
          </button>
        </div>
      </div>
    </div>

    <!-- Messages Container -->
    <div class="messages-container" ref="messagesContainer">
      <div
        v-for="(msg, index) in messages"
        :key="index"
        :class="['message', msg.is_self ? 'self' : msg.from === 'System' ? 'system' : 'peer']"
      >
        <div class="message-header">
          <span class="message-from">{{ msg.from }}</span>
          <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
        </div>
        <div class="message-content">{{ msg.content }}</div>
      </div>
      <div v-if="messages.length === 0" class="no-messages">
        No messages yet. Press <kbd>Ctrl+J</kbd> to join a room!
      </div>
    </div>

    <!-- Join Room Modal -->
    <div v-if="joinRoomMode" class="modal-overlay" @click="joinRoomMode = false">
      <div class="modal" @click.stop>
        <h3>Join a Room</h3>
        <input
          v-model="roomInput"
          @keyup.enter="joinRoom"
          @keyup.esc="joinRoomMode = false"
          placeholder="Enter room name..."
          autofocus
          class="room-input"
        />
        <div class="modal-buttons">
          <button @click="joinRoom" class="btn-primary">Join</button>
          <button @click="joinRoomMode = false; roomInput = ''" class="btn-secondary">
            Cancel
          </button>
        </div>
      </div>
    </div>

    <!-- Connect to Peer Modal -->
    <div v-if="connectPeerMode" class="modal-overlay" @click="connectPeerMode = false">
      <div class="modal" @click.stop>
        <h3>Connect to Peer</h3>
        <p class="modal-help">Enter the full multiaddr of the peer (e.g., /ip6/::1/tcp/8080/p2p/12D3...)</p>
        <input
          v-model="peerAddressInput"
          @keyup.enter="connectToPeer"
          @keyup.esc="connectPeerMode = false"
          placeholder="Paste peer multiaddr..."
          autofocus
          class="room-input"
        />
        <div class="modal-buttons">
          <button @click="connectToPeer" class="btn-primary">Connect</button>
          <button @click="connectPeerMode = false; peerAddressInput = ''" class="btn-secondary">
            Cancel
          </button>
        </div>
      </div>
    </div>

    <!-- Input Area -->
    <div class="input-area">
      <div class="shortcuts">
        <span class="shortcut"><kbd>Ctrl+J</kbd> Join Room</span>
        <span class="shortcut"><kbd>Ctrl+P</kbd> Connect Peer</span>
        <span class="shortcut"><kbd>Enter</kbd> Send Message</span>
      </div>
      <div class="input-row">
        <textarea
          v-model="inputMessage"
          @keyup.enter.exact="sendMessage"
          placeholder="Type a message..."
          class="message-input"
          :disabled="!isInitialized"
          rows="3"
        ></textarea>
        <button @click="sendMessage" :disabled="!isInitialized || !inputMessage.trim()" class="send-button">
          Send
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
* {
  box-sizing: border-box;
}

.chat-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #1e1e1e;
  color: #f0f0f0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif;
}

.header {
  padding: 1rem 1.5rem;
  background: #2a2a2a;
  border-bottom: 1px solid #3a3a3a;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
}

.header h1 {
  margin: 0 0 0.75rem 0;
  font-size: 1.25rem;
  font-weight: 600;
  color: #ffffff;
  letter-spacing: -0.015em;
}

.peer-info {
  display: flex;
  gap: 1.5rem;
  flex-wrap: wrap;
  font-size: 0.8125rem;
}

.peer-info > div {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.label {
  color: #a0a0a0;
  font-weight: 500;
}

.value {
  color: #ffffff;
  font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, monospace;
  background: #353535;
  padding: 0.25rem 0.5rem;
  border-radius: 6px;
  font-size: 0.75rem;
  border: 1px solid #3a3a3a;
}

.addresses-section {
  padding: 0.75rem 1.5rem;
  background: #252525;
  border-bottom: 1px solid #3a3a3a;
}

.section-title {
  font-weight: 600;
  color: #60a5fa;
  margin-bottom: 0.5rem;
  font-size: 0.8125rem;
}

.addresses-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.address-item {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.address {
  flex: 1;
  font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, monospace;
  font-size: 0.6875rem;
  color: #b0b0b0;
  background: #2a2a2a;
  padding: 0.5rem 0.75rem;
  border-radius: 6px;
  overflow-x: auto;
  border: 1px solid #3a3a3a;
}

.copy-btn {
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 600;
  background: #353535;
  color: #f0f0f0;
  border: 1px solid #3a3a3a;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.copy-btn:hover {
  background: #404040;
  border-color: #60a5fa;
}

.copy-btn.copied {
  background: #00a884;
  color: #ffffff;
  border-color: #00a884;
}

.messages-container {
  flex: 1;
  overflow-y: auto;
  padding: 1rem 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  background: #1e1e1e;
}

.messages-container::-webkit-scrollbar {
  width: 12px;
}

.messages-container::-webkit-scrollbar-track {
  background: #1e1e1e;
}

.messages-container::-webkit-scrollbar-thumb {
  background: #404040;
  border-radius: 6px;
  border: 3px solid #1e1e1e;
}

.messages-container::-webkit-scrollbar-thumb:hover {
  background: #505050;
}

.message {
  padding: 0.75rem 1rem;
  border-radius: 8px;
  max-width: 70%;
  animation: slideIn 0.2s ease;
  border: 1px solid transparent;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateY(5px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.message.self {
  align-self: flex-end;
  background: #005c4b;
  color: #ffffff;
  border-color: #005c4b;
}

.message.peer {
  align-self: flex-start;
  background: #2a2a2a;
  color: #f0f0f0;
  border-color: #3a3a3a;
}

.message.system {
  align-self: center;
  background: rgba(96, 165, 250, 0.15);
  border: 1px solid #60a5fa;
  color: #60a5fa;
  max-width: 85%;
  font-size: 0.8125rem;
  text-align: center;
}

.message-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 0.375rem;
  font-size: 0.75rem;
  opacity: 0.9;
}

.message-from {
  font-weight: 600;
}

.message-time {
  font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, monospace;
  opacity: 0.7;
}

.message-content {
  font-size: 0.875rem;
  line-height: 1.5;
  word-wrap: break-word;
}

.no-messages {
  text-align: center;
  color: #a0a0a0;
  padding: 3rem;
  font-size: 0.875rem;
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  animation: fadeIn 0.15s ease;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

.modal {
  background: #2a2a2a;
  padding: 1.5rem;
  border-radius: 12px;
  min-width: 450px;
  border: 1px solid #3a3a3a;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.6);
  animation: scaleIn 0.2s ease;
}

@keyframes scaleIn {
  from {
    transform: scale(0.95);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

.modal h3 {
  margin: 0 0 1rem 0;
  color: #ffffff;
  font-size: 1.125rem;
  font-weight: 600;
}

.modal-help {
  margin: 0 0 1rem 0;
  color: #a0a0a0;
  font-size: 0.8125rem;
  line-height: 1.5;
}

.room-input {
  width: 100%;
  padding: 0.625rem 0.75rem;
  font-size: 0.875rem;
  border: 1px solid #3a3a3a;
  border-radius: 6px;
  background: #1e1e1e;
  color: #f0f0f0;
  margin-bottom: 1rem;
  font-family: inherit;
  transition: all 0.15s ease;
}

.room-input:focus {
  outline: none;
  border-color: #60a5fa;
  box-shadow: 0 0 0 3px rgba(96, 165, 250, 0.25);
}

.modal-buttons {
  display: flex;
  gap: 0.75rem;
  justify-content: flex-end;
}

.input-area {
  padding: 1rem 1.5rem;
  background: #2a2a2a;
  border-top: 1px solid #3a3a3a;
}

.shortcuts {
  display: flex;
  gap: 1.25rem;
  margin-bottom: 0.75rem;
  font-size: 0.75rem;
  flex-wrap: wrap;
}

.shortcut {
  color: #a0a0a0;
  display: flex;
  align-items: center;
  gap: 0.375rem;
}

kbd {
  background: #353535;
  color: #f0f0f0;
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
  font-size: 0.6875rem;
  font-weight: 600;
  border: 1px solid #404040;
  font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, monospace;
}

.input-row {
  display: flex;
  gap: 0.75rem;
  align-items: flex-end;
}

.message-input {
  flex: 1;
  padding: 0.625rem 0.75rem;
  font-size: 0.875rem;
  border: 1px solid #3a3a3a;
  border-radius: 6px;
  background: #1e1e1e;
  color: #f0f0f0;
  font-family: inherit;
  resize: none;
  transition: all 0.15s ease;
  line-height: 1.5;
}

.message-input:focus {
  outline: none;
  border-color: #60a5fa;
  box-shadow: 0 0 0 3px rgba(96, 165, 250, 0.25);
}

.message-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  background: #252525;
}

.message-input::placeholder {
  color: #808080;
}

.send-button,
.btn-primary,
.btn-secondary {
  padding: 0.5rem 1.25rem;
  font-size: 0.875rem;
  font-weight: 600;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.send-button,
.btn-primary {
  background: #00a884;
  color: #ffffff;
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.send-button:hover:not(:disabled),
.btn-primary:hover {
  background: #00c896;
}

.send-button:active:not(:disabled),
.btn-primary:active {
  background: #008c6f;
  box-shadow: inset 0 1px 0 rgba(0, 0, 0, 0.2);
}

.send-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  background: #353535;
  color: #808080;
}

.btn-secondary {
  background: transparent;
  color: #f0f0f0;
  border: 1px solid #3a3a3a;
}

.btn-secondary:hover {
  background: #353535;
  border-color: #404040;
}

.btn-secondary:active {
  background: #2a2a2a;
}

.current-room {
  color: #4ade80;
}

.current-room .label {
  color: #a0a0a0;
}

.current-room .value {
  background: rgba(74, 222, 128, 0.15);
  border-color: #4ade80;
  color: #4ade80;
}
</style>

