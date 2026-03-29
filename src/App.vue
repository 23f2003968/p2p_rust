<script setup>
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// Component imports
import Header from './components/Header.vue';
import Addresses from './components/Addresses.vue';
import Messages from './components/Messages.vue';
import Sidebar from './components/Sidebar.vue';
import InputArea from './components/InputArea.vue';
import LoadingBar from './components/LoadingBar.vue';
import ConnectPeerModal from './components/ConnectPeerModal.vue';
import JoinRoomModal from './components/JoinRoomModal.vue';

// ============================================================================
// STATE - Core data
// ============================================================================

const peerID = ref('');
const addresses = ref([]);
const isInitialized = ref(false);
const currentEvent = ref('');
const inputMessage = ref('');

const contacts = ref(new Map());
const activeContactId = ref(null);
const showChatView = ref(false);
const username = ref('');

// Dialog States
const showConnectModal = ref(false);
const showJoinModal = ref(false);
const peerAddressInput = ref('');
const roomInput = ref('');

// Component refs
const messagesRef = ref(null);
const sidebarRef = ref(null);

// Event listener cleanup
let unlistenChat = null;
let unlistenConnection = null;
let statusTimeoutId = null;

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

function shortPeerId(id) {
  if (!id) return 'unknown';
  return id.length > 12 ? id.slice(-8) : id;
}

function updateCurrentEvent(content) {
  currentEvent.value = content;
  if (statusTimeoutId) clearTimeout(statusTimeoutId);
  statusTimeoutId = setTimeout(() => {
    currentEvent.value = '';
  }, 5000);
}

function scrollToBottom() {
  if (messagesRef.value?.scrollToBottom) {
    messagesRef.value.scrollToBottom();
  }
}

// ============================================================================
// CONTACT MANAGEMENT
// ============================================================================

function createContact(peerId) {
  return {
    peerId,
    nickname: '',
    messages: [],
    unreadCount: 0,
    lastMessage: '',
    lastMessageTime: null,
    status: 'connecting',
  };
}

function addOrUpdateContact(peerId, updates = {}) {
  if (!contacts.value.has(peerId)) {
    contacts.value.set(peerId, createContact(peerId));
  }
  const contact = contacts.value.get(peerId);
  Object.assign(contact, updates);
  return contact;
}

function getActiveContact() {
  if (!activeContactId.value) return null;
  return contacts.value.get(activeContactId.value);
}

function getActiveMessages() {
  const contact = getActiveContact();
  return contact?.messages || [];
}

// Group chat = 2+ unique non-self senders in the message list
const isGroupChat = computed(() => {
  const msgs = getActiveMessages();
  const senders = new Set(msgs.filter(m => !m.is_self).map(m => m.from));
  return senders.size > 1;
});

function selectContact(peerId) {
  activeContactId.value = peerId;
  const contact = contacts.value.get(peerId);
  if (contact) {
    contact.unreadCount = 0;
    scrollToBottom();
  }
}

function openChat(peerId) {
  selectContact(peerId);
  showChatView.value = true;
}

// ============================================================================
// MESSAGE HANDLING
// ============================================================================

function addMessageToContact(peerId, message) {
  const contact = addOrUpdateContact(peerId);
  contact.messages.push(message);
  
  // Update last message preview
  contact.lastMessage = message.content.substring(0, 50) + 
    (message.content.length > 50 ? '...' : '');
  contact.lastMessageTime = new Date(message.timestamp).getTime();
  
  // Increment unread count if not active
  if (activeContactId.value !== peerId) {
    contact.unreadCount++;
  }
  
  // Scroll if this is active contact
  if (activeContactId.value === peerId) {
    scrollToBottom();
  }
}

async function sendMessage() {
  if (!inputMessage.value.trim() || !activeContactId.value) return;

  try {
    const contact = contacts.value.get(activeContactId.value);
    if (!contact) return;

    await invoke('send_message', { 
      peer_id: activeContactId.value,
      message: inputMessage.value
    });

    // Don't add message locally — backend sends ChatMessage event on publish
    inputMessage.value = '';
  } catch (error) {
    console.error('Failed to send message:', error);
    updateCurrentEvent('[!] Failed to send message');
  }
}

// ============================================================================
// CONNECTION HANDLING
// ============================================================================

// Called when mDNS discovers a peer — adds to sidebar as available
function handlePeerDiscovered(discoveredPeerId) {
  console.log('[App] Peer discovered:', discoveredPeerId);
  // Skip own peer ID
  if (discoveredPeerId === peerID.value) return;
  if (!contacts.value.has(discoveredPeerId)) {
    addOrUpdateContact(discoveredPeerId, { 
      status: 'discovered',
      nickname: shortPeerId(discoveredPeerId),
    });
  }
}

