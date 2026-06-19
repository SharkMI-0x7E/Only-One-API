#!/usr/bin/env bash
# load-test.sh - RapidGate 负载测试脚本
# 使用 goose 进行 HTTP 负载测试，支持自定义配置

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
DEFAULT_CONFIG="$PROJECT_DIR/config/load-test.yaml"
LOG_FILE="$PROJECT_DIR/target/load-test-$(date +%Y%m%d-%H%M%S).log"

usage() {
    cat <<EOF
用法: $0 [选项]

选项:
    -c, --config <path>     goose 配置文件路径（默认: config/load-test.yaml）
    -u, --users <n>         并发用户数（覆盖配置文件）
    -r, --ramp-up <secs>    爬坡时间（秒），默认 10
    -d, --duration <secs>   测试持续时间（秒），默认 60
    -t, --target <url>      目标 URL（覆盖配置文件）
    -o, --output <path>     指定输出日志路径
    --report <path>         生成 HTML 报告到指定路径
    -h, --help              显示帮助

示例:
    $0 --config config/load-test.yaml
    $0 --target http://localhost:3000 --users 100 --duration 120
    $0 --users 50 --ramp-up 30 --report report.html
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

CONFIG_FILE=""
USERS=""
RAMP_UP=10
DURATION=60
TARGET=""
REPORT=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -c|--config)
            [[ -z "${2:-}" ]] && error "--config 需要参数"
            CONFIG_FILE="$2"; shift 2 ;;
        -u|--users)
            [[ -z "${2:-}" ]] && error "--users 需要参数"
            USERS="$2"; shift 2 ;;
        -r|--ramp-up)
            [[ -z "${2:-}" ]] && error "--ramp-up 需要参数"
            RAMP_UP="$2"; shift 2 ;;
        -d|--duration)
            [[ -z "${2:-}" ]] && error "--duration 需要参数"
            DURATION="$2"; shift 2 ;;
        -t|--target)
            [[ -z "${2:-}" ]] && error "--target 需要参数"
            TARGET="$2"; shift 2 ;;
        -o|--output)
            [[ -z "${2:-}" ]] && error "--output 需要参数"
            LOG_FILE="$2"; shift 2 ;;
        --report)
            [[ -z "${2:-}" ]] && error "--report 需要参数"
            REPORT="$2"; shift 2 ;;
        -h|--help)
            usage ;;
        *)
            error "未知参数: $1" ;;
    esac
done

# 确定配置文件
if [[ -n "$CONFIG_FILE" ]]; then
    [[ ! -f "$CONFIG_FILE" ]] && error "配置文件不存在: $CONFIG_FILE"
elif [[ -f "$DEFAULT_CONFIG" ]]; then
    CONFIG_FILE="$DEFAULT_CONFIG"
else
    error "未找到配置文件，请使用 -c 指定或使用 --help 查看用法"
fi

mkdir -p "$(dirname "$LOG_FILE")"

log "开始负载测试"
log "配置文件: $CONFIG_FILE"
[[ -n "$USERS" ]] && log "并发用户数: $USERS"
log "爬坡时间: ${RAMP_UP}s"
log "测试时长: ${DURATION}s"
[[ -n "$TARGET" ]] && log "目标 URL: $TARGET"
log "日志文件: $LOG_FILE"

# 检查 goose 是否安装
if ! command -v goose &>/dev/null; then
    error "未找到 goose 命令，请先安装: cargo install goose-cli"
fi

# 构建 goose 参数
GOOSE_ARGS=(-c "$CONFIG_FILE" -u "$RAMP_UP" -t "$DURATION")
[[ -n "$USERS" ]] && GOOSE_ARGS+=(-v "$USERS")
[[ -n "$TARGET" ]] && GOOSE_ARGS+=(-H "$TARGET")
[[ -n "$REPORT" ]] && GOOSE_ARGS+=(--report-file "$REPORT")

log "执行命令: goose ${GOOSE_ARGS[*]}"
if goose ${GOOSE_ARGS[@]} 2>&1 | tee -a "$LOG_FILE"; then
    log "负载测试完成"
else
    error "负载测试失败，退出码: $?"
fi

[[ -n "$REPORT" ]] && log "HTML 报告已生成: $REPORT"
log "结果已保存至: $LOG_FILE"
