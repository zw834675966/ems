#!/usr/bin/env bash
set -e


# 加载环境变量 (使用 set -a 自动 export)
if [ -f .env ]; then
  set -a
  source .env
  set +a
fi

# Ensure cargo bin is in PATH
export PATH="/home/zw/.cargo/bin:$PATH"



echo "🚀 正在启动 EMS 服务..."

# WSL/Windows 文件系统下，Rust 增量编译有时会因为锁文件失败而中断。
# 关闭增量编译可避免 `could not create session directory lock file`。
export CARGO_INCREMENTAL=0

# 1. 运行环境检查
if [ -f "./scripts/ems-doctor.sh" ]; then
  # 仅显示简要信息，除非有错误
  ./scripts/ems-doctor.sh > ems-doctor.log 2>&1 || { cat ems-doctor.log; exit 1; }
  echo "✅ 环境检查通过。"
fi

# 2. 运行数据库迁移
echo "正在执行数据库迁移..."
/home/zw/.cargo/bin/sqlx migrate run

# 3. 启动采集引擎（后台）
echo "正在启动采集引擎 (Collector)..."
cargo run -q -p ems-collector --bin ems-collector > collector.log 2>&1 &
COLLECTOR_PID=$!
echo "采集引擎 PID: $COLLECTOR_PID (日志: collector.log)"

# 4. 启动 API 服务（前台）
echo "正在启动 API 服务..."
# API 服务会自动尝试启动 pnpm dev (web admin)
# 使用 -q 减少编译日志，但保留运行日志
cargo run -q --bin ems-api &
API_PID=$!
echo "API 服务 PID: $API_PID"
echo "---------------------------------------------------"
echo "✅  EMS 服务已启动"
echo "    - API 接口:   http://localhost:8080"
echo "    - 管理后台:   http://localhost:8848 (启动中...)"
echo "---------------------------------------------------"
echo "查看日志: tail -f collector.log"

cleanup() {
  echo -e "\n正在停止服务..."
  [ -n "$COLLECTOR_PID" ] && kill $COLLECTOR_PID 2>/dev/null || true
  [ -n "$API_PID" ] && kill $API_PID 2>/dev/null || true
  echo "完成。"
}

trap cleanup SIGINT SIGTERM

wait $API_PID $COLLECTOR_PID
