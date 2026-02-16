#!/bin/bash
# =============================================================================
# Vibe Kanban AI Tool Orchestration Platform - 部署脚本
# =============================================================================
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 检查依赖
check_dependencies() {
    log_info "检查依赖..."

    if ! command -v docker &> /dev/null; then
        log_error "Docker 未安装。请先安装 Docker: https://docs.docker.com/get-docker/"
        exit 1
    fi

    if ! command -v docker compose &> /dev/null; then
        log_error "Docker Compose 未安装。请先安装 Docker Compose"
        exit 1
    fi

    log_info "依赖检查通过"
}

# 检查环境变量
check_env() {
    log_info "检查环境变量..."

    if [ ! -f ".env" ]; then
        log_error ".env 文件不存在"
        log_info "请复制 .env.example 并填写配置："
        echo "  cp .env.example .env"
        echo "  vim .env"
        exit 1
    fi

    # 加载环境变量
    set -a
    source .env
    set +a

    # 检查必填变量
    local required_vars=(
        "POSTGRES_PASSWORD"
        "VIBEKANBAN_REMOTE_JWT_SECRET"
        "ELECTRIC_ROLE_PASSWORD"
        "SERVER_PUBLIC_BASE_URL"
        "CENTER_WS_URL"
        "TOKEN_HASH_SECRET"
    )

    local missing=()
    for var in "${required_vars[@]}"; do
        if [ -z "${!var}" ]; then
            missing+=("$var")
        fi
    done

    # 检查 OAuth（至少需要一个）
    if [ -z "$GITHUB_OAUTH_CLIENT_ID" ] && [ -z "$GOOGLE_OAUTH_CLIENT_ID" ]; then
        missing+=("GITHUB_OAUTH_CLIENT_ID 或 GOOGLE_OAUTH_CLIENT_ID")
    fi

    if [ ${#missing[@]} -gt 0 ]; then
        log_error "以下必填环境变量未设置："
        for var in "${missing[@]}"; do
            echo "  - $var"
        done
        exit 1
    fi

    log_info "环境变量检查通过"
}

# 生成密钥
generate_secrets() {
    log_info "生成密钥..."

    if [ -z "$VIBEKANBAN_REMOTE_JWT_SECRET" ]; then
        echo "VIBEKANBAN_REMOTE_JWT_SECRET=$(openssl rand -base64 48)"
    fi

    if [ -z "$ELECTRIC_ROLE_PASSWORD" ]; then
        echo "ELECTRIC_ROLE_PASSWORD=$(openssl rand -hex 16)"
    fi

    if [ -z "$TOKEN_HASH_SECRET" ]; then
        echo "TOKEN_HASH_SECRET=$(openssl rand -hex 32)"
    fi

    if [ -z "$POSTGRES_PASSWORD" ]; then
        echo "POSTGRES_PASSWORD=$(openssl rand -hex 16)"
    fi
}

# 构建镜像
build() {
    log_info "构建 Docker 镜像..."
    docker compose build --no-cache
    log_info "构建完成"
}

# 启动服务
start() {
    log_info "启动服务..."
    docker compose up -d

    log_info "等待服务启动..."
    sleep 5

    # 检查服务状态
    if docker compose ps | grep -q "unhealthy\|Exit"; then
        log_error "部分服务启动失败，请检查日志："
        echo "  docker compose logs"
        exit 1
    fi

    log_info "服务启动成功！"
    echo ""
    echo "访问地址: $SERVER_PUBLIC_BASE_URL"
    echo "查看日志: docker compose logs -f"
}

# 停止服务
stop() {
    log_info "停止服务..."
    docker compose down
    log_info "服务已停止"
}

# 重启服务
restart() {
    stop
    start
}

# 查看状态
status() {
    docker compose ps
}

# 查看日志
logs() {
    docker compose logs -f "$@"
}

# 更新部署
update() {
    log_info "更新部署..."

    # 拉取最新代码
    cd "$SCRIPT_DIR/.."
    git pull origin feature/ai-tool-orchestration-platform

    cd "$SCRIPT_DIR"

    # 重新构建并启动
    build
    docker compose up -d

    log_info "更新完成"
}

# 清理数据（危险操作）
clean() {
    log_warn "此操作将删除所有数据！"
    read -p "确认删除？(输入 'yes' 确认): " confirm

    if [ "$confirm" != "yes" ]; then
        log_info "操作已取消"
        exit 0
    fi

    docker compose down -v
    log_info "数据已清理"
}

# 显示帮助
show_help() {
    echo "Vibe Kanban 部署脚本"
    echo ""
    echo "用法: ./deploy.sh <命令>"
    echo ""
    echo "命令:"
    echo "  check       检查依赖和环境变量"
    echo "  secrets     生成随机密钥（复制到 .env）"
    echo "  build       构建 Docker 镜像"
    echo "  start       启动所有服务"
    echo "  stop        停止所有服务"
    echo "  restart     重启所有服务"
    echo "  status      查看服务状态"
    echo "  logs        查看服务日志"
    echo "  update      更新并重新部署"
    echo "  clean       清理所有数据（危险）"
    echo "  help        显示此帮助"
}

# 主入口
case "${1:-help}" in
    check)
        check_dependencies
        check_env
        ;;
    secrets)
        generate_secrets
        ;;
    build)
        check_dependencies
        build
        ;;
    start)
        check_dependencies
        check_env
        start
        ;;
    stop)
        stop
        ;;
    restart)
        check_dependencies
        check_env
        restart
        ;;
    status)
        status
        ;;
    logs)
        shift
        logs "$@"
        ;;
    update)
        check_dependencies
        update
        ;;
    clean)
        clean
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        log_error "未知命令: $1"
        show_help
        exit 1
        ;;
esac
