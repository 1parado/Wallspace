<script setup lang="ts">
import { useUiStore } from '../../stores/ui';

const ui = useUiStore();
</script>

<template>
  <div class="toasts">
    <TransitionGroup name="toast">
      <div v-for="t in ui.toasts" :key="t.id" class="toast" :class="t.kind">
        <span class="dot" />
        <span class="msg">{{ t.message }}</span>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toasts {
  position: fixed;
  right: 20px;
  bottom: 20px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 300;
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: center;
  gap: 10px;
  max-width: 380px;
  padding: 10px 16px 10px 14px;
  border-radius: 12px;
  font-size: 13px;
  color: var(--text-1);
  background: var(--glass-strong);
  backdrop-filter: blur(24px) saturate(1.2);
  -webkit-backdrop-filter: blur(24px) saturate(1.2);
  border: 1px solid var(--stroke-strong);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.35);
}

.dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--text-1);
}

.toast.error .dot {
  background: var(--accent-heart);
}

.toast-enter-active {
  transition:
    opacity var(--dur-2) var(--ease-out),
    transform var(--dur-2) var(--ease-out);
}

.toast-leave-active {
  transition:
    opacity var(--dur-1) var(--ease-out),
    transform var(--dur-1) var(--ease-out);
}

.toast-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateY(6px);
}
</style>
