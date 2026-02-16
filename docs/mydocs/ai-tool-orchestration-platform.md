# AI 工具跨设备管理平台 - 架构设计文档

## 1. 项目愿景

将 Vibe Kanban 改造为一个 **AI 编码工具跨设备管理与协作平台**，支持：

- 跨设备管理 AI 编码工具（Claude Code、Gemini CLI、Codex 等）
- 将任务部署到不同服务器执行
- 人机协作的任务分配
- AI 任务执行的细粒度可观测性
- 统一的配置管理（MCP、Skills、Agents、模型）

---

## 2. 核心设计决策

| 决策点 | 选择 | 理由 |
|-------|------|------|
| 服务器连接方式 | Agent 守护进程为主 + SSH 兜底 | 实时性好、可观测性强、网络友好 |
| 通信协议 | WebSocket | 复用现有基础设施，开发效率高 |
| Agent 部署方式 | Docker 容器 | 隔离性好、易于更新、可打包依赖 |
| 用户权限模型 | 简单角色（Admin / Member） | 够用就行，降低复杂度 |

---

## 3. 实现优先级

```
Phase 1: 服务器管理 + Agent Daemon 基础
         ↓
Phase 2: 任务调度与分配（人机协作）
         ↓
Phase 3: 可观测性（工具调用事件流）
         ↓
Phase 4: 配置下发（MCP/Skill/Agent）
```

---

## 4. 系统架构

### 4.1 整体架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    中心控制面板 (Center)                      │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌──────────┐  │
│  │  Web UI   │  │  API 层   │  │ 调度引擎   │  │ 配置管理  │  │
│  │ (React)   │  │  (Axum)   │  │           │  │          │  │
│  └───────────┘  └───────────┘  └───────────┘  └──────────┘  │
│         ▲              ▲              ▲                      │
└─────────┼──────────────┼──────────────┼──────────────────────┘
          │              │              │
          │         WebSocket      SSH (降级)
          │              │              │
┌─────────┴──────────────┴──────────────┴──────────────────────┐
│                   Agent Daemon (每台服务器)                    │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐              │
│  │  心跳上报   │  │  执行器管理  │  │  事件采集   │              │
│  │  状态监控   │  │ (executors) │  │ (可观测性)  │              │
│  └────────────┘  └────────────┘  └────────────┘              │
│  ┌────────────┐  ┌────────────┐                              │
│  │  配置接收   │  │  SSH 回退   │                              │
│  │  配置应用   │  │  管理端口   │                              │
│  └────────────┘  └────────────┘                              │
│                        │                                     │
│              ┌─────────┴─────────┐                           │
│              │   AI 编码工具      │                           │
│              │ Claude/Gemini/... │                           │
│              └───────────────────┘                           │
└──────────────────────────────────────────────────────────────┘
```

### 4.2 数据模型扩展

基于现有模型，新增以下实体：

```
Organization (组织/团队) [现有]
├── OrganizationMember [现有]
├── Project [现有]
│   └── Task [现有]
│       └── Workspace [现有]
│
├── Server [新增] ─────────────────┐
│   ├── id: Uuid                   │
│   ├── organization_id: Uuid      │
│   ├── name: String               │
│   ├── host: String               │
│   ├── status: ServerStatus       │  Online/Offline/Error
│   ├── agent_version: String      │
│   ├── last_heartbeat: DateTime   │
│   ├── system_info: Json          │  CPU/内存/磁盘
│   ├── ssh_config: Option<Json>   │  SSH 兜底配置
│   └── created_at: DateTime       │
│                                  │
├── ServerExecutor [新增] ─────────┤  服务器上可用的执行器
│   ├── server_id: Uuid            │
│   ├── executor_type: String      │  claude_code/gemini/codex
│   ├── version: String            │
│   ├── status: ExecutorStatus     │  Available/Busy/Error
│   └── config: Json               │
│                                  │
├── ExecutionEvent [新增] ─────────┤  可观测性事件
│   ├── id: Uuid                   │
│   ├── execution_process_id: Uuid │
│   ├── event_type: String         │
│   ├── payload: Json              │
│   ├── timestamp: DateTime        │
│   └── server_id: Uuid            │
│                                  │
└── ConfigDeployment [新增] ───────┘  配置下发记录
    ├── id: Uuid
    ├── server_id: Uuid
    ├── config_type: String          MCP/Skill/Agent/Model
    ├── config_content: Json
    ├── status: DeploymentStatus     Pending/Applied/Failed
    └── deployed_at: DateTime
```

---

## 5. Agent Daemon 设计

### 5.1 模块结构

```
crates/agent-daemon/
├── src/
│   ├── main.rs              # 入口，启动守护进程
│   ├── connection.rs        # WebSocket 连接管理（重连、心跳）
│   ├── protocol.rs          # 消息协议定义
│   ├── executor_bridge.rs   # 桥接现有 executors/ crate
│   ├── event_collector.rs   # 事件采集（工具调用、技能使用）
│   ├── config_manager.rs    # 配置接收与应用
│   ├── system_monitor.rs    # 系统资源监控（CPU/内存/磁盘）
│   └── ssh_fallback.rs      # SSH 回退管理端口
└── Cargo.toml
```

### 5.2 WebSocket 消息协议

```rust
// ===== Agent → Center (上行) =====

/// Agent 首次连接注册
struct AgentRegister {
    agent_id: Uuid,
    agent_token: String,
    hostname: String,
    agent_version: String,
    system_info: SystemInfo,
    available_executors: Vec<ExecutorInfo>,
}

/// 心跳消息
struct Heartbeat {
    timestamp: DateTime<Utc>,
    system_stats: SystemStats,      // CPU/内存/磁盘使用率
    running_tasks: Vec<Uuid>,       // 正在执行的任务
}

/// 执行事件
struct ExecutionEvent {
    execution_id: Uuid,
    event_type: ExecutionEventType, // Started/Completed/Failed
    details: Option<Value>,
}

/// 工具调用事件（细粒度可观测性）
struct ToolUseEvent {
    execution_id: Uuid,
    timestamp: DateTime<Utc>,
    tool_name: String,              // Read/Edit/Bash/WebSearch...
    parameters_summary: String,     // 参数摘要（脱敏）
    duration_ms: Option<u64>,
    result_summary: Option<String>,
}

/// 技能使用事件
struct SkillInvokeEvent {
    execution_id: Uuid,
    skill_name: String,
    args: Option<String>,
    timestamp: DateTime<Utc>,
}

/// 日志流
struct LogStream {
    execution_id: Uuid,
    log_type: LogType,              // Stdout/Stderr/Normalized
    content: String,
    timestamp: DateTime<Utc>,
}

/// 审批请求
struct ApprovalRequest {
    request_id: Uuid,
    execution_id: Uuid,
    tool_name: String,
    description: String,
    timeout_seconds: u32,
}

/// 配置应用确认
struct ConfigAck {
    deployment_id: Uuid,
    status: ConfigAckStatus,        // Applied/Failed
    error_message: Option<String>,
}

// ===== Center → Agent (下行) =====

/// 任务分配
struct TaskAssign {
    task_id: Uuid,
    workspace_config: WorkspaceConfig,
    executor_type: String,
    prompt: String,
    env: ExecutionEnv,
}

/// 任务取消
struct TaskCancel {
    task_id: Uuid,
    reason: Option<String>,
}

/// 配置下发
struct ConfigPush {
    deployment_id: Uuid,
    config_type: ConfigType,        // MCP/Skill/Agent/Model
    content: Value,
    apply_mode: ApplyMode,          // Immediate/OnNextTask/Manual
}

/// 审批响应
struct ApprovalResponse {
    request_id: Uuid,
    approved: bool,
    message: Option<String>,
}

/// 管理命令
struct Command {
    command_type: CommandType,      // Restart/Update/Shutdown
    params: Option<Value>,
}
```

### 5.3 Docker 部署

```dockerfile
# Dockerfile for Agent Daemon
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release --package agent-daemon

FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    openssh-client \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 安装 Node.js (用于部分 AI 工具)
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs

COPY --from=builder /app/target/release/agent-daemon /app/agent-daemon

# 配置目录
VOLUME ["/app/config", "/app/workspaces", "/var/log/agent"]

# 环境变量
ENV CENTER_URL=""
ENV AGENT_TOKEN=""
ENV LOG_LEVEL="info"

EXPOSE 9999

ENTRYPOINT ["/app/agent-daemon"]
```

启动命令：
```bash
docker run -d \
  --name vibe-agent \
  --restart unless-stopped \
  -e CENTER_URL=wss://your-center.com/agent/ws \
  -e AGENT_TOKEN=your-agent-token \
  -v /path/to/repos:/app/workspaces \
  -v /path/to/config:/app/config \
  -v /var/log/vibe-agent:/var/log/agent \
  vibe-agent-daemon:latest
```

---

## 6. 用户权限模型

### 6.1 角色定义

| 角色 | 权限 |
|------|------|
| **Admin** | 管理服务器、下发配置、管理成员、分配任务、查看所有任务和执行日志 |
| **Member** | 查看分配给自己的任务、执行任务、查看自己有权限的服务器状态 |

### 6.2 权限检查点

```rust
enum Permission {
    // 服务器管理
    ServerCreate,
    ServerDelete,
    ServerView,

    // 配置管理
    ConfigPush,
    ConfigView,

    // 任务管理
    TaskCreate,
    TaskAssign,
    TaskViewAll,
    TaskViewOwn,

    // 成员管理
    MemberInvite,
    MemberRemove,
    MemberRoleChange,
}

// 角色权限映射
fn get_permissions(role: Role) -> Vec<Permission> {
    match role {
        Role::Admin => vec![/* 所有权限 */],
        Role::Member => vec![
            Permission::ServerView,
            Permission::ConfigView,
            Permission::TaskCreate,
            Permission::TaskViewOwn,
        ],
    }
}
```

---

## 7. Phase 1 实现计划

### 7.1 目标

实现服务器管理 + Agent Daemon 基础，能够：
- 在中心面板注册/管理服务器
- Agent Daemon 连接到中心并保持心跳
- 查看服务器在线状态和资源使用
- 查看服务器上可用的执行器

### 7.2 任务分解

**后端 (Rust)**
1. 新增 `Server` 数据模型和迁移
2. 新增服务器管理 API（CRUD）
3. 实现 Agent WebSocket 端点
4. 创建 `agent-daemon` crate 基础结构
5. 实现心跳和状态上报

**前端 (React)**
1. 服务器列表页面
2. 添加服务器对话框
3. 服务器详情面板（状态、资源、执行器）
4. Agent 安装指引页面

### 7.3 预期产出

- 用户可以在 Web UI 添加服务器
- 用户可以在远程服务器上运行 Agent Daemon Docker 容器
- Agent 自动连接到中心并上报状态
- 用户可以在 Web UI 看到服务器在线状态和资源使用情况

---

## 8. 可复用的现有基础设施

| 模块 | 路径 | 复用方式 |
|------|------|---------|
| 执行器抽象 | `crates/executors/` | 直接在 Agent Daemon 中引用 |
| 进程管理 | `crates/local-deployment/src/container.rs` | 提取通用逻辑到 services |
| WebSocket | `crates/server/src/routes/terminal.rs` | 参考实现 Agent WS 端点 |
| 远程客户端 | `crates/services/src/services/remote_client.rs` | 参考实现 Agent 连接逻辑 |
| OAuth/认证 | `crates/remote/src/auth/` | 复用 JWT 验证逻辑 |
| 组织模型 | `crates/remote/src/db/` | 扩展添加 Server 关联 |

---

## 9. 后续 Phase 概要

### Phase 2: 任务调度与分配
- 任务分配给用户或 AI
- 任务调度到指定服务器
- 任务状态同步

### Phase 3: 可观测性
- 工具调用事件采集
- 技能使用追踪
- 执行链路可视化
- 实时日志流

### Phase 4: 配置下发
- MCP Server 配置管理
- Skills 配置管理
- Agent 人格配置
- 模型选择配置
- 批量下发和版本管理

---

## 10. 详细技术设计

### 10.1 数据库 Schema 设计

#### 10.1.1 Server 表（服务器）

```sql
-- 迁移文件: YYYYMMDDHHMMSS_create_servers_table.sql
CREATE TABLE servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    -- 基本信息
    name VARCHAR(255) NOT NULL,
    description TEXT,
    host VARCHAR(255) NOT NULL,           -- IP 或域名
    port INTEGER DEFAULT 9999,            -- Agent 端口

    -- 连接状态
    status VARCHAR(50) NOT NULL DEFAULT 'pending',  -- pending/online/offline/error
    agent_id UUID,                        -- Agent 注册后分配的 ID
    agent_version VARCHAR(50),
    last_heartbeat_at TIMESTAMPTZ,
    connection_error TEXT,                -- 最近的连接错误信息

    -- 系统信息（由 Agent 上报）
    system_info JSONB DEFAULT '{}',       -- {os, arch, cpu_cores, total_memory, ...}
    system_stats JSONB DEFAULT '{}',      -- {cpu_usage, memory_usage, disk_usage, ...}

    -- SSH 兜底配置（可选）
    ssh_enabled BOOLEAN DEFAULT false,
    ssh_host VARCHAR(255),
    ssh_port INTEGER DEFAULT 22,
    ssh_username VARCHAR(255),
    ssh_auth_type VARCHAR(50),            -- password/key
    ssh_credentials_encrypted TEXT,       -- 加密存储的凭证

    -- 元数据
    tags JSONB DEFAULT '[]',              -- 标签，用于分组和筛选
    metadata JSONB DEFAULT '{}',          -- 扩展字段

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(organization_id, name)
);

CREATE INDEX idx_servers_organization_id ON servers(organization_id);
CREATE INDEX idx_servers_status ON servers(status);
CREATE INDEX idx_servers_agent_id ON servers(agent_id);
```

#### 10.1.2 ServerExecutor 表（服务器执行器）

```sql
-- 迁移文件: YYYYMMDDHHMMSS_create_server_executors_table.sql
CREATE TABLE server_executors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,

    -- 执行器信息
    executor_type VARCHAR(100) NOT NULL,  -- claude_code/gemini_cli/codex/...
    executor_name VARCHAR(255),           -- 显示名称
    version VARCHAR(50),

    -- 状态
    status VARCHAR(50) NOT NULL DEFAULT 'unknown',  -- available/busy/error/unknown
    current_task_id UUID,                 -- 当前正在执行的任务

    -- 能力和配置
    capabilities JSONB DEFAULT '[]',      -- [session_fork, setup_helper, ...]
    config JSONB DEFAULT '{}',            -- 执行器特定配置

    -- 统计
    total_executions INTEGER DEFAULT 0,
    successful_executions INTEGER DEFAULT 0,
    failed_executions INTEGER DEFAULT 0,

    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(server_id, executor_type)
);

CREATE INDEX idx_server_executors_server_id ON server_executors(server_id);
CREATE INDEX idx_server_executors_status ON server_executors(status);
```

#### 10.1.3 ExecutionEvent 表（执行事件）

```sql
-- 迁移文件: YYYYMMDDHHMMSS_create_execution_events_table.sql
CREATE TABLE execution_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- 关联
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    execution_process_id UUID NOT NULL,   -- 关联到 execution_processes 表
    session_id UUID,                      -- 可选，关联到 sessions 表

    -- 事件信息
    event_type VARCHAR(100) NOT NULL,     -- tool_call/skill_invoke/agent_switch/...
    event_category VARCHAR(50) NOT NULL,  -- execution/observability/system

    -- 事件数据
    payload JSONB NOT NULL DEFAULT '{}',

    -- 时间
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    duration_ms INTEGER,                  -- 事件持续时间（如工具调用）

    -- 索引优化字段
    tool_name VARCHAR(100),               -- 冗余存储，便于查询

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 分区表（按时间分区，便于清理历史数据）
-- CREATE TABLE execution_events_YYYYMM PARTITION OF execution_events
--     FOR VALUES FROM ('YYYY-MM-01') TO ('YYYY-MM+1-01');

