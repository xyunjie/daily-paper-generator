<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";

const route = useRoute();
const router = useRouter();

const activeKey = computed(() => {
  if (route.path === "/records") return "records";
  if (route.path === "/settings") return "settings";
  return "home";
});

function goSettings() {
  router.push("/settings");
}

function goHome() {
  router.push("/");
}

function goRecords() {
  router.push("/records");
}
</script>

<template>
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
      </div>
    </a-layout-header>

    <a-layout-content class="app-content">
      <router-view />
    </a-layout-content>
  </a-layout>
</template>

<style>
.app-layout {
  min-height: 100vh;
}

.app-header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 24px;
  background: #1677ff;
  box-shadow: 0 2px 12px rgba(22, 119, 255, 0.35);
}

.logo {
  color: #ffffff;
  font-size: 18px;
  font-weight: 600;
  cursor: pointer;
}

.header-tabs {
  display: flex;
  align-items: center;
  gap: 8px;
}

.header-tab {
  color: #ffffff !important;
  font-weight: 500;
  padding: 0 14px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.12);
}

.header-tab:hover,
.header-tab:focus {
  color: #ffffff !important;
  background: rgba(255, 255, 255, 0.2) !important;
}

.header-tab.active {
  color: #ffffff !important;
  background: rgba(255, 255, 255, 0.28) !important;
}

.app-content {
  padding: 24px;
}
</style>
