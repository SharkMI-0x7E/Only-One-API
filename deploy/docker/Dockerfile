# RapidGate 多阶段 Dockerfile
# 目标：最终镜像 < 50MB，仅包含静态编译的二进制与配置文件
# 构建命令（在仓库根执行）：
#   docker build -f deploy/docker/Dockerfile -t rapidgate:latest .

# ============================================================
# 阶段 1：构建阶段
# ============================================================
FROM rust:1.77-bookworm AS builder

WORKDIR /build

# 先复制依赖清单，利用 Docker 缓存层加速重复构建
COPY Cargo.toml Cargo.lock ./
# 创建占位源码，让 cargo 提前下载并缓存依赖
RUN mkdir -p src && echo "fn main() {}" > src/main.rs && \
    cargo build --release --locked || true
# 清理占位产物
RUN rm -rf src target/release/deps/rapidgate*

# 复制真实源码并构建
COPY src ./src
# 若存在 plugins / benches 等子目录也一并复制（保持构建通用性）
COPY plugins ./plugins
COPY benches ./benches

RUN cargo build --release --locked && \
    strip target/release/rapidgate

# ============================================================
# 阶段 2：运行阶段
# 使用 distroless/static-debian12：无 shell、无包管理器，攻击面最小
# 仅含 ca-certificates（rustls 仍需系统根证书）与 tzdata（日志时区）
# ============================================================
FROM gcr.io/distroless/static-debian12:nonroot

# distroless 默认 USER 为 nonroot（uid=65532），无需显式设置
LABEL maintainer="RapidGate Contributors"
LABEL description="High-performance unified LLM API gateway"

# 二进制
COPY --from=builder /build/target/release/rapidgate /usr/local/bin/rapidgate
# 默认配置（运行时可通过挂载覆盖）
COPY config /etc/rapidgate

# 运行时环境变量
ENV RGD_CONFIG_DIR=/etc/rapidgate
ENV RUST_LOG=info

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/rapidgate"]