CREATE INDEX idx_execution_events_server_id ON execution_events(server_id);
CREATE INDEX idx_execution_events_execution_process_id ON execution_events(execution_process_id);
CREATE INDEX idx_execution_events_event_type ON execution_events(event_type);
CREATE INDEX idx_execution_events_timestamp ON execution_events(timestamp DESC);
CREATE INDEX idx_execution_events_tool_name ON execution_events(tool_name) WHERE tool_name IS NOT NULL;
```

#### 10.1.4 ConfigDeployment 表（配置下发）

```sql
-- 迁移文件: YYYYMMDDHHMMSS_create_config_deployments_table.sql
CREATE TABLE config_deployments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- 关联
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    server_id UUID REFERENCES servers(id) ON DELETE CASCADE,  -- NULL 表示全局配置

    -- 配置信息
    config_type VARCHAR(50) NOT NULL,     -- mcp_servers/skills/agents/models
    config_name VARCHAR(255) NOT NULL,    -- 配置名称
    config_version INTEGER NOT NULL DEFAULT 1,
    config_content JSONB NOT NULL,

    -- 下发状态
    status VARCHAR(50) NOT NULL DEFAULT 'pending',  -- pending/deploying/applied/failed/rollback
    apply_mode VARCHAR(50) NOT NULL DEFAULT 'immediate',  -- immediate/on_next_task/manual

    -- 结果
    applied_at TIMESTAMPTZ,
    error_message TEXT,

    -- 审计
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_config_deployments_organization_id ON config_deployments(organization_id);
CREATE INDEX idx_config_deployments_server_id ON config_deployments(server_id);
CREATE INDEX idx_config_deployments_config_type ON config_deployments(config_type);
CREATE INDEX idx_config_deployments_status ON config_deployments(status);
```

#### 10.1.5 TaskAssignment 表（任务分配）

```sql
-- 迁移文件: YYYYMMDDHHMMSS_create_task_assignments_table.sql
CREATE TABLE task_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- 关联
    task_id UUID NOT NULL,                -- 关联到 tasks 表

    -- 分配目标（二选一）
    assignee_user_id UUID REFERENCES users(id),      -- 分配给人
    assignee_server_id UUID REFERENCES servers(id),  -- 分配给 AI（服务器）

    -- 分配信息
    assignment_type VARCHAR(50) NOT NULL, -- human/ai
    priority INTEGER DEFAULT 0,           -- 优先级

    -- 状态
    status VARCHAR(50) NOT NULL DEFAULT 'pending',  -- pending/accepted/in_progress/completed/rejected

    -- AI 执行相关
    executor_type VARCHAR(100),           -- 指定使用的执行器
    execution_config JSONB DEFAULT '{}',  -- 执行配置

    -- 时间
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,

    -- 审计
    assigned_by UUID REFERENCES users(id),

    CONSTRAINT chk_assignee CHECK (
        (assignee_user_id IS NOT NULL AND assignee_server_id IS NULL) OR
        (assignee_user_id IS NULL AND assignee_server_id IS NOT NULL)
    )
);

CREATE INDEX idx_task_assignments_task_id ON task_assignments(task_id);
CREATE INDEX idx_task_assignments_assignee_user_id ON task_assignments(assignee_user_id);
CREATE INDEX idx_task_assignments_assignee_server_id ON task_assignments(assignee_server_id);
CREATE INDEX idx_task_assignments_status ON task_assignments(status);
```

### 10.2 API 设计

#### 10.2.1 服务器管理 API

```
# 服务器 CRUD
POST   /api/v1/organizations/{org_id}/servers           # 创建服务器
GET    /api/v1/organizations/{org_id}/servers           # 列出服务器
GET    /api/v1/organizations/{org_id}/servers/{id}      # 获取服务器详情
PUT    /api/v1/organizations/{org_id}/servers/{id}      # 更新服务器
DELETE /api/v1/organizations/{org_id}/servers/{id}      # 删除服务器

# 服务器操作
POST   /api/v1/servers/{id}/regenerate-token            # 重新生成 Agent Token
POST   /api/v1/servers/{id}/test-ssh                    # 测试 SSH 连接
POST   /api/v1/servers/{id}/restart-agent               # 重启 Agent（通过 SSH）

# 服务器执行器
GET    /api/v1/servers/{id}/executors                   # 列出执行器
PUT    /api/v1/servers/{id}/executors/{type}            # 更新执行器配置

# Agent WebSocket 端点
WS     /api/v1/agent/ws                                 # Agent 连接端点
```

#### 10.2.2 请求/响应示例

```typescript
// POST /api/v1/organizations/{org_id}/servers
// 创建服务器请求
interface CreateServerRequest {
  name: string;
  description?: string;
  host: string;
  port?: number;  // 默认 9999
  tags?: string[];
  ssh_config?: {
    enabled: boolean;
    host?: string;
    port?: number;
    username?: string;
    auth_type: 'password' | 'key';
    credentials: string;  // 密码或私钥
  };
}

// 创建服务器响应
interface CreateServerResponse {
  id: string;
  name: string;
  host: string;
  port: number;
  status: 'pending';
  agent_token: string;  // 用于 Agent 连接的 Token（仅创建时返回）
  docker_command: string;  // 生成的 Docker 启动命令
  created_at: string;
}

// GET /api/v1/organizations/{org_id}/servers
// 服务器列表响应
interface ServerListResponse {
  servers: Server[];
  total: number;
}

interface Server {
  id: string;
  name: string;
  description?: string;
  host: string;
  port: number;
  status: 'pending' | 'online' | 'offline' | 'error';
  agent_version?: string;
  last_heartbeat_at?: string;
  system_info?: SystemInfo;
  system_stats?: SystemStats;
  executors: ServerExecutor[];
  tags: string[];
  created_at: string;
  updated_at: string;
}

interface SystemInfo {
  os: string;
  arch: string;
  cpu_cores: number;
  total_memory_gb: number;
  hostname: string;
}

interface SystemStats {
  cpu_usage_percent: number;
  memory_usage_percent: number;
  disk_usage_percent: number;
  running_tasks: number;
}

interface ServerExecutor {
  executor_type: string;
  executor_name: string;
  version?: string;
  status: 'available' | 'busy' | 'error' | 'unknown';
  current_task_id?: string;
  capabilities: string[];
}
```

### 10.3 WebSocket 协议详细设计

#### 10.3.1 消息封装格式

```typescript
// 所有 WebSocket 消息的统一封装
interface WsMessage {
  id: string;           // 消息 ID，用于请求-响应匹配
  type: string;         // 消息类型
  timestamp: string;    // ISO 8601 时间戳
  payload: unknown;     // 具体消息内容
}

// 错误响应
interface WsError {
  id: string;
  type: 'error';
  timestamp: string;
  payload: {
    code: string;
    message: string;
    details?: unknown;
  };
}
```

#### 10.3.2 Agent → Center 消息详细定义

```typescript
// 1. Agent 注册
interface AgentRegisterMessage {
  type: 'agent_register';
  payload: {
    agent_token: string;        // 服务器创建时生成的 Token
    hostname: string;
    agent_version: string;
    system_info: {
      os: string;               // linux/darwin/windows
      arch: string;             // x86_64/aarch64
      cpu_cores: number;
      total_memory_gb: number;
      disk_total_gb: number;
    };
    available_executors: {
      executor_type: string;
      version: string;
      capabilities: string[];
      config_path?: string;     // MCP 配置路径
    }[];
  };
}

