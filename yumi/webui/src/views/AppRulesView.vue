<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Bridge } from '@/utils/bridge';
import { getPackagesInfo } from '@/kernelsu';
import { useSchedulerStore } from '@/stores/scheduler';

const { t } = useI18n();
const store = useSchedulerStore();

const appLabelMap = ref<Record<string, string>>({});
const apps = ref<string[]>([]);
const searchText = ref('');
const showActionSheet = ref(false);
const selectedPkg = ref('');

const actions = computed(() => [
  { name: t('mode_powersave'), subname: t('desc_powersave'), color: '#10B981', modeKey: 'powersave' },
  { name: t('mode_balance'), subname: t('desc_balance'), color: '#3B82F6', modeKey: 'balance' },
  { name: t('mode_performance'), subname: t('desc_performance'), color: '#F59E0B', modeKey: 'performance' },
  { name: t('mode_fast'), subname: t('desc_fast'), color: '#EF4444', modeKey: 'fast' },
  { name: t('mode_fas'), subname: t('desc_fas'), color: '#EC4899', modeKey: 'fas' },
  { name: t('delete_rule'), color: '#EF4444', isDelete: true }
]);

const modeLabel = (modeKey: string) => {
  switch (modeKey) {
    case 'powersave': return t('mode_powersave');
    case 'balance': return t('mode_balance');
    case 'performance': return t('mode_performance');
    case 'fast': return t('mode_fast');
    case 'fas': return t('mode_fas');
    default: return modeKey;
  }
};

onMounted(async () => {
  const packages = await Bridge.getInstalledApps();
  apps.value = packages;
  try {
    const infos = getPackagesInfo(packages);
    infos.forEach(info => {
      appLabelMap.value[info.packageName] = info.appLabel;
    });
  } catch (e) { /* 降级显示包名 */ }
  await store.initData();
});

const filteredApps = computed(() => {
  const q = searchText.value.toLowerCase();
  if (!q) return apps.value;
  return apps.value.filter(pkg =>
    pkg.toLowerCase().includes(q) ||
    (appLabelMap.value[pkg] || '').toLowerCase().includes(q)
  );
});

const getLabel = (pkg: string) => appLabelMap.value[pkg] || pkg;

const openMenu = (pkg: string) => {
  selectedPkg.value = pkg;
  showActionSheet.value = true;
};

const onSelectAction = async (item: any) => {
  showActionSheet.value = false;
  if (item.isDelete) {
    delete store.appRules[selectedPkg.value];
    await Bridge.saveAppRule(selectedPkg.value, '');
  } else {
    store.appRules[selectedPkg.value] = item.modeKey;
    await Bridge.saveAppRule(selectedPkg.value, item.modeKey);
  }
};
</script>

<template>
  <div class="app-rules">
    <van-nav-bar :title="t('app_management')" left-arrow @click-left="$router.back()" fixed placeholder />

    <div class="search-wrap">
      <van-search v-model="searchText" :placeholder="t('search_apps')" shape="round" />
    </div>

    <van-list>
      <div v-for="pkg in filteredApps" :key="pkg" class="app-item glass-card" @click="openMenu(pkg)">
        <img
          :src="`ksu://icon/${pkg}`"
          class="app-icon"
          loading="lazy"
        />
        <div class="app-info">
          <div class="app-name">{{ getLabel(pkg) }}</div>
          <div class="app-pkg">{{ pkg }}</div>
        </div>
        <van-tag v-if="store.appRules[pkg]" :color="actions.find(a => a.modeKey === store.appRules[pkg])?.color || '#3B82F6'" size="medium" round>
          {{ modeLabel(store.appRules[pkg]) }}
        </van-tag>
        <span v-else class="no-rule">{{ t('not_configured') }}</span>
      </div>
    </van-list>

    <van-action-sheet
      v-model:show="showActionSheet"
      :actions="actions"
      :description="`${t('select_mode_for')} ${getLabel(selectedPkg)}`"
      :cancel-text="t('cancel')"
      @select="onSelectAction"
    />
  </div>
</template>

<style scoped>
.app-rules {
  min-height: 100vh;
}

.search-wrap {
  padding: 8px var(--space-md);
  background: transparent;
}
.search-wrap :deep(.van-search) {
  background: transparent;
  padding: 0;
}
.search-wrap :deep(.van-search__content) {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border: 1px solid var(--glass-border);
}
.search-wrap :deep(.van-field__control) {
  color: var(--text-primary);
}

.app-item {
  display: flex;
  align-items: center;
  margin: 0 var(--space-md) var(--space-sm);
  padding: 12px 14px;
  cursor: pointer;
}

.app-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  margin-right: 12px;
  flex-shrink: 0;
}

.app-info {
  flex: 1;
  min-width: 0;
}

.app-name {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.app-pkg {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.no-rule {
  font-size: 12px;
  color: var(--text-muted);
}
</style>
