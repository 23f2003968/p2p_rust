<script setup>
import { ref, watch } from 'vue';

const props = defineProps({
  currentEvent: String,
});

const brailleFrames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const frameIndex = ref(0);
let interval = null;

watch(() => props.currentEvent, (val) => {
  if (val) {
    if (!interval) {
      interval = setInterval(() => {
        frameIndex.value = (frameIndex.value + 1) % brailleFrames.length;
      }, 80);
    }
  } else {
    if (interval) {
      clearInterval(interval);
      interval = null;
    }
  }
}, { immediate: true });
</script>

<template>
  <div class="loading-bar-wrapper" v-if="currentEvent">
    <div class="loading-bar">
      <span class="braille">{{ brailleFrames[frameIndex] }}</span>
      <span class="event-text">{{ currentEvent }}</span>
    </div>
  </div>
</template>

<style scoped>
.loading-bar-wrapper {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  background: #0d0d0d;
  border-top: 1px solid #1a1a1a;
  z-index: 100;
  padding-bottom: env(safe-area-inset-bottom, 0px);
}

.loading-bar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 8px;
  min-height: 22px;
}

.braille {
  color: #00ff41;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
  width: 12px;
  text-align: center;
}

.event-text {
  color: #00ff41;
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
  overflow: hidden;
  font-family: inherit;
}
</style>