// 注册响应
interface AgentRegisterResponse {
  type: 'agent_register_ack';
  payload: {
    agent_id: string;           // 分配的 Agent ID
    server_id: string;          // 关联的服务器 ID
    heartbeat_interval_ms: number;  // 心跳间隔
    pending_configs: ConfigPush[];  // 待应用的配置
  };
}

// 2. 心跳
interface HeartbeatMessage {
  type: 'heartbeat';
  payload: {
    system_stats: {
      cpu_usage_percent: number;
      memory_usage_percent: number;
      memory_available_gb: number;
      disk_usage_percent: number;
      disk_available_gb: number;
      load_average: [number, number, number];  // 1/5/15 分钟
    };
    running_executions: {
      execution_id: string;
      task_id: string;
      executor_type: string;
      started_at: string;
      status: string;
    }[];
    executor_statuses: {
      executor_type: string;
      status: 'available' | 'busy' | 'error';
      error_message?: string;
    }[];
  };
}

// 3. 执行状态事件
interface ExecutionStatusMessage {
  type: 'execution_status';
  payload: {
    execution_id: string;
    task_id: string;
    status: 'started' | 'running' | 'completed' | 'failed' | 'cancelled';
    executor_type: string;
    details?: {
      exit_code?: number;
      error_message?: string;
      duration_ms?: number;
      output_summary?: string;
    };
  };
}

// 4. 工具调用事件（可观测性核心）
interface ToolUseMessage {
  type: 'tool_use';
  payload: {
    execution_id: string;
    event_id: string;
    tool_name: string;          // Read/Edit/Bash/Grep/WebSearch/Task/...
    tool_category: 'file' | 'search' | 'execute' | 'web' | 'agent' | 'other';
    parameters: {
      summary: string;          // 参数摘要（脱敏后）
      file_path?: string;       // 文件操作时的路径
      command?: string;         // Bash 命令（脱敏后）
      query?: string;           // 搜索查询
    };
    started_at: string;
    completed_at?: string;
    duration_ms?: number;
    result: {
      success: boolean;
      summary?: string;         // 结果摘要
      error?: string;
    };
  };
}

// 5. 技能调用事件
interface SkillInvokeMessage {
  type: 'skill_invoke';
  payload: {
    execution_id: string;
    event_id: string;
    skill_name: string;         // commit/review-pr/tdd/...
    args?: string;
    started_at: string;
    completed_at?: string;
    result?: {
      success: boolean;
      summary?: string;
    };
  };
}

// 6. Agent 切换事件
interface AgentSwitchMessage {
  type: 'agent_switch';
  payload: {
    execution_id: string;
    event_id: string;
    from_agent?: string;
    to_agent: string;
    reason?: string;
    timestamp: string;
  };
}

// 7. 上下文使用事件
interface ContextUsageMessage {
  type: 'context_usage';
  payload: {
    execution_id: string;
    tokens_used: number;
    tokens_limit: number;
    usage_percent: number;
    warning_level?: 'normal' | 'high' | 'critical';
  };
}

// 8. 日志流
interface LogStreamMessage {
  type: 'log_stream';
  payload: {
    execution_id: string;
    log_type: 'stdout' | 'stderr' | 'normalized' | 'system';
    content: string;
    timestamp: string;
  };
}

// 9. 审批请求
interface ApprovalRequestMessage {
  type: 'approval_request';
  payload: {
    request_id: string;
    execution_id: string;
    tool_name: string;
    action_description: string;
    risk_level: 'low' | 'medium' | 'high';
    details: {
      command?: string;
      file_path?: string;
      changes_preview?: string;
    };
    timeout_seconds: number;
    requested_at: string;
  };
}

// 10. 配置应用确认
interface ConfigAckMessage {
  type: 'config_ack';
  payload: {
    deployment_id: string;
    status: 'applied' | 'failed' | 'skipped';
    applied_at?: string;
    error_message?: string;
    affected_executors?: string[];
  };
}
```

#### 10.3.3 Center → Agent 消息详细定义

```typescript
// 1. 任务分配
interface TaskAssignMessage {
  type: 'task_assign';
  payload: {
    assignment_id: string;
    task_id: string;
    task_title: string;
    task_description?: string;

    // 执行配置
    executor_type: string;
    prompt: string;

    // 工作区配置
    workspace: {
      repos: {
        url: string;
        branch: string;
        target_branch?: string;
        working_dir?: string;
      }[];
      env_vars?: Record<string, string>;
    };

    // 执行选项
    options: {
      auto_commit: boolean;
      run_setup_scripts: boolean;
      run_cleanup_scripts: boolean;
      timeout_minutes?: number;
    };
  };
}

// 2. 任务取消
interface TaskCancelMessage {
  type: 'task_cancel';
  payload: {
    task_id: string;
    execution_id?: string;
    reason?: string;
    force: boolean;           // 是否强制终止
  };
}

// 3. 配置下发
interface ConfigPushMessage {
  type: 'config_push';
  payload: {
    deployment_id: string;
    config_type: 'mcp_servers' | 'skills' | 'agents' | 'models' | 'executor_config';
    config_name: string;
    config_version: number;
    content: unknown;         // 具体配置内容
    apply_mode: 'immediate' | 'on_next_task' | 'manual';
    target_executors?: string[];  // 空表示所有执行器
  };
}

// 4. 审批响应
interface ApprovalResponseMessage {
  type: 'approval_response';
  payload: {
    request_id: string;
    approved: boolean;
    message?: string;
    responded_by?: string;    // 响应者用户 ID
    responded_at: string;
  };
}

// 5. 管理命令
interface CommandMessage {
  type: 'command';
  payload: {
    command_id: string;
    command_type: 'restart' | 'update' | 'shutdown' | 'refresh_executors' | 'clear_cache';
    params?: Record<string, unknown>;
    timeout_seconds?: number;
  };
}

// 命令响应
interface CommandResponseMessage {
  type: 'command_response';
  payload: {
    command_id: string;
    success: boolean;
    message?: string;
    result?: unknown;
  };
}
```

### 10.4 Agent Daemon 详细设计

#### 10.4.1 核心模块职责

```
┌─────────────────────────────────────────────────────────────────┐
│                      Agent Daemon 架构                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │   main.rs   │───▶│ connection  │◀──▶│  protocol   │         │
│  │  (入口点)    │    │  (连接管理)  │    │  (消息解析)  │         │
│  └─────────────┘    └──────┬──────┘    └─────────────┘         │
│                            │                                    │
│         ┌──────────────────┼──────────────────┐                │
│         ▼                  ▼                  ▼                │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │   system    │    │  executor   │    │   config    │         │
│  │  monitor    │    │   bridge    │    │  manager    │         │
│  │ (系统监控)   │    │ (执行器桥接) │    │ (配置管理)   │         │
│  └─────────────┘    └──────┬──────┘    └─────────────┘         │
│                            │                                    │
│                            ▼                                    │
│                     ┌─────────────┐                            │
│                     │   event     │                            │
│                     │ collector   │                            │
│                     │ (事件采集)   │                            │
│                     └─────────────┘                            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### 10.4.2 连接管理 (connection.rs)

