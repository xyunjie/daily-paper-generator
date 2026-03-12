# 日报工具 (Daily Paper Tool)

一个基于 Tauri + Vue 3 的跨平台桌面应用，用于自动生成工作日报和周报。通过集成 Jira 和 GitLab，自动获取工作任务和代码提交记录，并借助 AI 模型生成规范化的工作内容描述。

## 功能特性

- **自动获取工作内容** - 从 Jira 获取当日完成的任务，从 GitLab 获取代码提交记录
- **AI 智能润色** - 支持 OpenAI 兼容接口，自动将原始数据转换为规范的日报要点
- **本地数据存储** - 基于 SQLite 的本地数据库，所有数据安全存储在本地
- **周报导出** - 一键导出本周工作内容为 Excel 文件
- **跨平台支持** - 支持 macOS、Windows 和 Linux

## 技术栈

- **前端**: Vue 3 + TypeScript + Ant Design Vue + Vite
- **后端**: Tauri v2 + Rust
- **数据库**: SQLite (tauri-plugin-sql)
- **文档生成**: docx-rs (Word) + rust_xlsxwriter (Excel)

## 安装

### 从 Release 下载

前往 [Releases](https://github.com/xyunjie/daily-paper-generator/releases) 页面下载对应平台的安装包：

- **macOS**: `.dmg` 文件
- **Windows**: `.msi` 或 NSIS 安装程序
- **Linux**: `.deb` 或 `.AppImage`

### 从源码构建

#### 环境要求

- Node.js 18+
- pnpm 9+
- Rust 1.70+
- 系统依赖（参考 [Tauri 官方文档](https://tauri.app/start/prerequisites/)）

#### 构建步骤

```bash
# 克隆仓库
git clone https://github.com/xyunjie/daily-paper-generator.git
cd daily-paper-generator

# 安装依赖
pnpm install

# 开发模式
pnpm tauri dev

# 构建生产版本
pnpm tauri build
```

## 配置说明

首次使用需要在「配置」页面完成以下设置：

### Jira 配置

| 字段 | 说明 | 示例 |
|------|------|------|
| Jira URL | Jira 服务器地址 | `https://your-company.atlassian.net` |
| 邮箱 | 登录邮箱 | `your@email.com` |
| API Token | Jira API Token | [获取方式](https://id.atlassian.com/manage-profile/security/api-tokens) |
| 用户名 | Jira 用户名 | `zhangsan` |

### GitLab 配置

| 字段 | 说明 | 示例 |
|------|------|------|
| GitLab URL | GitLab 服务器地址 | `https://gitlab.com` |
| Private Token | 访问令牌 | [创建 Token](https://docs.gitlab.com/ee/user/profile/personal_access_tokens.html) |
| 用户名 | GitLab 用户名 | `zhangsan` |
| 用户 ID | GitLab 用户 ID（可选） | `123456` |

> **提示**: 如果填写了用户 ID，将使用 Events API 获取提交记录，速度更快；否则使用项目遍历方式。

### 通用配置

| 字段 | 说明 |
|------|------|
| 用户邮箱 | 用于匹配 Git 提交记录的作者邮箱 |

### 模型配置（可选）

支持任意 OpenAI 兼容的 API 接口，用于智能润色日报内容：

| 字段 | 说明 | 示例 |
|------|------|------|
| Base URL | API 基础地址 | `https://api.openai.com` |
| API Key | API 密钥 | `sk-...` |
| Model | 模型名称 | `gpt-4o-mini` |

> **注意**: 如果不配置模型，系统会使用本地规则算法生成日报内容。

## 使用指南

### 本周工作

首页展示本周（周一至周日）的工作内容卡片，每个卡片底部有三个操作按钮：

1. **自动获取** - 从 Jira 和 GitLab 拉取当日原始数据，并标注来源（Jira/GitLab）
2. **AI润色** - 对已获取的数据进行智能润色，生成规范的工作要点
3. **编辑** - 手动编辑当日工作内容

**使用流程**：
- 点击「自动获取」按钮获取原始数据（会显示来源标签）
- 如需AI优化，点击「AI润色」按钮（需配置模型）
- 可随时点击「编辑」按钮手动调整内容
- 点击「导出本周工作内容」生成 Excel 文件

### 自动获取逻辑

**自动获取**：
- 从 Jira 获取当日完成的任务
- 从 GitLab 获取代码提交记录
- 保留原始内容，并标注来源（Jira/GitLab）

**AI润色**（需配置模型）：
- 合并 Jira 任务和 GitLab 提交信息
- 去除 Jira Key、项目路径、提交 Hash 等技术细节
- 生成 3-8 条规范的中文工作要点

### 工作记录

- 按天聚合显示历史工作记录
- 支持关键字搜索
- 支持日期范围筛选
- 分页浏览

## 项目结构

```
daily-paper-tool/
├── src/                    # Vue 前端源码
│   ├── pages/              # 页面组件
│   │   ├── Home.vue        # 本周工作
│   │   ├── Records.vue     # 工作记录
│   │   └── Settings.vue    # 配置页面
│   ├── db/                 # 数据库操作
│   ├── router/             # 路由配置
│   └── utils/              # 工具函数
├── src-tauri/              # Tauri Rust 后端
│   └── src/
│       ├── lib.rs          # 入口
│       ├── config.rs       # 配置管理
│       ├── jira.rs         # Jira API 集成
│       ├── gitlab.rs       # GitLab API 集成
│       ├── fetch.rs        # 数据获取与整合
│       ├── llm.rs          # AI 模型调用
│       └── report.rs       # 报告生成
└── .github/workflows/      # GitHub Actions
    └── release.yml         # 自动构建发布
```

## 开发

### 常用命令

```bash
# 前端开发
pnpm dev

# Tauri 开发（前端 + Rust）
pnpm tauri dev

# 构建前端
pnpm build

# 构建桌面应用
pnpm tauri build
```

### 数据库表结构

```sql
CREATE TABLE work_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    work_date TEXT NOT NULL,      -- 工作日期 YYYY-MM-DD
    content TEXT NOT NULL,        -- 工作内容
    source TEXT DEFAULT 'manual', -- 来源: jira/gitlab/manual
    created_at TEXT NOT NULL      -- 创建时间
);
```

## 发布流程

项目使用 GitHub Actions 自动构建和发布：

1. 更新版本号（`package.json` 和 `tauri.conf.json`）
2. 创建并推送 tag：
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. GitHub Actions 自动构建三个平台的安装包
4. 构建完成后，在 Releases 页面编辑并发布草稿

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
