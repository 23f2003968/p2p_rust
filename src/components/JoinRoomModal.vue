<script setup>
defineProps({
  roomInput: String,
  onJoinRoom: Function,
  onClose: Function,
});

const emit = defineEmits(['update:roomInput']);
</script>

<template>
  <div class="modal-overlay" @click="onClose">
    <div class="modal" @click.stop>
      <h3>Join Room</h3>
      <input
        :value="roomInput"
        @input="emit('update:roomInput', $event.target.value)"
        @keyup.enter="onJoinRoom"
        @keyup.esc="onClose"
        placeholder="room name..."
        autofocus
        class="modal-input"
      />
      <div class="modal-buttons">
        <button @click="onJoinRoom" class="btn-primary">JOIN</button>
        <button @click="onClose" class="btn-secondary">CANCEL</button>
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
  background: rgba(0, 0, 0, 0.85);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 1rem;
}

.modal {
  background: #0d0d0d;
  padding: 1rem;
  border-radius: 2px;
  width: 100%;
  max-width: 400px;
  border: 1px solid #00ff41;
}

.modal h3 {
  margin: 0 0 0.75rem 0;
  color: #00ff41;
  font-size: 0.8125rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  font-family: inherit;
}

.modal-input {
  width: 100%;
  padding: 0.375rem 0.5rem;
  font-size: 0.75rem;
  border: 1px solid #1a1a1a;
  border-radius: 2px;
  background: #0a0a0a;
  color: #c0c0c0;
  margin-bottom: 0.75rem;
  font-family: inherit;
  transition: all 0.1s ease;
}

.modal-input:focus {
  outline: none;
  border-color: #00ff41;
}

.modal-buttons {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
}

.btn-primary {
  background: transparent;
  color: #00ff41;
  border: 1px solid #00ff41;
  padding: 0.375rem 0.75rem;
  font-size: 0.6875rem;
  font-weight: 700;
  font-family: inherit;
  border-radius: 2px;
  cursor: pointer;
  transition: all 0.1s ease;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.btn-primary:hover {
  background: #00ff41;
  color: #000;
}

.btn-secondary {
  background: transparent;
  color: #555;
  border: 1px solid #333;
  padding: 0.375rem 0.75rem;
  font-size: 0.6875rem;
  font-weight: 700;
  font-family: inherit;
  border-radius: 2px;
  cursor: pointer;
  transition: all 0.1s ease;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.btn-secondary:hover {
  color: #c0c0c0;
  border-color: #555;
}
</style>