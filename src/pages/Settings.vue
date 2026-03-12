<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { message } from "ant-design-vue";

interface JiraConfig {
  base_url: string;
  email: string;
  api_token: string;
  username: string;
}

interface GitLabConfig {
  base_url: string;
  private_token: string;
  username: string;
  user_id: string;
}

interface AppConfig {
  jira: JiraConfig;
  gitlab: GitLabConfig;
  user_email: string;
  model: {
    base_url: string;
    api_key: string;
    model: string;
  };
  prompts: {
    polish_system: string;
    polish_few_shot: string;
    summary_system: string;
  };
}

const config = ref<AppConfig>({
  jira: { base_url: "", email: "", api_token: "", username: "" },
  gitlab: { base_url: "", private_token: "", username: "", user_id: "" },
  user_email: "",
  model: { base_url: "", api_key: "", model: "" },
  prompts: { polish_system: "", polish_few_shot: "", summary_system: "" },
});
const loading = ref(false);

onMounted(async () => {
  loading.value = true;
  try {
    const result = await invoke<AppConfig>("load_config");
    if (result) {
      config.value = {
        ...result,
        prompts: result.prompts || { polish_system: "", polish_few_shot: "", summary_system: "" },
      };
    }
  } catch (e) {
    message.error(`加载配置失败: ${e}`);
  } finally {
    loading.value = false;
  }
});

async function saveConfig() {
  if (!config.value.jira.base_url || !config.value.gitlab.base_url || !config.value.user_email) {
    message.warning("请填写完整配置");
    return;
  }
  loading.value = true;
  try {
    await invoke("save_config", { config: config.value });
    message.success("配置已保存");
  } catch (e) {
    message.error(`保存失败: ${e}`);
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="settings-container">
    <div class="settings-grid">
      <a-card title="Jira 配置" class="config-card">
        <a-form layout="vertical" :model="config" class="config-form">
          <a-form-item label="Jira URL">
            <a-input v-model:value="config.jira.base_url" placeholder="https://your-company.atlassian.net" />
          </a-form-item>
          <a-form-item label="邮箱">
            <a-input v-model:value="config.jira.email" placeholder="your@email.com" />
          </a-form-item>
          <a-form-item label="API Token">
            <a-input-password v-model:value="config.jira.api_token" placeholder="Jira API Token" />
          </a-form-item>
          <a-form-item label="Jira 用户名">
            <a-input v-model:value="config.jira.username" placeholder="例如: xuyunjie" />
          </a-form-item>
        </a-form>
      </a-card>

      <a-card title="GitLab 配置" class="config-card">
        <a-form layout="vertical" :model="config" class="config-form">
          <a-form-item label="GitLab URL">
            <a-input v-model:value="config.gitlab.base_url" placeholder="https://gitlab.com" />
          </a-form-item>
          <a-form-item label="Private Token">
            <a-input-password v-model:value="config.gitlab.private_token" placeholder="GitLab Access Token" />
          </a-form-item>
          <a-form-item label="GitLab 用户名">
            <a-input v-model:value="config.gitlab.username" placeholder="例如: xuyunjie" />
          </a-form-item>
          <a-form-item label="GitLab 用户 ID">
            <a-input v-model:value="config.gitlab.user_id" placeholder="例如: 123456" />
          </a-form-item>
        </a-form>
      </a-card>

      <a-card title="通用配置" class="config-card">
        <a-form layout="vertical" :model="config" class="config-form">
          <a-form-item label="用户邮箱">
            <a-input v-model:value="config.user_email" placeholder="用于查询提交记录的邮箱" />
          </a-form-item>
        </a-form>
      </a-card>

      <a-card title="模型配置" class="config-card">
        <a-form layout="vertical" :model="config" class="config-form">
          <a-form-item label="Base URL">
            <a-input v-model:value="config.model.base_url" placeholder="https://api.example.com" />
          </a-form-item>
          <a-form-item label="API Key">
            <a-input-password v-model:value="config.model.api_key" placeholder="sk-..." />
          </a-form-item>
          <a-form-item label="Model">
            <a-input v-model:value="config.model.model" placeholder="gpt-4o-mini" />
          </a-form-item>
        </a-form>
      </a-card>

      <a-card title="提示词配置" class="config-card prompts-card">
        <a-form layout="vertical" :model="config">
          <a-form-item>
            <template #label>
              <span>AI润色 System Prompt</span>
              <span class="prompt-hint">留空则使用默认提示词</span>
            </template>
            <a-textarea
              v-model:value="config.prompts.polish_system"
              :rows="6"
              placeholder="留空使用默认：你是日报助手。请将输入信息整合为可直接填日报的中文要点..."
            />
          </a-form-item>
          <a-form-item>
            <template #label>
              <span>AI润色 Few-shot 示例</span>
              <span class="prompt-hint">留空则使用默认示例</span>
            </template>
            <a-textarea
              v-model:value="config.prompts.polish_few_shot"
              :rows="8"
              placeholder="留空使用默认示例（包含输入/输出示范）"
            />
          </a-form-item>
          <a-form-item>
            <template #label>
              <span>周总结 System Prompt</span>
              <span class="prompt-hint">留空则使用默认提示词</span>
            </template>
            <a-textarea
              v-model:value="config.prompts.summary_system"
              :rows="6"
              placeholder="留空使用默认：你是工作总结助手。请将本周的工作内容整合为一段精炼的周总结..."
            />
          </a-form-item>
        </a-form>
      </a-card>
    </div>

    <div class="save-actions">
      <a-button type="primary" @click="saveConfig" :loading="loading">保存配置</a-button>
    </div>
  </div>
</template>

<style>
.settings-container {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
  gap: 16px;
}

.config-card {
  width: 100%;
}

.config-form {
  max-width: 520px;
}

.prompts-card {
  grid-column: 1 / -1;
}

.prompt-hint {
  margin-left: 8px;
  font-size: 12px;
  color: #999;
  font-weight: normal;
}

.save-actions {
  display: flex;
  justify-content: flex-end;
}
</style>