```rust
/// 连接管理器 - 负责 WebSocket 连接的建立、维护和重连
pub struct ConnectionManager {
    center_url: String,
    agent_token: String,
    reconnect_config: ReconnectConfig,
    state: Arc<RwLock<ConnectionState>>,
}

/// 重连配置
pub struct ReconnectConfig {
    pub initial_delay_ms: u64,      // 初始重连延迟: 1000ms
    pub max_delay_ms: u64,          // 最大重连延迟: 60000ms
    pub backoff_multiplier: f64,    // 退避乘数: 2.0
    pub max_retries: Option<u32>,   // 最大重试次数: None (无限)
    pub jitter_percent: u8,         // 抖动百分比: 20%
}

/// 连接状态
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected {
        agent_id: Uuid,
        server_id: Uuid,
        connected_at: DateTime<Utc>,
    },
    Reconnecting {
        attempt: u32,
        next_retry_at: DateTime<Utc>,
    },
}

impl ConnectionManager {
    /// 启动连接（阻塞，内部处理重连）
    pub async fn run(&self, message_handler: impl MessageHandler) -> Result<(), AgentError>;

    /// 发送消息到 Center
    pub async fn send(&self, message: WsMessage) -> Result<(), AgentError>;

    /// 获取当前连接状态
    pub fn state(&self) -> ConnectionState;

    /// 主动断开连接
    pub async fn disconnect(&self);
}

/// 消息处理器 trait
pub trait MessageHandler: Send + Sync {
    async fn handle(&self, message: WsMessage) -> Option<WsMessage>;
}
```

#### 10.4.3 执行器桥接 (executor_bridge.rs)

```rust
/// 执行器桥接 - 复用现有 executors crate，适配远程执行场景
pub struct ExecutorBridge {
    executors: HashMap<String, Box<dyn StandardCodingAgentExecutor>>,
    running_executions: Arc<RwLock<HashMap<Uuid, RunningExecution>>>,
    event_sender: mpsc::Sender<ExecutionEvent>,
}

/// 运行中的执行
pub struct RunningExecution {
    pub execution_id: Uuid,
    pub task_id: Uuid,
    pub executor_type: String,
    pub started_at: DateTime<Utc>,
    pub cancel_token: CancellationToken,
    pub child_handle: Option<AsyncGroupChild>,
}

impl ExecutorBridge {
    /// 发现可用的执行器
    pub async fn discover_executors(&self) -> Vec<ExecutorInfo>;

    /// 启动任务执行
    pub async fn start_execution(
        &self,
        task: TaskAssignMessage,
    ) -> Result<Uuid, ExecutorError>;

    /// 取消执行
    pub async fn cancel_execution(
        &self,
        execution_id: Uuid,
        force: bool,
    ) -> Result<(), ExecutorError>;

    /// 获取执行状态
    pub fn get_execution_status(&self, execution_id: Uuid) -> Option<ExecutionStatus>;

    /// 获取所有运行中的执行
    pub fn running_executions(&self) -> Vec<RunningExecution>;
}
```

#### 10.4.4 事件采集 (event_collector.rs)

```rust
/// 事件采集器 - 从执行器输出中提取可观测性事件
pub struct EventCollector {
    event_sender: mpsc::Sender<ExecutionEvent>,
    parsers: Vec<Box<dyn EventParser>>,
}

/// 事件解析器 trait
pub trait EventParser: Send + Sync {
    /// 解析日志行，提取事件
    fn parse(&self, line: &str, context: &ParseContext) -> Option<ExecutionEvent>;

    /// 支持的执行器类型
    fn supported_executors(&self) -> Vec<String>;
}

/// Claude Code 事件解析器
pub struct ClaudeCodeEventParser;

impl EventParser for ClaudeCodeEventParser {
    fn parse(&self, line: &str, context: &ParseContext) -> Option<ExecutionEvent> {
        // 解析 Claude Code 的 JSON Patch 输出
        // 提取工具调用、技能使用、Agent 切换等事件
    }
}

/// 解析上下文
pub struct ParseContext {
    pub execution_id: Uuid,
    pub executor_type: String,
    pub current_tool: Option<String>,
    pub tool_start_time: Option<DateTime<Utc>>,
}

/// 执行事件类型
pub enum ExecutionEvent {
    ToolUse(ToolUseEvent),
    SkillInvoke(SkillInvokeEvent),
    AgentSwitch(AgentSwitchEvent),
    ContextUsage(ContextUsageEvent),
    Log(LogEvent),
    Status(StatusEvent),
}
```

#### 10.4.5 配置管理 (config_manager.rs)

```rust
/// 配置管理器 - 接收和应用中心下发的配置
pub struct ConfigManager {
    config_dir: PathBuf,
    pending_configs: Arc<RwLock<Vec<PendingConfig>>>,
}

/// 待应用的配置
pub struct PendingConfig {
    pub deployment_id: Uuid,
    pub config_type: ConfigType,
    pub content: Value,
    pub apply_mode: ApplyMode,
    pub received_at: DateTime<Utc>,
}

/// 配置类型
pub enum ConfigType {
    McpServers,      // MCP Server 配置
    Skills,          // 技能配置
    Agents,          // Agent 人格配置
    Models,          // 模型选择配置
    ExecutorConfig,  // 执行器特定配置
}

/// 应用模式
pub enum ApplyMode {
    Immediate,       // 立即应用
    OnNextTask,      // 下次任务时应用
    Manual,          // 手动应用
}

impl ConfigManager {
    /// 接收配置
    pub async fn receive_config(&self, config: ConfigPushMessage) -> Result<(), ConfigError>;

    /// 应用配置
    pub async fn apply_config(&self, deployment_id: Uuid) -> Result<ConfigAckMessage, ConfigError>;

    /// 应用所有待应用的配置
    pub async fn apply_pending_configs(&self) -> Vec<ConfigAckMessage>;

    /// 获取当前配置
    pub fn get_current_config(&self, config_type: ConfigType) -> Option<Value>;
}

/// MCP Server 配置结构
#[derive(Serialize, Deserialize)]
pub struct McpServerConfig {
    pub mcpServers: HashMap<String, McpServer>,
}

#[derive(Serialize, Deserialize)]
pub struct McpServer {
    pub command: String,
    pub args: Vec<String>,
    pub env: Option<HashMap<String, String>>,
}
```

#### 10.4.6 系统监控 (system_monitor.rs)

```rust
/// 系统监控器 - 采集系统资源使用情况
pub struct SystemMonitor {
    interval: Duration,
    stats: Arc<RwLock<SystemStats>>,
}

/// 系统统计信息
#[derive(Clone, Serialize)]
pub struct SystemStats {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub memory_available_gb: f64,
    pub disk_usage_percent: f64,
    pub disk_available_gb: f64,
    pub load_average: [f64; 3],
    pub collected_at: DateTime<Utc>,
}

/// 系统信息（静态）
#[derive(Clone, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_cores: u32,
    pub total_memory_gb: f64,
    pub disk_total_gb: f64,
    pub hostname: String,
}

impl SystemMonitor {
    /// 启动监控（后台任务）
    pub fn start(&self) -> JoinHandle<()>;

    /// 获取当前统计信息
    pub fn current_stats(&self) -> SystemStats;

    /// 获取系统信息
    pub fn system_info(&self) -> SystemInfo;
}
```

### 10.5 前端页面设计

#### 10.5.1 页面结构

```
frontend/src/pages/
├── servers/
│   ├── ServersPage.tsx           # 服务器列表页
│   ├── ServerDetailPage.tsx      # 服务器详情页
│   └── AddServerDialog.tsx       # 添加服务器对话框
├── observability/
│   ├── ExecutionTimelinePage.tsx # 执行时间线页
│   ├── ToolUsagePanel.tsx        # 工具使用面板
│   └── LiveLogsPanel.tsx         # 实时日志面板
├── config/
│   ├── ConfigManagementPage.tsx  # 配置管理页
│   ├── McpConfigEditor.tsx       # MCP 配置编辑器
│   ├── SkillsConfigEditor.tsx    # 技能配置编辑器
│   └── ConfigDeployDialog.tsx    # 配置下发对话框
└── assignments/
    ├── TaskAssignmentPage.tsx    # 任务分配页
    └── AssignmentDialog.tsx      # 分配对话框
```

#### 10.5.2 服务器列表页设计