// Called when user clicks Connect on a discovered peer in sidebar
async function connectToPeer(peerId) {
  console.log('[App] connectToPeer called for:', peerId);
  if (!peerId) return;

  // Show connecting status (NOT connected yet)
  addOrUpdateContact(peerId, { status: 'connecting' });

  try {
    // Dial the peer via the backend — connection-established event will confirm
    console.log('[App] Invoking request_connection for:', peerId);
    await invoke('request_connection', { peer_id: peerId });
    console.log('[App] ✓ Dial initiated for:', peerId);
  } catch (error) {
    console.error('[App] Failed to connect to peer:', error);
    addOrUpdateContact(peerId, { status: 'discovered' });
    updateCurrentEvent('[!] Failed to connect to peer');
  }
}

// Called when another peer explicitly requests connection (incoming-connection event)
function handleIncomingConnection(peerId) {
  console.log('[App] Incoming connection request from:', peerId);
  // Skip own peer ID
  if (peerId === peerID.value) return;
  // Add to contacts as a 'request' — shows in sidebar Requests section
  if (!contacts.value.has(peerId)) {
    addOrUpdateContact(peerId, {
      status: 'request',
      nickname: shortPeerId(peerId),
    });
  } else {
    addOrUpdateContact(peerId, { status: 'request' });
  }
}

async function acceptConnection(peerId) {
  console.log('[App] acceptConnection called for peer:', peerId);
  if (!peerId) {
    console.error('[App] acceptConnection: No peer ID!');
    return;
  }

  try {
    console.log('[App] Invoking accept_connection with peer_id:', peerId);
    await invoke('accept_connection', { peer_id: peerId });
    console.log('[App] ✓ Backend accepted connection for:', peerId);
    
    addOrUpdateContact(peerId, { status: 'connected' });
    selectContact(peerId);
    updateCurrentEvent('Connection accepted');
  } catch (error) {
    console.error('[App] Failed to accept connection:', error);
    updateCurrentEvent('[!] Failed to accept connection');
  }
}

async function rejectConnection(peerId) {
  console.log('[App] rejectConnection called for peer:', peerId);
  if (!peerId) return;

  try {
    await invoke('reject_connection', { peer_id: peerId });
    console.log('[App] Connection rejected for peer:', peerId);
  } catch (error) {
    console.error('[App] Failed to reject connection:', error);
    updateCurrentEvent('[!] Failed to reject connection');
  }

  // Remove from contacts
  contacts.value.delete(peerId);
}

// ============================================================================
// P2P INITIALIZATION
// ============================================================================

let initializationAttempted = false;

async function initP2P() {
  if (initializationAttempted) {
    console.log('P2P initialization already attempted, skipping...');
    return;
  }
  initializationAttempted = true;
  updateCurrentEvent('Initializing P2P Node');

  try {
    const id = await invoke('init_p2p');
    peerID.value = id;
    isInitialized.value = true;
    
    // Join default room for messaging
    try {
      await invoke('join_room', { room_name: 'general' });
      console.log('✓ Joined room: general');
    } catch (roomError) {
      console.error('Failed to join room:', roomError);
    }
    
    updateCurrentEvent('');

    // Fetch addresses
    updateNodeInfo();
    setInterval(updateNodeInfo, 5000);
  } catch (error) {
    console.error('Failed to initialize P2P:', error);
    updateCurrentEvent('[X] Failed to initialize P2P');
    initializationAttempted = false;
  }
}

async function updateNodeInfo() {
  try {
    const info = await invoke('get_node_info');
    addresses.value = info.addresses;
  } catch (error) {
    console.error('Failed to get node info:', error);
  }
}

// ============================================================================
// LIFECYCLE HOOKS
// ============================================================================

let unlistenDiscovery = null;

