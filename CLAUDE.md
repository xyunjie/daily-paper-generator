# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 常用命令

- 前端开发（Vite）:
  - `pnpm dev`
- Tauri 桌面开发（同时启动 Vite + Rust 后端）:
  - `pnpm tauri dev`
- 构建前端:
  - `pnpm build`（含 `vue-tsc --noEmit` 类型检查）
- 构建桌面应用:
  - `pnpm tauri build`
- Rust 单独编译检查:
  - `cd src-tauri && cargo check`

## 高层架构

Tauri 2 桌面应用：Vue 3 + Ant Design Vue 前端 → Tauri IPC → Rust 后端。用于从 Jira/GitLab 自动聚合每日工作内容，可选 LLM 润色后生成日报/周报。

### 前端（Vue 3 + Ant Design Vue）

- 入口：`src/main.ts` 挂载 `App.vue` 和路由（`src/router/index.ts`，hash 模式）。
- 页面：
  - `Home.vue`：本周卡片 + 动态表单新增 + 自动获取/AI润色流程。
  - `Records.vue`：按天聚合的分页表格（支持关键字 + 日期范围筛选）。
  - `Settings.vue`：Jira/GitLab/通用/模型/Prompt 配置卡片。
  - `Logs.vue`：查看应用日志文件（支持自动刷新）。
- 本地数据库：`src/db/index.ts` 封装 `tauri-plugin-sql`（SQLite `daily.db`，在 `tauri.conf.json` 中 preload）。
  - `work_items(work_date, content, source, created_at)`：按周查询、按日期替换（先删后插）、按天聚合分页（`GROUP_CONCAT`）。
  - `week_summaries(week_start, summary, updated_at)`：周总结持久化（UPSERT）。
- 日期工具：`src/utils/date.ts`，依赖 `date-fns` + `dayjs`。

### 后端（Tauri Rust · `src-tauri/`）

- 入口：`src/lib.rs` 注册命令与插件（`opener`, `sql`）。初始化时写日志到配置目录。
- **Tauri 命令**（前端通过 `invoke()` 调用）：
  - `save_config` / `load_config`：读写 `config.json`。
  - `fetch_daily_items`：从 Jira+GitLab 拉取原始条目（不经 LLM）。
  - `polish_daily_items`：对已获取条目调用 LLM 润色（失败回退本地规则）。
  - `summarize_week`：调用 LLM 生成周总结。
  - `generate_report`：生成日报 `.docx`。
  - `export_week_report`：生成周报 `.xlsx`，弹出系统保存对话框（`rfd`）。
  - `get_log_path` / `read_log_file`：日志查看。
- **配置**（`config.rs`）：`AppConfig` 持久化到 `{系统配置目录}/daily-paper-generator/config.json`。含 `JiraConfig`、`GitLabConfig`、`ModelConfig`、`PromptsConfig`（内置默认 prompt）。
- **外部集成**：
  - `jira.rs`：按日期 JQL 查询已完成任务。
  - `gitlab.rs`：两种模式——配置了 `user_id` 时走 Events API（推送事件 → 逐个获取 commit 详情），否则走 Projects membership + Commits API（遍历所有参与项目）。过滤 merge commit。
- **LLM 管道**（`llm.rs` + `fetch.rs`）：
  - `polish_with_openai`：OpenAI 兼容 `/v1/chat/completions`，system prompt + few-shot + 结构化输入。`postprocess_daily_bullets` 过滤违禁标记（Jira key、hex hash、URL、路径）。
  - `summarize_week_with_openai`：独立周总结请求。
  - 本地回退：`fetch.rs::summarize_locally` 按关键词分桶生成要点（无 LLM 时）。
- **报告生成**（`report.rs`）：
  - 日报：`docx-rs` 生成 `.docx`。
  - 周报：`rust_xlsxwriter` 生成 `.xlsx`（日期合并单元格 + 周总结行）。
  - 输出到配置目录，周报额外通过 `rfd` 弹出保存对话框。

### 数据流摘要

1. 首页"自动获取" → `fetch_daily_items`（Jira + GitLab 原始条目）→ 返回前端展示。
2. 首页"AI润色" → `polish_daily_items`（LLM 润色 / 本地回退）→ 替换前端条目。
3. 首页"保存" → 前端 `replaceWorkItems` 覆盖该日期全部条目（先删后插入 SQLite）。
4. 周报导出 → 前端汇总本周数据 + 可选"AI总结" → `export_week_report` 生成 `.xlsx`。
