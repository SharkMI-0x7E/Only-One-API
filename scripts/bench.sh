#!/usr/bin/env bash
# bench.sh - RapidGate 基准测试脚本
# 使用 cargo bench 运行基准测试，支持自定义参数

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LOG_FILE="$PROJECT_DIR/target/bench-$(date +%Y%m%d-%H%M%S).log"

usage() {
    cat <<EOF
用法: $0 [选项]

选项:
    -c, --config <path>     指定基准测试配置文件
    -f, --filter <pattern>  只运行匹配名称的测试
    -o, --output <path>     指定输出日志路径
    -s, --save-baseline     保存当前结果作为基线
    -h, --help              显示帮助

示例:
    $0
    $0 --filter chat_completion
    $0 --save-baseline --output results.log
EOF
    exit 0
}

log() {
    local msg="[$(date '+%Y-%m-%d %H:%M:%S')] $*"
    echo "$msg"
    echo "$msg" >> "$LOG_FILE"
}

error() {
    log "ERROR: $*" >&2
    exit 1
}

BENCH_ARGS=()
SAVE_BASELINE=false
CONFIG_FILE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -c|--config)
            [[ -z "${2:-}" ]] && error "--config 需要参数"
            CONFIG_FILE="$2"; shift 2 ;;
        -f|--filter)
            [[ -z "${2:-}" ]] && error "--filter 需要参数"
            BENCH_ARGS+=("$2"); shift 2 ;;
        -o|--output)
            [[ -z "${2:-}" ]] && error "--output 需要参数"
            LOG_FILE="$2"; shift 2 ;;
        -s|--save-baseline)
            SAVE_BASELINE=true; shift ;;
        -h|--help)
            usage ;;
        *)
            error "未知参数: $1" ;;
    esac
done

# 检查项目目录
[[ ! -f "$PROJECT_DIR/Cargo.toml" ]] && error "未找到 Cargo.toml，请在项目根目录运行"

# 创建日志目录
mkdir -p "$(dirname "$LOG_FILE")"

log "开始基准测试"
log "项目目录: $PROJECT_DIR"
log "日志文件: $LOG_FILE"

# 如果指定了配置文件，验证其存在
if [[ -n "$CONFIG_FILE" ]]; then
    [[ ! -f "$CONFIG_FILE" ]] && error "配置文件不存在: $CONFIG_FILE"
    BENCH_ARGS+=(--config "$CONFIG_FILE")
    log "使用配置文件: $CONFIG_FILE"
fi

# 如果保存基线
if [[ "$SAVE_BASELINE" == true ]]; then
    BENCH_ARGS+=(--save-baseline rapidgate-baseline)
    log "将保存基线: rapidgate-baseline"
fi

cd "$PROJECT_DIR"

# 运行基准测试
log "执行命令: cargo bench ${BENCH_ARGS[*]:-}"
if cargo bench ${BENCH_ARGS[@]+"${BENCH_ARGS[@]}"} 2>&1 | tee -a "$LOG_FILE"; then
    log "基准测试完成"
else
    error "基准测试失败，退出码: $?"
fi

log "结果已保存至: $LOG_FILE"
