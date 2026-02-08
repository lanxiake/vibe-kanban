# Vibe Kanban Agent Daemon Dockerfile
# 用于在远程服务器上部署 AI 工具管理守护进程

# ===== 构建阶段 =====
FROM rust:1.83-slim-bookworm AS builder

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 复制 Cargo 配置
COPY Cargo.toml Cargo.lock* ./

# 复制源代码
COPY src ./src

# 构建 release 版本
RUN cargo build --release

# ===== 运行阶段 =====
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
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户
RUN useradd -m -s /bin/bash agent

# 复制构建产物
COPY --from=builder /app/target/release/vibe-agent /usr/local/bin/vibe-agent

# 创建目录
RUN mkdir -p /app/workspaces /app/config /var/log/agent \
    && chown -R agent:agent /app /var/log/agent

# 切换到非 root 用户
USER agent

# 设置工作目录
WORKDIR /app

# 配置目录（挂载点）
VOLUME ["/app/config", "/app/workspaces", "/var/log/agent"]

# 环境变量
ENV CENTER_URL=""
ENV AGENT_TOKEN=""
ENV LOG_LEVEL="info"
ENV HEARTBEAT_INTERVAL="30"
ENV WORKSPACE_DIR="/app/workspaces"
ENV CONFIG_DIR="/app/config"

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD pgrep vibe-agent || exit 1

# 入口点
ENTRYPOINT ["/usr/local/bin/vibe-agent"]
