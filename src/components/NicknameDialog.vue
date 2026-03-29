<script setup>
import { ref } from 'vue';

const props = defineProps({
  peerId: String,
  defaultNickname: String,
  onConfirm: Function,
  onCancel: Function,
});

const nickname = ref(props.defaultNickname || '');

const handleConfirm = () => {
  if (nickname.value.trim()) {
    props.onConfirm(nickname.value.trim());
  }
};

const handleCancel = () => {
  if (props.onCancel) {
    props.onCancel();
  }
};
</script>

<template>
  <div class="modal-overlay" @click="handleCancel">
    <div class="modal" @click.stop>
      <h3>Set Nickname</h3>
      <p class="peer-id">Peer: <code>{{ peerId }}</code></p>
      <input
        v-model="nickname"
        type="text"
        placeholder="Enter nickname..."
        class="nickname-input"
        autofocus
        @keyup.enter="handleConfirm"
        @keyup.esc="handleCancel"
      />
      <p class="hint">Leave empty to use the default nickname</p>
      <div class="modal-buttons">
        <button class="btn-primary" @click="handleConfirm">Confirm</button>
        <button class="btn-secondary" @click="handleCancel">Cancel</button>
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
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  animation: fadeIn 0.15s ease;
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
  padding: 1.5rem;
  border-radius: 12px;
  min-width: 400px;
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

.peer-id {
  margin: 0 0 1rem 0;
  color: #a0a0a0;
  font-size: 0.8125rem;
  line-height: 1.5;
}

.peer-id code {
  background: #1e1e1e;
  color: #60a5fa;
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-family: 'SF Mono', Monaco, 'Cascadia Code', 'Roboto Mono', Consolas, monospace;
  font-size: 0.75rem;
  word-break: break-all;
}

.nickname-input {
  width: 100%;
  padding: 0.625rem 0.75rem;
  font-size: 0.875rem;
  border: 1px solid #3a3a3a;
  border-radius: 6px;
  background: #1e1e1e;
  color: #f0f0f0;
  margin-bottom: 0.5rem;
  font-family: inherit;
  transition: all 0.15s ease;
  box-sizing: border-box;
}

.nickname-input:focus {
  outline: none;
  border-color: #60a5fa;
  box-shadow: 0 0 0 3px rgba(96, 165, 250, 0.25);
}

.nickname-input::placeholder {
  color: #808080;
}

.hint {
  margin: 0 0 1rem 0;
  color: #808080;
  font-size: 0.75rem;
}

.modal-buttons {
  display: flex;
  gap: 0.75rem;
  justify-content: flex-end;
}

.btn-primary {
  background: #00a884;
  color: #ffffff;
  border: 1px solid rgba(255, 255, 255, 0.1);
  padding: 0.5rem 1.25rem;
  font-size: 0.875rem;
  font-weight: 600;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.btn-primary:hover {
  background: #00c896;
}

.btn-primary:active {
  background: #008c6f;
}

.btn-secondary {
  background: transparent;
  color: #f0f0f0;
  border: 1px solid #3a3a3a;
  padding: 0.5rem 1.25rem;
  font-size: 0.875rem;
  font-weight: 600;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s ease;
  white-space: nowrap;
}

.btn-secondary:hover {
  background: #353535;
  border-color: #404040;
}

.btn-secondary:active {
  background: #2a2a2a;
}
</style>