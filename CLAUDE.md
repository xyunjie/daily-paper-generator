# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 常用命令

- 前端开发（Vite）:
  - `pnpm dev`
- Tauri 桌面开发（同时启动 Vite + Rust 后端）:
  - `pnpm tauri dev`
- 构建前端:
  - `pnpm build`
- 构建桌面应用:
  - `pnpm tauri build`

## 高层架构

- **前端（Vue 3 + Ant Design Vue）**
  - 入口：`src/main.ts` 挂载 `App.vue` 和路由。
  - 路由：`src/router/index.ts`，主要页面：
    - `Home.vue`：本周卡片 + 动态表单新增 + 自动获取流程。
    - `Records.vue`：按天聚合的分页表格（支持关键字 + 日期范围筛选）。
    - `Settings.vue`：Jira/GitLab/通用/模型配置卡片。
  - 本地数据库：`src/db/index.ts` 封装 `tauri-plugin-sql`（SQLite）。表结构：
    - `work_items(work_date, content, created_at)`；支持按周查询、按日期替换、按天聚合分页。

- **后端（Tauri Rust）**
  - 入口：`src-tauri/src/lib.rs` 注册命令与插件（`opener`, `sql`）。
  - 配置：`src-tauri/src/config.rs` 读写应用配置目录下的 JSON。
  - 外部集成：
    - `jira.rs`：按日期 JQL 查询。
    - `gitlab.rs`：按项目 + 作者邮箱查询提交。
  - 自动获取管道：
    - `fetch.rs`：合并 Jira/GitLab 数据；若配置了模型则调用 OpenAI 兼容接口拆分与润色，否则本地拆分。
    - `llm.rs`：OpenAI 兼容 `/v1/chat/completions` 请求/响应解析。
  - 报告生成：`report.rs` 使用 `docx-rs` 生成 `.docx`。

- **数据流摘要**
  - 首页“自动获取”调用 `fetch_daily_items`，返回拆分/润色后的条目并写入 SQLite。
  - 首页“保存”会覆盖该日期的全部条目（先删后插）。
  - 工作记录页按天聚合（`GROUP_CONCAT`），分页以“天”为单位。
