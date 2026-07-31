<script setup lang="ts">
import { onMounted, computed } from 'vue';
import { useSchedulerStore } from '@/stores/scheduler';
import { useI18n } from 'vue-i18n';
import { toast } from '@/kernelsu';

const store = useSchedulerStore();
const { t, locale } = useI18n();

const toggleLanguage = () => {
  const newLang = locale.value === 'zh' ? 'en' : 'zh';
  locale.value = newLang;
  localStorage.setItem('app_lang', newLang);
};

const modes = computed(() => [
  { key: 'powersave', name: t('mode_powersave'), desc: t('desc_powersave'), icon: 'shield-o', color: 'var(--accent-green)', glow: 'var(--glow-green)' },
  { key: 'balance', name: t('mode_balance'), desc: t('desc_balance'), icon: 'balance-o', color: 'var(--accent-blue)', glow: 'var(--glow-blue)' },
  { key: 'performance', name: t('mode_performance'), desc: t('desc_performance'), icon: 'fire', color: 'var(--accent-orange)', glow: 'var(--glow-orange)' },
  { key: 'fast', name: t('mode_fast'), desc: t('desc_fast'), icon: 'upgrade', color: 'var(--accent-red)', glow: 'var(--glow-red)' },
]);

onMounted(() => {
  store.initData();
});

const handleModeSelect = async (modeKey: string) => {
  await store.switchMode(modeKey);
};

const copyQQGroup = async () => {
  try {
    await navigator.clipboard.writeText('1036909137');
    toast(t('copied'));
  } catch (err) {
    toast(t('qq_copy_fallback', { qq: '1036909137' }));
  }
};

const currentModeMeta = computed(() => modes.value.find(m => m.key === store.currentMode));
</script>

<template>
  <div class="home-container">

    <!-- 顶栏 -->
    <div class="top-header">
      <div class="lang-btn glass" @click="toggleLanguage">
        <van-icon name="exchange" size="14" />
        <span>{{ locale === 'zh' ? t('lang_en_short') : t('lang_zh_short') }}</span>
      </div>
    </div>

    <!-- 欢迎卡片 -->
    <div class="welcome-card glass-card shimmer-border">
      <div class="welcome-content">
        <h2>{{ t('welcome') }}</h2>
        <van-icon name="smile-o" size="36" color="rgba(255,255,255,0.5)" />
      </div>
    </div>

    <!-- 状态卡片 -->
    <div class="header-cards">
      <div class="status-card glass-card fade-in-up" style="animation-delay: 0.05s">
        <div class="status-indicator" :style="{ background: store.isDaemonRunning ? 'var(--accent-green)' : 'var(--text-muted)' }"></div>
        <van-icon :name="store.isDaemonRunning ? 'checked' : 'warning-o'" size="28" :color="store.isDaemonRunning ? 'var(--accent-green)' : 'var(--text-muted)'" />
        <div class="info">
          <h2>yumi</h2>
          <p>{{ store.isDaemonRunning ? t('daemon_running') : t('daemon_stopped') }}</p>
        </div>
      </div>

      <div class="status-card glass-card fade-in-up" style="animation-delay: 0.1s">
        <div class="status-indicator" :style="{ background: currentModeMeta?.color || 'var(--accent-blue)' }"></div>
        <van-icon :name="currentModeMeta?.icon || 'balance-o'" size="28" :color="currentModeMeta?.color || 'var(--accent-blue)'" />
        <div class="info">
          <h2>{{ currentModeMeta?.name || t('unknown_mode') }}</h2>
          <p>{{ t('current_status') }}</p>
        </div>
      </div>
    </div>

    <!-- 模式选择 -->
    <div class="section-title">{{ t('global_mode') }}</div>
    <van-grid :column-num="2" :gutter="12" :border="false" class="mode-grid">
      <van-grid-item v-for="mode in modes" :key="mode.key">
        <div
          class="mode-card glass-card"
          :class="{ 'is-active pulse-glow': store.currentMode === mode.key }"
          :style="{
            '--glow-color': mode.glow,
            borderColor: store.currentMode === mode.key ? mode.color : undefined,
          }"
          @click="handleModeSelect(mode.key)"
        >
          <van-icon
            :name="mode.icon"
            size="24"
            :color="store.currentMode === mode.key ? '#fff' : mode.color"
          />
          <span class="mode-name">{{ mode.name }}</span>
          <span class="mode-desc">{{ mode.desc }}</span>
        </div>
      </van-grid-item>
    </van-grid>

    <!-- 关于 -->
    <div class="section-title">{{ t('about') }}</div>
    <div class="about-card glass-card">
      <van-cell-group inset :border="false">
        <van-cell
          :title="t('qq_group')"
          value="103609137"
          icon="qq"
          clickable
          @click="copyQQGroup"
        />
        <van-cell
          :title="t('tg_group')"
          :value="t('click_to_join')"
          icon="chat-o"
          is-link
          url="https://t.me/+gp4adLJAsXYzMjc1"
        />
        <van-cell
          :title="t('github_repo')"
          :value="t('click_to_view')"
          icon="cluster-o"
          is-link
          url="https://github.com/imacte/yumi"
        />
      </van-cell-group>
    </div>

    <!-- 功能菜单 -->
    <div class="section-title">{{ t('more_features') }}</div>
    <div class="grid-menu">
      <van-grid clickable :column-num="3" :gutter="12" :border="false">
        <van-grid-item icon="apps-o" :text="t('app_management')" to="/apps" />
        <van-grid-item icon="setting-o" :text="t('detailed_config')" to="/config" />
        <van-grid-item icon="notes-o" :text="t('view_log')" to="/log" />
      </van-grid>
    </div>

  </div>