```tsx
// ServersPage.tsx 核心结构
interface ServersPageProps {}

export function ServersPage() {
  return (
    <div className="servers-page">
      {/* 顶部工具栏 */}
      <div className="toolbar">
        <h1>服务器管理</h1>
        <div className="actions">
          <SearchInput placeholder="搜索服务器..." />
          <FilterDropdown options={['全部', '在线', '离线', '错误']} />
          <Button onClick={openAddServerDialog}>
            <PlusIcon /> 添加服务器
          </Button>
        </div>
      </div>

      {/* 服务器统计卡片 */}
      <div className="stats-cards">
        <StatCard title="总服务器" value={totalServers} />
        <StatCard title="在线" value={onlineCount} color="green" />
        <StatCard title="离线" value={offlineCount} color="gray" />
        <StatCard title="运行中任务" value={runningTasks} color="blue" />
      </div>

      {/* 服务器列表 */}
      <div className="server-grid">
        {servers.map(server => (
          <ServerCard
            key={server.id}
            server={server}
            onSelect={() => navigateToDetail(server.id)}
          />
        ))}
      </div>
    </div>
  );
}

// ServerCard 组件
interface ServerCardProps {
  server: Server;
  onSelect: () => void;
}

function ServerCard({ server, onSelect }: ServerCardProps) {
  return (
    <Card onClick={onSelect} className="server-card">
      {/* 状态指示器 */}
      <StatusBadge status={server.status} />

      {/* 服务器信息 */}
      <div className="server-info">
        <h3>{server.name}</h3>
        <p className="host">{server.host}:{server.port}</p>
        <p className="version">Agent v{server.agent_version}</p>
      </div>

      {/* 资源使用 */}
      <div className="resource-usage">
        <ResourceBar label="CPU" value={server.system_stats?.cpu_usage_percent} />
        <ResourceBar label="内存" value={server.system_stats?.memory_usage_percent} />
        <ResourceBar label="磁盘" value={server.system_stats?.disk_usage_percent} />
      </div>

      {/* 执行器列表 */}
      <div className="executors">
        {server.executors.map(executor => (
          <ExecutorBadge
            key={executor.executor_type}
            executor={executor}
          />
        ))}
      </div>

      {/* 运行中任务 */}
      {server.system_stats?.running_tasks > 0 && (
        <div className="running-tasks">
          <TaskIcon /> {server.system_stats.running_tasks} 个任务运行中
        </div>
      )}
    </Card>
  );
}
```

#### 10.5.3 执行可观测性页面设计

```tsx
// ExecutionTimelinePage.tsx - 执行时间线
export function ExecutionTimelinePage({ executionId }: { executionId: string }) {
  const { events, isLoading } = useExecutionEvents(executionId);

  return (
    <div className="execution-timeline">
      {/* 执行概览 */}
      <ExecutionOverview executionId={executionId} />

      {/* 时间线视图 */}
      <div className="timeline-container">
        <Timeline>
          {events.map(event => (
            <TimelineItem key={event.id} event={event}>
              {renderEventContent(event)}
            </TimelineItem>
          ))}
        </Timeline>
      </div>

      {/* 侧边面板 */}
      <div className="side-panels">
        <ToolUsagePanel events={events} />
        <ContextUsagePanel executionId={executionId} />
      </div>
    </div>
  );
}

// 工具使用面板
function ToolUsagePanel({ events }: { events: ExecutionEvent[] }) {
  const toolStats = useMemo(() => computeToolStats(events), [events]);

  return (
    <Panel title="工具使用统计">
      {/* 工具调用分布图 */}
      <PieChart data={toolStats.distribution} />

      {/* 工具调用列表 */}
      <div className="tool-list">
        {toolStats.tools.map(tool => (
          <div key={tool.name} className="tool-item">
            <ToolIcon name={tool.name} />
            <span className="tool-name">{tool.name}</span>
            <span className="call-count">{tool.callCount} 次</span>
            <span className="avg-duration">{tool.avgDuration}ms</span>
            <SuccessRate rate={tool.successRate} />
          </div>
        ))}
      </div>
    </Panel>
  );
}

// 实时日志面板
function LiveLogsPanel({ executionId }: { executionId: string }) {
  const { logs, isConnected } = useLiveLogStream(executionId);

  return (
    <Panel title="实时日志">
      <div className="log-controls">
        <ConnectionStatus connected={isConnected} />
        <LogLevelFilter />
        <SearchInput placeholder="搜索日志..." />
      </div>

      <div className="log-viewer">
        <VirtualizedList
          items={logs}
          renderItem={(log) => (
            <LogLine
              timestamp={log.timestamp}
              level={log.level}
              content={log.content}
            />
          )}
        />
      </div>
    </Panel>
  );
}
```

#### 10.5.4 配置管理页面设计

```tsx
// ConfigManagementPage.tsx
export function ConfigManagementPage() {
  const [selectedType, setSelectedType] = useState<ConfigType>('mcp_servers');
  const [selectedServer, setSelectedServer] = useState<string | null>(null);

  return (
    <div className="config-management">
      {/* 配置类型选择 */}
      <Tabs value={selectedType} onChange={setSelectedType}>
        <Tab value="mcp_servers" label="MCP Servers" icon={<PlugIcon />} />
        <Tab value="skills" label="Skills" icon={<WandIcon />} />
        <Tab value="agents" label="Agents" icon={<BotIcon />} />
        <Tab value="models" label="Models" icon={<CpuIcon />} />
      </Tabs>

      {/* 配置编辑区 */}
      <div className="config-editor-area">
        {selectedType === 'mcp_servers' && (
          <McpConfigEditor
            serverId={selectedServer}
            onSave={handleSaveConfig}
          />
        )}
        {selectedType === 'skills' && (
          <SkillsConfigEditor serverId={selectedServer} />
        )}
        {/* ... 其他配置类型 */}
      </div>

      {/* 下发目标选择 */}
      <div className="deploy-target">
        <h3>下发目标</h3>
        <ServerSelector
          value={selectedServer}
          onChange={setSelectedServer}
          allowAll={true}
        />
        <Button onClick={handleDeploy}>
          <DeployIcon /> 下发配置
        </Button>
      </div>

      {/* 下发历史 */}
      <DeploymentHistory configType={selectedType} />
    </div>
  );
}

// MCP 配置编辑器
function McpConfigEditor({ serverId, onSave }: McpConfigEditorProps) {
  const { config, updateConfig } = useMcpConfig(serverId);

  return (
    <div className="mcp-editor">
      <div className="mcp-servers-list">
        {Object.entries(config.mcpServers).map(([name, server]) => (
          <McpServerItem
            key={name}
            name={name}
            server={server}
            onEdit={() => openEditDialog(name)}
            onDelete={() => deleteServer(name)}
          />
        ))}
        <Button variant="outline" onClick={addNewServer}>
          <PlusIcon /> 添加 MCP Server
        </Button>
      </div>

      {/* JSON 预览 */}
      <div className="json-preview">
        <CodeEditor
          language="json"
          value={JSON.stringify(config, null, 2)}
          onChange={handleJsonChange}
        />
      </div>
    </div>
  );
}
```

---

## 11. 详细实现计划

### 11.1 Phase 1: 服务器管理 + Agent Daemon 基础

#### 11.1.1 目标

建立服务器管理的基础设施，实现 Agent Daemon 的核心功能，使用户能够：
- 在 Web UI 中添加和管理服务器
- 在远程服务器上部署 Agent Daemon
- 查看服务器在线状态和资源使用情况
- 查看服务器上可用的 AI 执行器

#### 11.1.2 任务分解

**后端任务 (Rust)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P1-B1 | 创建 Server 数据模型 | 无 | `crates/db/src/models/server.rs` |
| P1-B2 | 创建 ServerExecutor 数据模型 | P1-B1 | `crates/db/src/models/server_executor.rs` |
| P1-B3 | 编写数据库迁移文件 | P1-B1, P1-B2 | `crates/db/migrations/` |
| P1-B4 | 实现服务器 CRUD API | P1-B3 | `crates/remote/src/routes/servers.rs` |
| P1-B5 | 实现 Agent Token 生成和验证 | P1-B4 | `crates/remote/src/auth/agent_token.rs` |
| P1-B6 | 实现 Agent WebSocket 端点 | P1-B5 | `crates/remote/src/routes/agent_ws.rs` |
| P1-B7 | 实现心跳处理和状态更新 | P1-B6 | `crates/remote/src/services/agent_connection.rs` |
| P1-B8 | 生成 TypeScript 类型 | P1-B4 | `shared/types.ts` |