onMounted(async () => {
  // Listen for incoming chat messages
  unlistenChat = await listen('chat-message', (event) => {
    const message = event.payload;

    // Handle system messages in loading bar
    if (message.from === 'System') {
      let eventText = message.content
        .replace(/^\[[OX!>.].*?\]\s*/g, '')
        .replace(/^[^\w]/, '')
        .trim();
      updateCurrentEvent(eventText);
    } else {
      // Add to contact's messages
      const peerId = message.peer_id || message.from;
      addMessageToContact(peerId, message);
      
      // Update contact nickname from sender's username (if not self)
      if (!message.is_self && message.from && message.from !== 'Unknown') {
        const contact = contacts.value.get(peerId);
        if (contact) {
          contact.nickname = message.from;
        }
      }
    }
  });

  // Listen for peer discovery (mDNS) — shows peers in sidebar
  unlistenDiscovery = await listen('peer-discovered', (event) => {
    const peerId = event.payload.peerId || event.payload;
    console.log('[App] peer-discovered event:', peerId);
    handlePeerDiscovered(peerId);
  });

  // Listen for connection established (our outgoing dial succeeded)
  const unlistenEstablished = await listen('connection-established', (event) => {
    const peerId = event.payload.peerId || event.payload;
    console.log('[App] connection-established event:', peerId);
    if (peerId === peerID.value) return;
    addOrUpdateContact(peerId, { status: 'connected' });
    selectContact(peerId);
    updateCurrentEvent('Connected to peer');
  });

  // Listen for explicit incoming connections (someone dialed us)
  unlistenConnection = await listen('incoming-connection', (event) => {
    const peerId = event.payload.peerId || event.payload;
    console.log('[App] incoming-connection event:', peerId);
    handleIncomingConnection(peerId);
  });

  // Listen for peer disconnection
  const unlistenDisconnected = await listen('peer-disconnected', (event) => {
    const peerId = event.payload.peer_id || event.payload.peerId;
    console.log('[App] peer-disconnected event:', peerId);
    const contact = contacts.value.get(peerId);
    if (contact) {
      contact.status = 'disconnected';
    }
  });

  // Listen for mDNS peer expiry
  const unlistenExpired = await listen('peer-expired', (event) => {
    const peerId = event.payload.peer_id || event.payload.peerId;
    console.log('[App] peer-expired event:', peerId);
    const contact = contacts.value.get(peerId);
    if (contact && contact.status === 'discovered') {
      contacts.value.delete(peerId);
    }
  });

  window.addEventListener('keydown', handleKeydown);

  // Visual Viewport API — adjust height when mobile keyboard opens
  if (window.visualViewport) {
    const onViewportResize = () => {
      document.documentElement.style.setProperty(
        '--viewport-height',
        `${window.visualViewport.height}px`
      );
    };
    window.visualViewport.addEventListener('resize', onViewportResize);
    window.visualViewport.addEventListener('scroll', onViewportResize);
    onViewportResize(); // set initial value
    // Store cleanup ref
    window.__vpCleanup = () => {
      window.visualViewport.removeEventListener('resize', onViewportResize);
      window.visualViewport.removeEventListener('scroll', onViewportResize);
    };
  }

  // Load saved username from localStorage
  const savedName = localStorage.getItem('p2p_username');
  if (savedName) {
    username.value = savedName;
    try {
      await invoke('set_username', { name: savedName });
    } catch (e) {
      // Node may not be ready yet, will be set after init
    }
  }

  await initP2P();

  // Re-set username after init if we loaded one
  if (savedName) {
    try {
      await invoke('set_username', { name: savedName });
    } catch (e) {
      console.warn('Failed to set saved username:', e);
    }
  }
});

onUnmounted(async () => {
  if (unlistenChat) unlistenChat();
  if (unlistenDiscovery) unlistenDiscovery();
  if (unlistenConnection) unlistenConnection();
  if (statusTimeoutId) clearTimeout(statusTimeoutId);
  window.removeEventListener('keydown', handleKeydown);
  if (window.__vpCleanup) window.__vpCleanup();

  try {
    await invoke('cleanup_p2p');
  } catch (error) {
    console.error('Failed to cleanup P2P:', error);
  }
});

function handleKeydown(event) {
  if (event.key === 'Escape') {
    if (showConnectModal.value || showJoinModal.value) {
      showConnectModal.value = false;
      showJoinModal.value = false;
    }
  }
}

async function handleConnectPeer() {
  const addr = peerAddressInput.value.trim();
  if (!addr) return;
  try {
    await invoke('request_connection', { peer_id: addr });
    updateCurrentEvent('Connecting to peer...');
  } catch (error) {
    console.error('Failed to connect:', error);
    updateCurrentEvent('[!] ' + error);
  }
  peerAddressInput.value = '';
  showConnectModal.value = false;
}

async function handleJoinRoom() {
  const name = roomInput.value.trim();
  if (!name) return;
  try {
    await invoke('join_room', { room_name: name });
    updateCurrentEvent(`Joined room: ${name}`);
  } catch (error) {
    console.error('Failed to join room:', error);
    updateCurrentEvent('[!] ' + error);
  }
  roomInput.value = '';
  showJoinModal.value = false;
}

async function saveUsername() {
  const name = username.value.trim();
  if (!name) return;
  try {
    await invoke('set_username', { name });
    localStorage.setItem('p2p_username', name);
    updateCurrentEvent(`Name set: ${name}`);
  } catch (error) {
    console.error('Failed to set username:', error);
  }
}
</script>

