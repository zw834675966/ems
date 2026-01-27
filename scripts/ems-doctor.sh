#!/usr/bin/env bash
set -e

# EMS 环境诊断脚本 (EMS Doctor)
# 旨在帮助独立开发者快速检查本地开发运行环境。

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🔍 正在进行 EMS 环境诊断..."
echo "=============================="

# 1. 检查环境变量文件
if [ -f .env ]; then
    echo -e "✅ [环境变量] .env 文件已找到。"
    # 加载环境变量用于后续测试
    export $(grep -v '^#' .env | xargs)
else
    echo -e "❌ [环境变量] .env 文件缺失！请从 .env.example 复制并配置。"
    exit 1
fi

# 2. 检查系统依赖服务
CHECK_SERVICES=("postgresql" "mosquitto")

for svc in "${CHECK_SERVICES[@]}"; do
    if systemctl is-active --quiet "$svc" 2>/dev/null || service "$svc" status >/dev/null 2>&1; then
        echo -e "✅ [系统服务] $svc 正在运行。"
    else
        echo -e "⚠️  [系统服务] $svc 未运行或未安装。请检查 sudo systemctl start $svc"
    fi
done

# 3. 检查后端依赖项 (sqlx, cargo)
if command -v cargo >/dev/null 2>&1; then
    echo -e "✅ [系统工具] cargo 已安装。"
else
    echo -e "❌ [系统工具] cargo 未找到，请安装 Rust 环境。"
fi

if command -v sqlx >/dev/null 2>&1; then
    echo -e "✅ [系统工具] sqlx-cli 已安装。"
else
    echo -e "⚠️  [系统工具] sqlx-cli 未找到，建议运行: cargo install sqlx-cli"
fi

# 4. 检查前端依赖项 (pnpm, node)
if command -v node >/dev/null 2>&1; then
    node_v=$(node -v)
    echo -e "✅ [前端工具] node 已安装 ($node_v)。"
else
    echo -e "❌ [前端工具] node 未找到。"
fi

if command -v pnpm >/dev/null 2>&1; then
    echo -e "✅ [前端工具] pnpm 已安装。"
else
    echo -e "⚠️  [前端工具] pnpm 未找到，前端可能无法启动。"
fi

# 5. 检查数据库连接
if [ -n "${DATABASE_URL:-}" ]; then
    if command -v psql >/dev/null 2>&1; then
        if psql "$DATABASE_URL" -c "select 1" >/dev/null 2>&1; then
            echo -e "✅ [数据库] 能够成功连接到数据库。"
        else
            echo -e "❌ [数据库] 无法连接到数据库，请检查 DATABASE_URL 和 Postgres 服务。"
        fi
    else
        echo -e "ℹ️  [数据库] psql 客户端未安装，跳过连接测试。"
    fi
fi

# 6. 检查端口占用
PORTS=(8080 8848 1883 5432)
for port in "${PORTS[@]}"; do
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        echo -e "⚠️  [网络端口] 端口 $port 已被占用。"
    fi
done

echo "=============================="
echo -e "${GREEN}诊断完成。${NC} 如果有 ❌ 或 ⚠️，请先修复后再运行 run-dev.sh。"
