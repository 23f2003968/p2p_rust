<script setup>
defineProps({
  activeContact: Object,
  peerID: String,
});

const getInitials = (nickname) => {
  if (!nickname) return '?';
  return nickname
    .split('-')
    .map(word => word[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);
};

</script>

<template>
  <div class="header">
    <div class="header-top">
      <div class="peer-id-info">
        <span class="label">Your ID:</span>
        <span class="value">{{ peerID }}</span>
      </div>
    </div>
    <div v-if="activeContact" class="active-contact-info">
      <div class="contact-avatar">
        <div class="avatar-circle">{{ getInitials(activeContact.nickname) }}</div>
        <div class="status-indicator" :class="activeContact.status"></div>
      </div>
      <div class="contact-details">
        <span class="contact-name">{{ activeContact.nickname }}</span>
        <span class="contact-status">{{ activeContact.status || 'connected' }}</span>
      </div>
    </div>

  </div>
</template>

<style scoped>
.header {
  padding: 0.5rem 0.75rem;
  background: #111;
  border-bottom: 1px solid #1a1a1a;
}

.header-top {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.peer-id-info {
  display: flex;
  gap: 0.375rem;
  align-items: center;
  font-size: 0.6875rem;
}

.label {
  color: #555;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.value {
  color: #00ff41;
  font-family: inherit;
  background: #0a0a0a;
  padding: 0.125rem 0.375rem;
  border-radius: 2px;
  font-size: 0.625rem;
  border: 1px solid #1a1a1a;
}

.active-contact-info {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.contact-avatar {
  position: relative;
  flex-shrink: 0;
}

.avatar-circle {
  width: 28px;
  height: 28px;
  border-radius: 2px;
  background: #1a1a1a;
  border: 1px solid #333;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #808080;
  font-size: 0.5625rem;
  font-weight: 700;
  font-family: inherit;
}

.status-indicator {
  position: absolute;
  bottom: -1px;
  right: -1px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  border: 2px solid #111;
}

.status-indicator.connected {
  background: #00ff41;
}

.status-indicator.connecting {
  background: #ffcc00;
}

.status-indicator.disconnected {
  background: #ff4444;
}

.contact-details {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.contact-name {
  color: #c0c0c0;
  font-size: 0.75rem;
  font-weight: 500;
}

.contact-status {
  color: #555;
  font-size: 0.625rem;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.no-contact {
  color: #444;
  font-size: 0.6875rem;
}
</style>