**Agent Daemon 任务 (Rust)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P1-A1 | 创建 agent-daemon crate | 无 | `crates/agent-daemon/Cargo.toml` |
| P1-A2 | 实现 main.rs 入口 | P1-A1 | `crates/agent-daemon/src/main.rs` |
| P1-A3 | 实现连接管理模块 | P1-A2 | `crates/agent-daemon/src/connection.rs` |
| P1-A4 | 实现消息协议模块 | P1-A3 | `crates/agent-daemon/src/protocol.rs` |
| P1-A5 | 实现系统监控模块 | P1-A2 | `crates/agent-daemon/src/system_monitor.rs` |
| P1-A6 | 实现执行器发现 | P1-A2 | `crates/agent-daemon/src/executor_discovery.rs` |
| P1-A7 | 编写 Dockerfile | P1-A2 | `crates/agent-daemon/Dockerfile` |
| P1-A8 | 编写 docker-compose 示例 | P1-A7 | `crates/agent-daemon/docker-compose.yml` |

**前端任务 (React)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P1-F1 | 创建服务器列表页面 | P1-B8 | `frontend/src/pages/servers/ServersPage.tsx` |
| P1-F2 | 创建服务器卡片组件 | P1-F1 | `frontend/src/components/servers/ServerCard.tsx` |
| P1-F3 | 创建添加服务器对话框 | P1-F1 | `frontend/src/components/dialogs/AddServerDialog.tsx` |
| P1-F4 | 创建服务器详情页面 | P1-F1 | `frontend/src/pages/servers/ServerDetailPage.tsx` |
| P1-F5 | 实现服务器状态实时更新 | P1-F1 | `frontend/src/hooks/useServerStatus.ts` |
| P1-F6 | 创建 Agent 安装指引组件 | P1-F3 | `frontend/src/components/servers/AgentInstallGuide.tsx` |
| P1-F7 | 添加路由配置 | P1-F1 | `frontend/src/App.tsx` |
| P1-F8 | 添加导航菜单项 | P1-F7 | `frontend/src/components/layout/Sidebar.tsx` |

#### 11.1.3 验收标准

- [ ] 用户可以在 Web UI 中添加新服务器，获得 Agent Token 和 Docker 启动命令
- [ ] Agent Daemon 可以通过 Docker 在远程服务器上启动
- [ ] Agent 成功连接到中心服务器并完成注册
- [ ] 服务器状态（在线/离线）在 Web UI 中实时更新
- [ ] 服务器资源使用情况（CPU/内存/磁盘）正确显示
- [ ] 服务器上可用的执行器列表正确显示
- [ ] Agent 断线后自动重连
- [ ] 心跳超时后服务器状态自动变为离线

---

### 11.2 Phase 2: 任务调度与分配

#### 11.2.1 目标

实现任务的人机协作分配，使用户能够：
- 将任务分配给团队成员（人）
- 将任务分配给 AI（指定服务器和执行器）
- 查看任务分配状态
- AI 自动接收并执行分配的任务

#### 11.2.2 任务分解

**后端任务 (Rust)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P2-B1 | 创建 TaskAssignment 数据模型 | Phase 1 | `crates/db/src/models/task_assignment.rs` |
| P2-B2 | 编写数据库迁移文件 | P2-B1 | `crates/db/migrations/` |
| P2-B3 | 实现任务分配 API | P2-B2 | `crates/remote/src/routes/assignments.rs` |
| P2-B4 | 实现任务调度服务 | P2-B3 | `crates/remote/src/services/task_scheduler.rs` |
| P2-B5 | 扩展 Agent WebSocket 支持任务下发 | P2-B4 | `crates/remote/src/routes/agent_ws.rs` |
| P2-B6 | 实现任务状态同步 | P2-B5 | `crates/remote/src/services/task_sync.rs` |

**Agent Daemon 任务 (Rust)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P2-A1 | 实现任务接收处理 | Phase 1 | `crates/agent-daemon/src/task_handler.rs` |
| P2-A2 | 实现执行器桥接模块 | P2-A1 | `crates/agent-daemon/src/executor_bridge.rs` |
| P2-A3 | 实现工作区创建 | P2-A2 | `crates/agent-daemon/src/workspace.rs` |
| P2-A4 | 实现执行状态上报 | P2-A2 | `crates/agent-daemon/src/status_reporter.rs` |
| P2-A5 | 实现任务取消处理 | P2-A2 | `crates/agent-daemon/src/task_handler.rs` |

**前端任务 (React)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P2-F1 | 创建任务分配对话框 | Phase 1 | `frontend/src/components/dialogs/AssignTaskDialog.tsx` |
| P2-F2 | 扩展任务卡片显示分配信息 | P2-F1 | `frontend/src/components/tasks/TaskCard.tsx` |
| P2-F3 | 创建分配人/AI 选择器 | P2-F1 | `frontend/src/components/assignments/AssigneeSelector.tsx` |
| P2-F4 | 创建服务器/执行器选择器 | P2-F1 | `frontend/src/components/assignments/ServerExecutorSelector.tsx` |
| P2-F5 | 实现任务分配状态显示 | P2-F2 | `frontend/src/components/assignments/AssignmentStatus.tsx` |
| P2-F6 | 扩展看板支持分配筛选 | P2-F2 | `frontend/src/pages/ui-new/ProjectKanban.tsx` |

#### 11.2.3 验收标准

- [ ] 用户可以将任务分配给团队成员
- [ ] 用户可以将任务分配给指定服务器上的 AI 执行器
- [ ] 分配给 AI 的任务自动下发到对应的 Agent
- [ ] Agent 接收任务后自动创建工作区并启动执行
- [ ] 任务执行状态实时同步到中心服务器
- [ ] 用户可以取消正在执行的任务
- [ ] 看板上可以按分配人/AI 筛选任务

---

### 11.3 Phase 3: 可观测性

#### 11.3.1 目标

实现 AI 任务执行的细粒度可观测性，使用户能够：
- 实时查看 AI 工具的执行进度
- 观测每次工具调用（Read/Edit/Bash 等）
- 观测技能使用情况
- 查看执行时间线和统计
- 实时查看执行日志

#### 11.3.2 任务分解

**后端任务 (Rust)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P3-B1 | 创建 ExecutionEvent 数据模型 | Phase 2 | `crates/db/src/models/execution_event.rs` |
| P3-B2 | 编写数据库迁移文件 | P3-B1 | `crates/db/migrations/` |
| P3-B3 | 实现事件存储服务 | P3-B2 | `crates/remote/src/services/event_store.rs` |
| P3-B4 | 实现事件查询 API | P3-B3 | `crates/remote/src/routes/events.rs` |
| P3-B5 | 实现事件 WebSocket 流 | P3-B4 | `crates/remote/src/routes/event_stream.rs` |
| P3-B6 | 实现日志流 WebSocket | P3-B5 | `crates/remote/src/routes/log_stream.rs` |

**Agent Daemon 任务 (Rust)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P3-A1 | 实现事件采集器 | Phase 2 | `crates/agent-daemon/src/event_collector.rs` |
| P3-A2 | 实现 Claude Code 事件解析器 | P3-A1 | `crates/agent-daemon/src/parsers/claude_code.rs` |
| P3-A3 | 实现 Gemini CLI 事件解析器 | P3-A1 | `crates/agent-daemon/src/parsers/gemini.rs` |
| P3-A4 | 实现 Codex 事件解析器 | P3-A1 | `crates/agent-daemon/src/parsers/codex.rs` |
| P3-A5 | 实现事件上报 | P3-A1 | `crates/agent-daemon/src/event_reporter.rs` |
| P3-A6 | 实现日志流上报 | P3-A5 | `crates/agent-daemon/src/log_streamer.rs` |

