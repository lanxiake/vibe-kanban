# Vibe Kanban AI Tool Orchestration Platform - 部署指南

本文档介绍如何部署 Vibe Kanban AI 工具跨设备管理平台。

## 目录

- [架构概览](#架构概览)
- [快速开始](#快速开始)
- [详细部署步骤](#详细部署步骤)
- [Agent Daemon 部署](#agent-daemon-部署)
- [验证部署](#验证部署)
- [常见问题](#常见问题)
- [运维操作](#运维操作)

---

## 架构概览

```
┌─────────────────────────────────────────────────────────────┐
│                    中心服务器 (Center)                        │
│  ┌──────────┐  ┌────────────┐  ┌────────────────────────┐   │
│  │PostgreSQL│  │ ElectricSQL│  │   Remote Server        │   │
│  │  :5432   │  │   :3000    │  │   :8081 → :3000        │   │
│  │          │  │  (内部)     │  │   (Web UI + API)       │   │
│  └──────────┘  └────────────┘  └──────────┬─────────────┘   │
└───────────────────────────────────────────┼─────────────────┘
                                            │ WebSocket
                              ┌─────────────┴──────────────┐
                              │     Agent Daemon (远程)     │
                              │  - 心跳上报                  │
                              │  - 执行器发现                │
                              │  - 任务执行 (Phase 2)        │
                              └────────────────────────────┘
```

---

## 快速开始

### 前置要求

- Docker 20.10+
- Docker Compose v2
- Git
- 2GB+ 内存
- 开放端口：3000 (HTTP)

### 一键部署

```bash
# 1. 克隆代码
git clone https://github.com/lanxiake/vibe-kanban.git
cd vibe-kanban
git checkout feature/ai-tool-orchestration-platform

# 2. 进入部署目录
cd deploy

# 3. 复制并编辑环境变量
cp .env.example .env
vim .env  # 填写必要配置

# 4. 生成密钥（可选，复制输出到 .env）
./deploy.sh secrets

# 5. 启动服务
./deploy.sh start

# 6. 访问 Web UI
# 打开浏览器访问 http://your-server-ip:3000
```

---

## 详细部署步骤

### 步骤 1: 准备环境变量

复制 `.env.example` 为 `.env` 并填写以下必填项：

| 变量 | 说明 | 示例 |
|------|------|------|
| `POSTGRES_PASSWORD` | 数据库密码 | `openssl rand -hex 16` |
| `VIBEKANBAN_REMOTE_JWT_SECRET` | JWT 密钥 (Base64, ≥32字节) | `openssl rand -base64 48` |
| `ELECTRIC_ROLE_PASSWORD` | ElectricSQL 角色密码 | `openssl rand -hex 16` |
| `SERVER_PUBLIC_BASE_URL` | 公共访问 URL | `http://192.168.1.100:3000` |
| `CENTER_WS_URL` | Agent WebSocket URL | `ws://192.168.1.100:3000/v1/agent/ws` |
| `TOKEN_HASH_SECRET` | Token 哈希密钥 | `openssl rand -hex 32` |
| `GITHUB_OAUTH_CLIENT_ID` | GitHub OAuth Client ID | 从 GitHub 获取 |
| `GITHUB_OAUTH_CLIENT_SECRET` | GitHub OAuth Secret | 从 GitHub 获取 |

### 步骤 2: 配置 OAuth

至少需要配置一个 OAuth 提供商（GitHub 或 Google）。

#### GitHub OAuth

1. 访问 https://github.com/settings/developers
2. 点击 "New OAuth App"
3. 填写信息：
   - Application name: `Vibe Kanban`
   - Homepage URL: `http://your-server:3000`
   - Authorization callback URL: `http://your-server:3000/auth/github/callback`
4. 创建后复制 Client ID 和 Client Secret 到 `.env`

#### Google OAuth (可选)

1. 访问 https://console.cloud.google.com/apis/credentials
2. 创建 OAuth 2.0 客户端 ID
3. 添加授权重定向 URI: `http://your-server:3000/auth/google/callback`
4. 复制凭据到 `.env`

### 步骤 3: 构建并启动

```bash
# 检查配置
./deploy.sh check

# 构建镜像（首次部署）
./deploy.sh build

# 启动服务
./deploy.sh start

# 查看日志
./deploy.sh logs
```

### 步骤 4: 验证服务

```bash
# 检查服务状态
./deploy.sh status

# 应该看到 3 个服务都是 healthy:
# - vibe-db (PostgreSQL)
# - vibe-electric (ElectricSQL)
# - vibe-server (Remote Server)
```

---

## Agent Daemon 部署

Agent Daemon 部署在需要管理 AI 工具的远程服务器上。

### 方式 1: 使用 Web UI 生成的 Docker 命令

1. 登录 Web UI → 进入组织 → Servers
2. 点击 "添加服务器"
3. 填写服务器信息并创建
4. 复制生成的 Docker 命令
5. 在目标服务器上执行该命令

### 方式 2: 使用 docker-compose

```bash
# 在目标服务器上
cd deploy/agent

# 复制并编辑环境变量
cp .env.example .env
vim .env  # 填写 CENTER_URL 和 AGENT_TOKEN

# 启动 Agent
docker compose up -d

# 查看日志
docker compose logs -f
```

### Agent 环境变量

| 变量 | 必填 | 说明 |
|------|------|------|
| `CENTER_URL` | ✅ | 中心服务器 WebSocket URL |
| `AGENT_TOKEN` | ✅ | 从 Web UI 获取的 Token |
| `LOG_LEVEL` | ❌ | 日志级别 (默认: info) |
| `HEARTBEAT_INTERVAL` | ❌ | 心跳间隔秒数 (默认: 30) |

---

## 验证部署

### 1. 检查中心服务器

```bash
# 健康检查
curl http://your-server:3000/health

# 应返回: {"status":"ok"}
```

### 2. 检查数据库迁移

```bash
# 查看服务器日志，确认迁移成功
docker compose logs server | grep -i migration
```

### 3. 测试完整流程

1. 访问 `http://your-server:3000`
2. 使用 GitHub/Google 登录
3. 创建组织
4. 添加服务器 → 获取 Agent Token
5. 在远程服务器部署 Agent Daemon
6. 在 Web UI 中查看服务器状态变为 "Online"

---

## 常见问题

### Q: 服务启动失败，提示数据库连接错误

检查 PostgreSQL 是否正常启动：
```bash
docker compose logs db
```

确保 `POSTGRES_PASSWORD` 在 `.env` 中正确设置。

### Q: OAuth 登录失败

1. 检查 OAuth 回调 URL 是否正确配置
2. 确保 `SERVER_PUBLIC_BASE_URL` 与实际访问地址一致
3. 检查 OAuth Client ID 和 Secret 是否正确

### Q: Agent 连接失败

1. 检查 `CENTER_WS_URL` 是否可从 Agent 服务器访问
2. 确保防火墙开放 3000 端口
3. 检查 Agent Token 是否正确
4. 查看 Agent 日志：`docker compose logs vibe-agent`

### Q: ElectricSQL 启动失败

ElectricSQL 需要等待数据库和服务器都就绪：
```bash
# 重启 ElectricSQL
docker compose restart electric
```

---

## 运维操作

### 查看日志

```bash
# 所有服务
./deploy.sh logs

# 特定服务
./deploy.sh logs server
./deploy.sh logs db
./deploy.sh logs electric
```

### 重启服务

```bash
./deploy.sh restart
```

### 更新部署

```bash
# 拉取最新代码并重新部署
./deploy.sh update
```

### 备份数据库

```bash
# 导出数据库
docker compose exec db pg_dump -U remote remote > backup.sql

# 恢复数据库
cat backup.sql | docker compose exec -T db psql -U remote remote
```

### 清理数据（危险）

```bash
# 删除所有数据和容器
./deploy.sh clean
```

---

## 端口说明

| 端口 | 服务 | 说明 |
|------|------|------|
| 3000 | Remote Server | Web UI 和 API（对外） |
| 5432 | PostgreSQL | 数据库（内部） |
| 65432 | ElectricSQL | PG Proxy（内部） |

---

## 环境变量完整列表

### 必填

| 变量 | 说明 |
|------|------|
| `POSTGRES_PASSWORD` | PostgreSQL 密码 |
| `VIBEKANBAN_REMOTE_JWT_SECRET` | JWT 签名密钥 (Base64) |
| `ELECTRIC_ROLE_PASSWORD` | ElectricSQL 角色密码 |
| `SERVER_PUBLIC_BASE_URL` | 公共访问 URL |
| `CENTER_WS_URL` | Agent WebSocket URL |
| `TOKEN_HASH_SECRET` | Agent Token 哈希密钥 |
| `GITHUB_OAUTH_CLIENT_ID` | GitHub OAuth ID |
| `GITHUB_OAUTH_CLIENT_SECRET` | GitHub OAuth Secret |

### 可选

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `RUST_LOG` | `info` | 日志级别 |
| `GOOGLE_OAUTH_CLIENT_ID` | - | Google OAuth ID |
| `GOOGLE_OAUTH_CLIENT_SECRET` | - | Google OAuth Secret |
| `LOOPS_EMAIL_API_KEY` | - | 邮件服务 API Key |
| `R2_*` | - | Cloudflare R2 存储配置 |
| `GITHUB_APP_*` | - | GitHub App 集成配置 |
| `POSTHOG_*` | - | PostHog 分析配置 |

---

## 技术支持

- GitHub Issues: https://github.com/lanxiake/vibe-kanban/issues
- 文档: https://github.com/lanxiake/vibe-kanban/tree/feature/ai-tool-orchestration-platform/docs
