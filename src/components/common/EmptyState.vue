<script setup lang="ts">
defineProps<{
  title: string;
  subtitle: string;
  actionLabel?: string;
}>();

const emit = defineEmits<{ action: [] }>();
</script>

<template>
  <div class="empty">
    <p class="empty-title">{{ title }}</p>
    <p class="empty-subtitle">{{ subtitle }}</p>
    <button v-if="actionLabel" class="btn-ghost" @click="emit('action')">
      {{ actionLabel }}
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="m12 5 7 7-7 7"/></svg>
    </button>
  </div>
</template>

<style scoped>
.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 96px 24px;
  text-align: center;
}

.empty-title {
  font-size: 22px;
  font-weight: 600;
  letter-spacing: -0.02em;
  color: var(--text-1);
}

.empty-subtitle {
  font-size: 14px;
  color: var(--text-3);
  margin-bottom: 10px;
}
</style>
