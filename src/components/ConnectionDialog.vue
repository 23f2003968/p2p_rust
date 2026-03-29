<script setup>
defineProps({
  peerId: String,
});

const emit = defineEmits(['accept', 'reject']);

function handleAccept() {
  console.log('[ConnectionDialog] Accept clicked');
  emit('accept');
}

function handleReject() {
  console.log('[ConnectionDialog] Reject clicked');
  emit('reject');
}
</script>

<template>
  <div class="modal-overlay" @click="handleReject">
    <div class="modal" @click.stop>
      <div class="modal-header">
        <h3>Incoming Connection</h3>
      </div>
      
      <div class="modal-content">
        <p class="description">A peer wants to connect with you</p>
        
        <div class="peer-id-box">
          <span class="peer-id-label">Peer ID:</span>
          <code class="peer-id-value">{{ peerId }}</code>
        </div>

        <p class="question">Do you want to accept this connection?</p>
      </div>

      <div class="modal-buttons">
        <button 
          @click="handleAccept" 
          class="btn-accept"
        >
          <span>✓</span>
          Accept
        </button>
        <button 
          @click="handleReject" 
          class="btn-reject"
        >
          <span>✕</span>
          Reject
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.8);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  animation: fadeIn 0.2s ease;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

.modal {
  background: #2a2a2a;
  border: 1px solid #3a3a3a;
  border-radius: 12px;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.6);
  overflow: hidden;
  animation: slideUp 0.3s ease;
  max-width: 400px;
  width: 90%;
}

@keyframes slideUp {
  from {
    transform: translateY(20px);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}

.modal-header {
  padding: 1.5rem 1.5rem 1rem;
  border-bottom: 1px solid #3a3a3a;
  background: #1e1e1e;
}

.modal-header h3 {
  margin: 0;
  color: #ffffff;
  font-size: 1.25rem;
  font-weight: 600;
  letter-spacing: -0.015em;
}

.modal-content {
  padding: 1.5rem;
}

.description {
  margin: 0 0 1.5rem 0;
  color: #a0a0a0;
  font-size: 0.875rem;
  line-height: 1.5;
}

.peer-id-box {
  background: rgba(96, 165, 250, 0.1);
  border: 1px solid #60a5fa;
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.peer-id-label {
  color: #60a5fa;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.peer-id-value {
  color: #f0f0f0;
  font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, monospace;
  font-size: 0.75rem;
  word-break: break-all;
  line-height: 1.4;
}

.error-message {
  color: #ff6b6b;
  font-size: 0.75rem;
  padding: 0.75rem;
  background: rgba(255, 107, 107, 0.1);
  border: 1px solid rgba(255, 107, 107, 0.3);
  border-radius: 4px;
  margin-bottom: 1rem;
}

.question {
  margin: 0;
  color: #ffffff;
  font-size: 0.875rem;
  font-weight: 500;
  text-align: center;
}

.modal-buttons {
  display: flex;
  gap: 0.75rem;
  padding: 1.5rem;
  background: #1e1e1e;
  border-top: 1px solid #3a3a3a;
}

.btn-accept,
.btn-reject {
  flex: 1;
  padding: 0.75rem 1rem;
  font-size: 0.875rem;
  font-weight: 600;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  white-space: nowrap;
}

.btn-accept {
  background: #10b981;
  color: #ffffff;
}

.btn-accept:hover:not(:disabled) {
  background: #059669;
}

.btn-accept:active:not(:disabled) {
  background: #047857;
}

.btn-accept:disabled {
  background: #059669;
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-reject {
  background: #ef4444;
  color: #ffffff;
}

.btn-reject:hover:not(:disabled) {
  background: #dc2626;
}

.btn-reject:active:not(:disabled) {
  background: #b91c1c;
}

.btn-reject:disabled {
  background: #dc2626;
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-accept span,
.btn-reject span {
  font-size: 1rem;
  font-weight: 700;
}

.spinner {
  display: inline-block;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}
</style>
