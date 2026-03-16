<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { theme } from "ant-design-vue";
import { useTheme } from "./composables/useTheme";

const route = useRoute();
const router = useRouter();
const { themeMode, toggleTheme } = useTheme();

const antTheme = computed(() => ({
  algorithm:
    themeMode.value === "dark" ? theme.darkAlgorithm : theme.defaultAlgorithm,
}));

const activeKey = computed(() => {
  if (route.path === "/weekly") return "weekly";
  if (route.path === "/records") return "records";
  if (route.path === "/settings") return "settings";
  if (route.path === "/logs") return "logs";
  return "home";
});

function goSettings() {
  router.push("/settings");
}

function goHome() {
  router.push("/");
}

function goWeekly() {
  router.push("/weekly");
}

function goRecords() {
  router.push("/records");
}

function goLogs() {
  router.push("/logs");
}
</script>

<template>
  <a-config-provider :theme="antTheme">
    <a-layout class="app-layout">
      <a-layout-header class="app-header">
        <div class="logo" @click="goHome">日报生成器</div>
        <div class="header-tabs">
          <a-button
            type="text"
            class="header-tab"
            :class="{ active: activeKey === 'home' }"
            @click="goHome"
          >
            本周工作
          </a-button>
          <a-button
            type="text"
            class="header-tab"
            :class="{ active: activeKey === 'weekly' }"
            @click="goWeekly"
          >
            周报管理
          </a-button>
          <a-button
            type="text"
            class="header-tab"
            :class="{ active: activeKey === 'records' }"
            @click="goRecords"
          >
            工作记录
          </a-button>
          <a-button
            type="text"
            class="header-tab"
            :class="{ active: activeKey === 'settings' }"
            @click="goSettings"
          >
            配置
          </a-button>
          <a-button
            type="text"
            class="header-tab"
            :class="{ active: activeKey === 'logs' }"
            @click="goLogs"
          >
            日志
          </a-button>
        </div>
        <div class="header-right">
          <a-button type="text" class="theme-toggle" @click="toggleTheme">
            {{ themeMode === 'dark' ? '☀' : '☾' }}
          </a-button>
        </div>
      </a-layout-header>

      <a-layout-content class="app-content">
        <router-view />
      </a-layout-content>
    </a-layout>
  </a-config-provider>
</template>

<style>
:root {
  --bg-page: #f5f5f5;
  --bg-section: #fafafa;
  --bg-section-border: #e8e8e8;
  --text-primary: #333;
  --text-secondary: #666;
  --text-muted: #999;
  --text-placeholder: #bbb;
  --header-bg: #ffffff;
  --header-shadow: rgba(0, 0, 0, 0.08);
  --header-text: #333;
  --header-tab-bg: rgba(0, 0, 0, 0.04);
  --header-tab-hover: rgba(0, 0, 0, 0.08);
  --header-tab-active: rgba(22, 119, 255, 0.1);
  --header-tab-active-text: #1677ff;
}

html[data-theme="dark"] {
  --bg-page: #141414;
  --bg-section: #1f1f1f;
  --bg-section-border: #303030;
  --text-primary: #e8e8e8;
  --text-secondary: #a0a0a0;
  --text-muted: #6b6b6b;
  --text-placeholder: #4a4a4a;
  --header-bg: #1f1f1f;
  --header-shadow: rgba(0, 0, 0, 0.45);
  --header-text: #e8e8e8;
  --header-tab-bg: rgba(255, 255, 255, 0.08);
  --header-tab-hover: rgba(255, 255, 255, 0.14);
  --header-tab-active: rgba(22, 119, 255, 0.25);
  --header-tab-active-text: #4096ff;
}

html[data-theme="dark"] body {
  background: var(--bg-page);
  color: var(--text-primary);
}

.app-layout {
  min-height: 100vh;
  background: var(--bg-page) !important;
}

.app-header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 24px;
  background: var(--header-bg) !important;
  box-shadow: 0 1px 4px var(--header-shadow);
  border-bottom: 1px solid var(--bg-section-border);
}

.logo {
  color: var(--header-text);
  font-size: 18px;
  font-weight: 600;
  cursor: pointer;
}

.header-tabs {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}

.header-tab {
  color: var(--header-text) !important;
  font-weight: 500;
  padding: 0 14px;
  border-radius: 8px;
  background: var(--header-tab-bg);
}

.header-tab:hover,
.header-tab:focus {
  color: var(--header-text) !important;
  background: var(--header-tab-hover) !important;
}

.header-tab.active {
  color: var(--header-tab-active-text) !important;
  background: var(--header-tab-active) !important;
  font-weight: 600;
}

.header-right {
  display: flex;
  align-items: center;
}

.theme-toggle {
  color: var(--header-text) !important;
  font-size: 18px;
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: var(--header-tab-bg);
}

.theme-toggle:hover {
  background: var(--header-tab-hover) !important;
}

.app-content {
  padding: 24px;
  background: var(--bg-page);
}
</style>
