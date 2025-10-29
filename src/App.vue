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
      <h1>🚀 P2P Chat - Tauri + Rust + Vue</h1>
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
      <div class="section-title">📡 My Addresses:</div>
      <div class="addresses-list">
        <div v-for="(addr, index) in addresses.slice(0, 3)" :key="index" class="address">
          {{ addr }}
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
.chat-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
  color: #f0f0f0;
  font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
}

.header {
  padding: 1.5rem;
  background: linear-gradient(90deg, #0f3460 0%, #16213e 100%);
  border-bottom: 2px solid #e94560;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
}

.header h1 {
  margin: 0 0 1rem 0;
  font-size: 1.8rem;
  background: linear-gradient(90deg, #00d4ff, #ff00ff);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.peer-info {
  display: flex;
  gap: 2rem;
  flex-wrap: wrap;
  font-size: 0.9rem;
}

.peer-info > div {
  display: flex;
  gap: 0.5rem;
}

.label {
  color: #00d4ff;
  font-weight: 600;
}

.value {
  color: #fff;
  font-family: 'Courier New', monospace;
  background: rgba(255, 255, 255, 0.1);
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
}

.addresses-section {
  padding: 1rem 1.5rem;
  background: rgba(0, 0, 0, 0.2);
  border-bottom: 1px solid rgba(233, 69, 96, 0.3);
}

.section-title {
  font-weight: 600;
  color: #00d4ff;
  margin-bottom: 0.5rem;
}

.addresses-list {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.address {
  font-family: 'Courier New', monospace;
  font-size: 0.75rem;
  color: #b0b0b0;
  background: rgba(255, 255, 255, 0.05);
  padding: 0.3rem 0.6rem;
  border-radius: 4px;
  overflow-x: auto;
}

.messages-container {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
}

.messages-container::-webkit-scrollbar {
  width: 8px;
}

.messages-container::-webkit-scrollbar-track {
  background: rgba(0, 0, 0, 0.2);
}

.messages-container::-webkit-scrollbar-thumb {
  background: #e94560;
  border-radius: 4px;
}

.message {
  padding: 0.8rem 1rem;
  border-radius: 8px;
  max-width: 80%;
  animation: slideIn 0.3s ease;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.message.self {
  align-self: flex-end;
  background: linear-gradient(135deg, #00d4ff 0%, #0077ff 100%);
  color: #fff;
}

.message.peer {
  align-self: flex-start;
  background: linear-gradient(135deg, #ff00ff 0%, #ff0080 100%);
  color: #fff;
}

.message.system {
  align-self: center;
  background: rgba(0, 255, 135, 0.1);
  border: 1px solid #00ff87;
  color: #00ff87;
  max-width: 90%;
  font-size: 0.9rem;
}

.message-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 0.4rem;
  font-size: 0.8rem;
  opacity: 0.9;
}

.message-from {
  font-weight: 600;
}

.message-time {
  font-family: 'Courier New', monospace;
  opacity: 0.7;
}

.message-content {
  font-size: 0.95rem;
  line-height: 1.4;
  word-wrap: break-word;
}

.no-messages {
  text-align: center;
  color: #888;
  padding: 3rem;
  font-size: 1.1rem;
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
  padding: 2rem;
  border-radius: 12px;
  min-width: 400px;
  border: 2px solid #e94560;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.5);
}

.modal h3 {
  margin: 0 0 1.5rem 0;
  color: #00d4ff;
  font-size: 1.5rem;
}

.modal-help {
  margin: 0 0 1rem 0;
  color: #aaa;
  font-size: 0.9rem;
  line-height: 1.4;
}

.room-input {
  width: 100%;
  padding: 0.8rem;
  font-size: 1rem;
  border: 2px solid #00d4ff;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
  margin-bottom: 1.5rem;
}

.room-input:focus {
  outline: none;
  border-color: #ff00ff;
  box-shadow: 0 0 10px rgba(255, 0, 255, 0.3);
}

.modal-buttons {
  display: flex;
  gap: 1rem;
  justify-content: flex-end;
}

.input-area {
  padding: 1rem 1.5rem;
  background: rgba(0, 0, 0, 0.3);
  border-top: 2px solid #e94560;
}

.shortcuts {
  display: flex;
  gap: 1.5rem;
  margin-bottom: 0.8rem;
  font-size: 0.85rem;
}

.shortcut {
  color: #888;
}

kbd {
  background: #e94560;
  color: #fff;
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 600;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
}

.input-row {
  display: flex;
  gap: 1rem;
}

.message-input {
  flex: 1;
  padding: 0.8rem;
  font-size: 1rem;
  border: 2px solid rgba(0, 212, 255, 0.3);
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.05);
  color: #fff;
  font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  resize: none;
}

.message-input:focus {
  outline: none;
  border-color: #00d4ff;
  box-shadow: 0 0 10px rgba(0, 212, 255, 0.3);
}

.message-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.send-button,
.btn-primary,
.btn-secondary {
  padding: 0.8rem 2rem;
  font-size: 1rem;
  font-weight: 600;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.3s ease;
}

.send-button,
.btn-primary {
  background: linear-gradient(135deg, #e94560 0%, #ff0080 100%);
  color: #fff;
}

.send-button:hover:not(:disabled),
.btn-primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(233, 69, 96, 0.4);
}

.send-button:active:not(:disabled),
.btn-primary:active {
  transform: translateY(0);
}

.send-button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-secondary {
  background: rgba(255, 255, 255, 0.1);
  color: #fff;
  border: 2px solid rgba(255, 255, 255, 0.3);
}

.btn-secondary:hover {
  background: rgba(255, 255, 255, 0.2);
}

.current-room {
  color: #00ff87;
}
</style>