**前端任务 (React)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P3-F1 | 创建执行时间线页面 | Phase 2 | `frontend/src/pages/observability/ExecutionTimelinePage.tsx` |
| P3-F2 | 创建时间线组件 | P3-F1 | `frontend/src/components/observability/Timeline.tsx` |
| P3-F3 | 创建工具调用事件组件 | P3-F2 | `frontend/src/components/observability/ToolUseEvent.tsx` |
| P3-F4 | 创建工具使用统计面板 | P3-F1 | `frontend/src/components/observability/ToolUsagePanel.tsx` |
| P3-F5 | 创建实时日志面板 | P3-F1 | `frontend/src/components/observability/LiveLogsPanel.tsx` |
| P3-F6 | 实现事件流 WebSocket Hook | P3-F2 | `frontend/src/hooks/useEventStream.ts` |
| P3-F7 | 实现日志流 WebSocket Hook | P3-F5 | `frontend/src/hooks/useLogStream.ts` |
| P3-F8 | 创建上下文使用面板 | P3-F1 | `frontend/src/components/observability/ContextUsagePanel.tsx` |

#### 11.3.3 验收标准

- [ ] 执行时间线页面正确显示所有事件
- [ ] 工具调用事件显示工具名称、参数摘要、耗时、结果
- [ ] 技能调用事件正确显示
- [ ] Agent 切换事件正确显示
- [ ] 工具使用统计面板显示调用分布和成功率
- [ ] 实时日志面板正确显示 stdout/stderr
- [ ] 上下文使用面板显示 Token 使用情况
- [ ] 事件流实时更新，无明显延迟

---

### 11.4 Phase 4: 配置下发

#### 11.4.1 目标

实现统一的配置管理和下发，使用户能够：
- 在 Web UI 中管理 MCP Server 配置
- 在 Web UI 中管理 Skills 配置
- 在 Web UI 中管理 Agent 人格配置
- 在 Web UI 中管理模型选择配置
- 将配置下发到指定服务器或所有服务器
- 查看配置下发历史和状态

#### 11.4.2 任务分解

**后端任务 (Rust)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P4-B1 | 创建 ConfigDeployment 数据模型 | Phase 3 | `crates/db/src/models/config_deployment.rs` |
| P4-B2 | 编写数据库迁移文件 | P4-B1 | `crates/db/migrations/` |
| P4-B3 | 实现配置管理 API | P4-B2 | `crates/remote/src/routes/configs.rs` |
| P4-B4 | 实现配置下发服务 | P4-B3 | `crates/remote/src/services/config_deployer.rs` |
| P4-B5 | 扩展 Agent WebSocket 支持配置下发 | P4-B4 | `crates/remote/src/routes/agent_ws.rs` |
| P4-B6 | 实现配置版本管理 | P4-B3 | `crates/remote/src/services/config_versioning.rs` |

**Agent Daemon 任务 (Rust)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P4-A1 | 实现配置管理器 | Phase 3 | `crates/agent-daemon/src/config_manager.rs` |
| P4-A2 | 实现 MCP 配置应用 | P4-A1 | `crates/agent-daemon/src/config/mcp.rs` |
| P4-A3 | 实现 Skills 配置应用 | P4-A1 | `crates/agent-daemon/src/config/skills.rs` |
| P4-A4 | 实现 Agent 配置应用 | P4-A1 | `crates/agent-daemon/src/config/agents.rs` |
| P4-A5 | 实现模型配置应用 | P4-A1 | `crates/agent-daemon/src/config/models.rs` |
| P4-A6 | 实现配置应用确认上报 | P4-A1 | `crates/agent-daemon/src/config_manager.rs` |

**前端任务 (React)**

| 任务 | 描述 | 依赖 | 文件 |
|------|------|------|------|
| P4-F1 | 创建配置管理页面 | Phase 3 | `frontend/src/pages/config/ConfigManagementPage.tsx` |
| P4-F2 | 创建 MCP 配置编辑器 | P4-F1 | `frontend/src/components/config/McpConfigEditor.tsx` |
| P4-F3 | 创建 Skills 配置编辑器 | P4-F1 | `frontend/src/components/config/SkillsConfigEditor.tsx` |
| P4-F4 | 创建 Agent 配置编辑器 | P4-F1 | `frontend/src/components/config/AgentConfigEditor.tsx` |
| P4-F5 | 创建模型配置编辑器 | P4-F1 | `frontend/src/components/config/ModelConfigEditor.tsx` |
| P4-F6 | 创建配置下发对话框 | P4-F1 | `frontend/src/components/dialogs/ConfigDeployDialog.tsx` |
| P4-F7 | 创建下发历史组件 | P4-F1 | `frontend/src/components/config/DeploymentHistory.tsx` |
| P4-F8 | 创建 JSON 编辑器组件 | P4-F2 | `frontend/src/components/common/JsonEditor.tsx` |

#### 11.4.3 验收标准

- [ ] 用户可以在 Web UI 中创建和编辑 MCP Server 配置
- [ ] 用户可以在 Web UI 中创建和编辑 Skills 配置
- [ ] 用户可以在 Web UI 中创建和编辑 Agent 人格配置
- [ ] 用户可以在 Web UI 中选择模型配置
- [ ] 用户可以将配置下发到单个服务器或所有服务器
- [ ] Agent 正确接收并应用配置
- [ ] 配置应用状态正确上报并显示
- [ ] 下发历史正确记录和显示
- [ ] 支持配置版本管理和回滚

---

## 12. 风险与缓解措施

### 12.1 技术风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| AI 工具输出格式不统一 | 事件解析困难 | 高 | 为每种工具实现专用解析器，使用正则和 JSON 解析结合 |
| WebSocket 连接不稳定 | 状态同步延迟 | 中 | 实现指数退避重连，本地事件缓存，断线恢复机制 |
| 大量事件导致性能问题 | 系统响应慢 | 中 | 事件采样、聚合、分页查询，使用时序数据库 |
| Docker 环境差异 | Agent 部署失败 | 中 | 提供多种基础镜像，详细的故障排查文档 |

### 12.2 产品风险

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| 用户学习成本高 | 采用率低 | 中 | 提供详细的入门指南，简化首次配置流程 |
| 配置下发导致 AI 工具异常 | 用户体验差 | 中 | 配置验证，支持回滚，提供配置模板 |
| 权限模型过于简单 | 无法满足企业需求 | 低 | 预留扩展点，后续可升级为 RBAC |

---

## 13. 附录

### 13.1 术语表

| 术语 | 定义 |
|------|------|
| Agent Daemon | 部署在远程服务器上的守护进程，负责与中心通信、执行任务、上报状态 |
| Center | 中心控制面板，包括 Web UI 和 API 服务器 |
| Executor | 执行器，指具体的 AI 编码工具（如 Claude Code、Gemini CLI） |
| MCP | Model Context Protocol，AI 工具的扩展协议 |
| Skill | 技能，AI 工具的预定义能力（如 /commit、/review-pr） |
| Workspace | 工作区，任务执行的隔离环境，包含 Git worktree |

### 13.2 参考资料

- [现有项目架构分析](./architecture-analysis.md)
- [Vibe Kanban 原始文档](../CLAUDE.md)
- [MCP 协议规范](https://modelcontextprotocol.io/)
- [Claude Code 文档](https://docs.anthropic.com/claude-code)

### 13.3 变更历史

| 版本 | 日期 | 作者 | 变更内容 |
|------|------|------|---------|
| 1.0 | 2026-02-08 | AI Assistant | 初始版本，包含完整架构设计和实现计划 |
