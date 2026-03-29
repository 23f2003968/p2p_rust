<script setup>
import { ref } from 'vue';

defineProps({
  addresses: Array,
});

const copiedIndex = ref(-1);

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
</script>

<template>
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
          {{ copiedIndex === index ? '[O]' : 'CP' }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.addresses-section {
  padding: 0.375rem 0.75rem;
  padding-bottom: 2rem;
  background: #0d0d0d;
  border-top: 1px solid #1a1a1a;
}

.section-title {
  font-weight: 700;
  color: #555;
  margin-bottom: 0.25rem;
  font-size: 0.625rem;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.addresses-list {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.address-item {
  display: flex;
  gap: 0.375rem;
  align-items: center;
}

.address {
  flex: 1;
  font-family: inherit;
  font-size: 0.5625rem;
  color: #00ff41;
  background: #0a0a0a;
  padding: 0.25rem 0.375rem;
  border-radius: 2px;
  overflow-x: auto;
  border: 1px solid #1a1a1a;
  white-space: nowrap;
}

.copy-btn {
  padding: 0.1875rem 0.375rem;
  font-size: 0.5625rem;
  font-weight: 700;
  font-family: inherit;
  background: transparent;
  color: #555;
  border: 1px solid #333;
  border-radius: 2px;
  cursor: pointer;
  transition: all 0.1s ease;
  white-space: nowrap;
  text-transform: uppercase;
}

.copy-btn:hover {
  color: #00ff41;
  border-color: #00ff41;
}

.copy-btn.copied {
  background: #002200;
  color: #00ff41;
  border-color: #00ff41;
}
</style>