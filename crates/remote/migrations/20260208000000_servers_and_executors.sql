-- AI 工具跨设备管理平台 - 服务器管理表
-- 用于管理远程服务器和 Agent Daemon 连接

-- 服务器状态枚举
CREATE TYPE server_status AS ENUM ('pending', 'online', 'offline', 'error');

-- 执行器状态枚举
CREATE TYPE executor_status AS ENUM ('unknown', 'available', 'busy', 'error');

-- 服务器表
CREATE TABLE servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    -- 基本信息
    name VARCHAR(255) NOT NULL,
    description TEXT,
    host VARCHAR(255) NOT NULL,
    port INTEGER NOT NULL DEFAULT 9999,

    -- 连接状态
    status server_status NOT NULL DEFAULT 'pending',
    agent_id UUID,
    agent_version VARCHAR(50),
    last_heartbeat_at TIMESTAMPTZ,
    connection_error TEXT,

    -- 系统信息（由 Agent 上报）
    system_info JSONB DEFAULT '{}',
    system_stats JSONB DEFAULT '{}',

    -- SSH 兜底配置（可选）
    ssh_enabled BOOLEAN NOT NULL DEFAULT false,
    ssh_host VARCHAR(255),
    ssh_port INTEGER DEFAULT 22,
    ssh_username VARCHAR(255),
    ssh_auth_type VARCHAR(50),
    ssh_credentials_encrypted TEXT,

    -- 元数据
    tags JSONB NOT NULL DEFAULT '[]',
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Agent Token（加密存储）
    agent_token_hash VARCHAR(255),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(organization_id, name)
);

-- 服务器执行器表
CREATE TABLE server_executors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,

    -- 执行器信息
    executor_type VARCHAR(100) NOT NULL,
    executor_name VARCHAR(255),
    version VARCHAR(50),

    -- 状态
    status executor_status NOT NULL DEFAULT 'unknown',
    current_task_id UUID,

    -- 能力和配置
    capabilities JSONB NOT NULL DEFAULT '[]',
    config JSONB NOT NULL DEFAULT '{}',

    -- 统计
    total_executions INTEGER NOT NULL DEFAULT 0,
    successful_executions INTEGER NOT NULL DEFAULT 0,
    failed_executions INTEGER NOT NULL DEFAULT 0,

    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(server_id, executor_type)
);

-- 索引
CREATE INDEX idx_servers_organization_id ON servers(organization_id);
CREATE INDEX idx_servers_status ON servers(status);
CREATE INDEX idx_servers_agent_id ON servers(agent_id);
CREATE INDEX idx_server_executors_server_id ON server_executors(server_id);
CREATE INDEX idx_server_executors_status ON server_executors(status);

-- 更新时间触发器
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_servers_updated_at
    BEFORE UPDATE ON servers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_server_executors_updated_at
    BEFORE UPDATE ON server_executors
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
