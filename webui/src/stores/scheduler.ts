// src/stores/scheduler.ts
import { defineStore } from 'pinia';
import { Bridge } from '@/utils/bridge';

export const useSchedulerStore = defineStore('scheduler', {
  state: () => ({
    currentMode: 'balance',
    appRules: {} as Record<string, string>,
    isDaemonRunning: false, // 必须有这个初始状态
    gpuState: { available: false, frequency: 0, model: '' } as { available: boolean; frequency: number; model: string },
    loading: false
  }),
  actions: {
    async initData() {
      this.loading = true;
      try {
        // 必须在这里同时调用三个接口
        const [mode, rules, running, gpuState] = await Promise.all([
          Bridge.getCurrentMode(),
          Bridge.getAppRules(),
          Bridge.isDaemonRunning(),
          Bridge.getGpuState()
        ]);
        this.currentMode = mode;
        this.appRules = rules;
        this.isDaemonRunning = running; // 必须有这一行赋值
        this.gpuState = gpuState;
      } finally {
        this.loading = false;
      }
    },
    async switchMode(mode: string) {
      await Bridge.setMode(mode);
      this.currentMode = mode;
    }
  }
});