</template>

<style scoped>
.home-container {
  padding-bottom: 50px;
  min-height: 100vh;
}

/* ---- 顶栏 ---- */
.top-header {
  display: flex;
  justify-content: flex-end;
  padding: var(--space-md);
}
.lang-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  cursor: pointer;
}
.lang-btn:active {
  background: var(--glass-bg-active);
}

/* ---- 欢迎卡片 ---- */
.welcome-card {
  margin: 0 var(--space-md) var(--space-sm);
  padding: 24px 20px;
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.15), rgba(139, 92, 246, 0.15));
}
.welcome-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.welcome-content h2 {
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary);
  letter-spacing: 0.5px;
}

/* ---- 状态卡片 ---- */
.header-cards {
  display: flex;
  gap: 12px;
  margin: var(--space-md);
}
.status-card {
  flex: 1;
  position: relative;
  overflow: hidden;
  padding: 16px 12px 16px 16px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
}
.status-card .info {
  margin-top: 8px;
}
.status-card h2 {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
}
.status-card p {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-secondary);
}

/* ---- 模式选择 ---- */
:deep(.van-grid-item__content) {
  padding: 0 !important;
  background-color: transparent !important;
}

.mode-card {
  width: 100%;
  height: 96px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all var(--transition-slow);
}
.mode-card:active {
  transform: scale(0.95);
}
.mode-card.is-active {
  background: rgba(255, 255, 255, 0.12);
  transform: translateY(-2px);
}
.mode-name {
  margin-top: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}
.mode-desc {
  margin-top: 4px;
  font-size: 11px;
  color: var(--text-secondary);
}

/* ---- 关于卡片 ---- */
.about-card {
  margin: 0 var(--space-md);
  overflow: hidden;
}
.about-card :deep(.van-cell-group--inset) {
  margin: 0;
  background: transparent;
}
.about-card :deep(.van-cell) {
  background: transparent;
  color: var(--text-primary);
}
.about-card :deep(.van-cell__value) {
  color: var(--text-secondary);
}

/* ---- 通用 ---- */
.section-title {
  margin: 20px var(--space-md) 10px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

/* ---- 功能菜单玻璃化 ---- */
.grid-menu :deep(.van-grid-item__content) {
  background: var(--glass-bg) !important;
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
  border-radius: var(--glass-radius-sm);
  transition: all var(--transition-normal);
}
.grid-menu :deep(.van-grid-item__content):active {
  background: var(--glass-bg-hover) !important;
}
.grid-menu :deep(.van-grid-item__icon) {
  color: var(--text-secondary);
}
.grid-menu :deep(.van-grid-item__text) {
  color: var(--text-secondary);
  margin-top: 6px;
}
</style>
