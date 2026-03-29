<script setup>
import { ref, nextTick } from 'vue';

defineProps({
  inputMessage: String,
  isInitialized: Boolean,
  onSendMessage: Function,
});

const emit = defineEmits(['update:inputMessage']);
const inputRef = ref(null);

function handleSend() {
  // Call send, then refocus the input to keep keyboard open
  // We do NOT blur first — this prevents keyboard close/reopen
  if (inputRef.value) {
    inputRef.value.focus();
  }
}
</script>

<template>
  <div class="input-area">
    <div class="input-row">
      <textarea
        ref="inputRef"
        :value="inputMessage"
        @input="emit('update:inputMessage', $event.target.value)"
        @keyup.enter.exact="onSendMessage"
        placeholder="Type a message..."
        class="message-input"
        :disabled="!isInitialized"
        rows="1"
      ></textarea>
      <button
        @mousedown.prevent="onSendMessage(); handleSend()"
        :disabled="!isInitialized || !inputMessage?.trim()"
        class="send-button"
      >
        &gt;
      </button>
    </div>
  </div>
</template>

<style scoped>
.input-area {
  padding: 0.375rem 0.75rem;
  background: #111;
  border-top: 1px solid #1a1a1a;
  flex-shrink: 0;
  padding-bottom: 32px;
}

.input-row {
  display: flex;
  gap: 0.375rem;
  align-items: flex-end;
}

.message-input {
  flex: 1;
  padding: 0.375rem 0.5rem;
  font-size: 0.8125rem;
  border: 1px solid #1a1a1a;
  border-radius: 2px;
  background: #0a0a0a;
  color: #c0c0c0;
  font-family: inherit;
  resize: none;
  transition: all 0.1s ease;
  line-height: 1.4;
}

.message-input:focus {
  outline: none;
  border-color: #00ff41;
}

.message-input:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.message-input::placeholder {
  color: #333;
}

.send-button {
  width: 32px;
  height: 32px;
  font-size: 0.875rem;
  font-weight: 700;
  font-family: inherit;
  border: 1px solid #00ff41;
  border-radius: 2px;
  cursor: pointer;
  transition: all 0.1s ease;
  background: transparent;
  color: #00ff41;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.send-button:hover:not(:disabled) {
  background: #00ff41;
  color: #000;
}

.send-button:disabled {
  opacity: 0.3;
  cursor: not-allowed;
  border-color: #333;
  color: #333;
}
</style>