<template>
  <div class="app-container">
    <!-- Desktop: side-by-side layout -->
    <!-- Mobile: list OR chat view -->

    <!-- Sidebar Panel (always visible on desktop, visible on mobile when not in chat) -->
    <div class="sidebar-panel" :class="{ 'mobile-hidden': showChatView }">
      <Sidebar
        ref="sidebarRef"
        :contacts="contacts"
        :activeContactId="activeContactId"
        :username="username"
        @select-contact="openChat"
        @connect-peer="connectToPeer"
        @accept-peer="acceptConnection"
        @reject-peer="rejectConnection"
        @show-connect-modal="showConnectModal = true"
        @show-join-modal="showJoinModal = true"
        @update:username="username = $event"
        @save-username="saveUsername"
      />
      <Addresses :addresses="addresses" />
    </div>

    <!-- Chat Panel (always visible on desktop, visible on mobile when in chat) -->
    <div class="chat-panel" :class="{ 'mobile-hidden': !showChatView }">
      <!-- Chat Header -->
      <div class="chat-header">
        <button class="back-btn mobile-only" @click="showChatView = false">&lt;</button>
        <div class="chat-header-info" v-if="getActiveContact()">
          <span class="chat-peer-name">{{ getActiveContact().nickname }}</span>
          <span class="chat-peer-status">{{ getActiveContact().status }}</span>
        </div>
        <div v-else class="chat-header-info">
          <span class="chat-peer-name" style="color: #444;">No chat selected</span>
        </div>
      </div>

      <!-- Messages -->
      <Messages 
        ref="messagesRef"
        :messages="getActiveMessages()"
        :activeContact="getActiveContact()"
        :isGroupChat="isGroupChat"
      />

      <!-- Input Area -->
      <InputArea
        :inputMessage="inputMessage"
        @update:inputMessage="inputMessage = $event"
        :isInitialized="!!(isInitialized && activeContactId)"
        :onSendMessage="sendMessage"
      />
    </div>

    <!-- Loading Bar (bottom) -->
    <LoadingBar :currentEvent="currentEvent" />

    <!-- Modals -->
    <ConnectPeerModal
      v-if="showConnectModal"
      v-model:peerAddressInput="peerAddressInput"
      :onConnectPeer="handleConnectPeer"
      :onClose="() => showConnectModal = false"
    />

    <JoinRoomModal
      v-if="showJoinModal"
      v-model:roomInput="roomInput"
      :onJoinRoom="handleJoinRoom"
      :onClose="() => showJoinModal = false"
    />
  </div>
</template>

<style scoped>
* {
  box-sizing: border-box;
}

.app-container {
  display: flex;
  height: var(--viewport-height, 100vh);
  background: #0a0a0a;
  color: #c0c0c0;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Consolas', monospace;
  font-size: 13px;
  padding-top: env(safe-area-inset-top, 0px);
  padding-bottom: env(safe-area-inset-bottom, 0px);
  padding-left: env(safe-area-inset-left, 0px);
  padding-right: env(safe-area-inset-right, 0px);
}

/* ===== Desktop: side-by-side ===== */
.sidebar-panel {
  width: 280px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-right: 1px solid #1a1a1a;
}

.sidebar-panel :deep(.sidebar) {
  width: 100% !important;
  max-height: none !important;
  flex: 1;
  border-right: none;
}

.sidebar-panel :deep(.addresses-section) {
  flex-shrink: 0;
}

.chat-panel {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.chat-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: #111;
  border-bottom: 1px solid #1a1a1a;
  flex-shrink: 0;
}

.back-btn {
  width: 28px;
  height: 28px;
  border: 1px solid #333;
  border-radius: 2px;
  background: transparent;
  color: #00ff41;
  font-family: inherit;
  font-size: 0.875rem;
  font-weight: 700;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.1s ease;
  flex-shrink: 0;
}

.back-btn:hover {
  background: #1a1a1a;
  border-color: #00ff41;
}

/* Hide back button on desktop */
.mobile-only {
  display: none;
}

.chat-header-info {
  display: flex;
  flex-direction: column;
  gap: 0.0625rem;
  min-width: 0;
}

.chat-peer-name {
  color: #c0c0c0;
  font-size: 0.75rem;
  font-weight: 500;
  text-overflow: ellipsis;
  overflow: hidden;
  white-space: nowrap;
}

.chat-peer-status {
  color: #555;
  font-size: 0.5625rem;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

/* ===== Mobile (<=640px): list OR chat ===== */
@media (max-width: 640px) {
  .app-container {
    flex-direction: column;
  }

  .sidebar-panel {
    width: 100%;
    flex: 1;
    border-right: none;
  }

  .chat-panel {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: var(--viewport-height, 100vh);
    z-index: 10;
    padding-top: env(safe-area-inset-top, 0px);
    padding-bottom: env(safe-area-inset-bottom, 0px);
    background: #0a0a0a;
  }

  .mobile-hidden {
    display: none !important;
  }

  .mobile-only {
    display: flex;
  }
}
</style>