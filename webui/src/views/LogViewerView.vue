<script setup lang="ts">
import { ref, onMounted, nextTick, computed } from 'vue';
import { Bridge } from '@/utils/bridge';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const logContent = ref('');
const loading = ref(false);
const terminalBody = ref<HTMLElement | null>(null);

const fetchLog = async () => {
  loading.value = true;
  try {
    const text = await Bridge.getDaemonLog();
    logContent.value = text || '';
    await nextTick();
    if (terminalBody.value) {
      terminalBody.value.scrollTop = terminalBody.value.scrollHeight;
    }
  } finally {
    loading.value = false;
  }
};

const formattedLog = computed(() => {
  if (!logContent.value) return `<div class="log-empty">${t('log_empty')}</div>`;
  return logContent.value.split('\n').map(line => {
    if (!line.trim()) return '';
    let html = line.replace(/\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\]/g, match => `<span class="log-time">${match}</span>`);
    html = html.replace(/\[INFO\]/g, `<span class="log-info">[INFO]</span>`);
    html = html.replace(/\[WARN\]/g, `<span class="log-warn">[WARN]</span>`);
    html = html.replace(/\[ERROR\]/g, `<span class="log-error">[ERROR]</span>`);
    html = html.replace(/\[(yumi[^\]]*|Scheduler|AppDetect|Screen|Boot)\]/g, match => `<span class="log-tag">${match}</span>`);
    return `<div class="log-line">${html}</div>`;
  }).join('');
});

onMounted(() => { fetchLog(); });
</script>

<template>
  <div class="log-viewer">
    <van-nav-bar
      :title="t('view_log')"
      left-arrow
      @click-left="$router.back()"
      fixed
      placeholder
      z-index="100"
    >
      <template #right>
        <van-icon name="replay" size="18" @click="fetchLog" />
      </template>
    </van-nav-bar>

    <van-loading v-if="loading && !logContent" class="loading-center" vertical>{{ t('loading') }}</van-loading>

    <div v-else class="terminal-card glass-card">
      <div class="terminal-header">
        <div class="mac-buttons">
          <span class="btn close"></span>
          <span class="btn minimize"></span>
          <span class="btn maximize"></span>
        </div>
        <div class="terminal-title">{{ t('log_terminal_title', { file: 'daemon.log', shell: 'bash' }) }}</div>
      </div>

      <div class="terminal-body" ref="terminalBody">
        <div class="log-container" v-html="formattedLog"></div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.log-viewer {
  min-height: 100vh;
  padding-bottom: 20px;
}

.loading-center {
  padding-top: 100px;
}

.terminal-card {
  margin: 16px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  height: calc(100vh - 100px);
  background: rgba(245, 247, 250, 0.85);
}

.terminal-header {
  background: rgba(0, 0, 0, 0.03);
  height: 36px;
  display: flex;
  align-items: center;
  padding: 0 16px;
  position: relative;
  border-bottom: 1px solid var(--glass-border);
}

.mac-buttons {
  display: flex;
  gap: 8px;
}

.mac-buttons .btn {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  display: inline-block;
}

.mac-buttons .close { background-color: #ff5f56; }
.mac-buttons .minimize { background-color: #ffbd2e; }
.mac-buttons .maximize { background-color: #27c93f; }

.terminal-title {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
  color: var(--text-muted);
  font-size: 13px;
  font-family: 'Inter', sans-serif;
  font-weight: 500;
}

.terminal-body {
  flex: 1;
  padding: 12px 16px;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(0, 0, 0, 0.12) transparent;
}

.terminal-body::-webkit-scrollbar {
  width: 4px;
}
.terminal-body::-webkit-scrollbar-track {
  background: transparent;
}
.terminal-body::-webkit-scrollbar-thumb {
  background-color: rgba(0, 0, 0, 0.1);
  border-radius: 2px;
}

.log-container {
  font-family: 'JetBrains Mono', 'Fira Code', 'Courier New', Consolas, monospace;
  font-size: 12px;
  line-height: 1.6;
  color: rgba(0, 0, 0, 0.7);
  word-wrap: break-word;
  white-space: pre-wrap;
}

:deep(.log-empty) { color: var(--text-muted); font-style: italic; }
:deep(.log-line) { margin-bottom: 2px; }
:deep(.log-time) { color: #059669; }
:deep(.log-info) { color: #2563EB; font-weight: bold; }
:deep(.log-warn) { color: #D97706; font-weight: bold; }
:deep(.log-error) { color: #DC2626; font-weight: bold; }
:deep(.log-tag) { color: #7C3AED; }
</style>
