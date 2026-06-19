#!/usr/bin/env bash
# fuzz.sh - RapidGate 模糊测试脚本
# 使用 cargo-fuzz 运行模糊测试，支持自定义参数

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LOG_FILE="$PROJECT_DIR/target/fuzz-$(date +%Y%m%d-%H%M%S).log"

usage() {
    cat <<EOF
用法: $0 [选项] <fuzz-target>

参数:
    fuzz-target             模糊测试目标名称（如 parse_route）

选项:
    -d, --duration <secs>   运行时长（秒），默认 60
    -j, --jobs <n>          并行任务数，默认 CPU 核心数
    -o, --output <path>     指定输出日志路径
    -m, --max-len <n>       最大输入长度，默认 4096
    --sanitizer <name>      使用的 sanitizer（address/memory/leak），默认 address
    -h, --help              显示帮助

示例:
    $0 parse_route
    $0 --duration 300 --max-len 8192 parse_config
    $0 --sanitizer memory route_match
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

DURATION=60
JOBS=""
MAX_LEN=4096
SANITIZER="address"
FUZZ_TARGET=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -d|--duration)
            [[ -z "${2:-}" ]] && error "--duration 需要参数"
            DURATION="$2"; shift 2 ;;
        -j|--jobs)
            [[ -z "${2:-}" ]] && error "--jobs 需要参数"
            JOBS="$2"; shift 2 ;;
        -o|--output)
            [[ -z "${2:-}" ]] && error "--output 需要参数"
            LOG_FILE="$2"; shift 2 ;;
        -m|--max-len)
            [[ -z "${2:-}" ]] && error "--max-len 需要参数"
            MAX_LEN="$2"; shift 2 ;;
        --sanitizer)
            [[ -z "${2:-}" ]] && error "--sanitizer 需要参数"
            SANITIZER="$2"; shift 2 ;;
        -h|--help)
            usage ;;
        -*)
            error "未知选项: $1" ;;
        *)
            FUZZ_TARGET="$1"; shift ;;
    esac
done

[[ -z "$FUZZ_TARGET" ]] && error "必须指定模糊测试目标"
[[ ! -f "$PROJECT_DIR/Cargo.toml" ]] && error "未找到 Cargo.toml，请在项目根目录运行"

mkdir -p "$(dirname "$LOG_FILE")"

log "开始模糊测试"
log "项目目录: $PROJECT_DIR"
log "目标: $FUZZ_TARGET"
log "时长: ${DURATION}s"
log "最大输入长度: $MAX_LEN"
log "Sanitizer: $SANITIZER"
log "日志文件: $LOG_FILE"

cd "$PROJECT_DIR"

# 构建 cargo-fuzz 参数
FUZZ_ARGS=(run "$FUZZ_TARGET" --max-len "$MAX_LEN" --sanitizer "$SANITIZER")
[[ -n "$JOBS" ]] && FUZZ_ARGS+=(-j "$JOBS")
FUZZ_ARGS+=(-- --max_total_time="$DURATION")

log "执行命令: cargo fuzz ${FUZZ_ARGS[*]}"
if cargo fuzz ${FUZZ_ARGS[@]} 2>&1 | tee -a "$LOG_FILE"; then
    log "模糊测试完成，未发现崩溃"
else
    EXIT_CODE=$?
    if [[ $EXIT_CODE -eq 77 ]]; then
        log "模糊测试发现崩溃，请检查 artifact 文件"
    else
        error "模糊测试异常退出，退出码: $EXIT_CODE"
    fi
fi

log "结果已保存至: $LOG_FILE"
