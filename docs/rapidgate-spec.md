# RapidGate — 可落地的架构规格（v1.0）

> 本文件是 RapidGate 的**可施工蓝图**：目录树完整、每个目录都有真实职责、依赖按提示词锁定、技术栈与协作规范以 `AGENTS.md` 为准。
>
> 任何 AI 编码助手或开发者在落地代码前，**先读完本文件**。

---

## 0. 文档使用约定

- **"首版"** 指 2026.06.x 系列（v0 阶段，单 crate）。
- **"阶段 N"** 指第九节演进路线中的对应阶段。
- 任何章节出现"阶段 N 启用"字样，**首版不允许创建对应目录/文件**。
- 涉及 CI/CD / 提交规范 / 版本号 / 工具链约束时，**以 `AGENTS.md` 为准**，本文件不重复。

---

## 1. 定位与设计原则

RapidGate 是一个用 Rust 编写的高性能统一 LLM API 网关，灵感来自 `one-api`。

**五条不可妥协的原则**（按重要性排序）：

1. **库优先（Core / Service 分层）**：核心业务逻辑封装在 `core` 模块，无 I/O、无网络、无全局运行时；`service` 模块负责 axum 集成、tokio 运行时、配置加载、文件监听。**首版不拆 workspace**（单 crate + 内部 mod），等阶段三再评估拆 `rapidgate-core` crate。
2. **流式优先（Streaming First）**：LLM 网关 90% 流量是 SSE 流式响应。所有转发路径必须**支持流式**，绝不能把响应体一次性 `collect` 到内存后再回写。
3. **配置即代码（Config-as-Data）**：路由、上游、限流、熔断**全部**从 YAML 文件加载，不在代码里硬编码任何业务配置。**敏感字段**（API key、token）一律从环境变量读取。
4. **热重载安全（Hot Reload with Rollback）**：路由配置变更可热加载，但必须支持**配置校验失败时回滚到旧版本**，in-flight 请求继续用旧配置跑完。
5. **SSRF 防护（Upstream Allowlist）**：所有上游 base_url 必须经过**白名单 + 域名解析后 IP 段检查**，防止 LLM 网关被用作 SSRF 跳板。

---

## 2. 目录树（终极版 · 带阶段标注）

本节是**项目最终形态的目录树**，每个目录 / 文件末尾用方括号标注其归属阶段：

- **`[S1]`** — 阶段一（基础落地）创建
- **`[S2]`** — 阶段二（强化）创建或增强
- **`[S3]`** — 阶段三（规模化）创建或增强
- **`[S1+]`** / **`[S2+]`** / **`[S3+]`** — 该阶段创建，后续阶段可继续增强

**读法**：阶段 N 的任务**只**创建本阶段标注的文件，**禁止**越阶段创建。

```text
RapidGate/
├── Cargo.toml                       [S1]   # 单 crate 清单（依赖按 §3 锁定）
├── Cargo.lock                       [S1]   # 必须提交
├── rustfmt.toml                     [S1]   # 代码风格
├── rust-toolchain.toml              [S1]   # 固定 Rust stable
├── .env.example                     [S1]   # 环境变量示例（无真实值）
├── .gitignore                       [S1]
├── LICENSE                          [S1]   # Apache-2.0
├── README.md                        [S3]   # 用户明确要求时再创建
├── AGENTS.md                        [S1]   # 协作规范（已存在）
├── deny.toml                        [S1]   # cargo-deny 许可证白名单（已存在）
│
├── docs/                            [S1]   # 项目文档
│   ├── CI-CD-SETUP.md               [S1]   # （已存在）
│   ├── rapidgate-spec.md            [S1]   # 本文件
│   ├── rapidgate-prompt.md          [S2]   # 三阶段流水线（升级版）
│   ├── rapidgate-stage3.md          [S3]   # 阶段三详细任务
│   ├── ARCHITECTURE.md              [S3]   # 架构图 + 决策记录
│   └── OPERATIONS.md                [S3]   # 运维手册（部署/监控/排障）
│
├── config/                          [S1]   # 配置文件（运行时按 RGD_CONFIG_DIR 解析）
│   ├── default.yaml                 [S1]   # 默认配置
│   ├── development.yaml             [S1]   # 开发环境覆盖
│   ├── production.yaml              [S1]   # 生产环境覆盖
│   ├── providers/                   [S3]   # 各 provider 独立配置
│   │   ├── openai.yaml              [S3]
│   │   ├── anthropic.yaml           [S3]
│   │   ├── gemini.yaml              [S3]
│   │   └── local.yaml               [S3]
│   └── routes/                      [S1]   # 路由配置（可独立热重载）
│       ├── v1.yaml                  [S1]   # 兼容 OpenAI v1
│       ├── v2.yaml                  [S3]   # v2 路由（向后兼容）
│       └── experiments/             [S3]   # 灰度实验
│           └── chat-v2-rollout.yaml [S3]
│
├── src/                             [S1]   # 全部源代码（单 crate）
│   ├── main.rs                      [S1+]  # 程序入口：信号、启动、优雅关闭
│   ├── lib.rs                       [S3]   # 暴露 lib 入口（供插件/CLI 复用）
│   │
│   ├── core/                        [S1+]  # 核心业务逻辑（无 I/O / 无网络 / 无 tokio）
│   │   ├── mod.rs                   [S1]
│   │   ├── error.rs                 [S1]   # CoreError 枚举 + From 转换
│   │   ├── config/                  [S1+]  # 配置数据模型（仅 serde 定义，不加载）
│   │   │   ├── mod.rs               [S1]
│   │   │   ├── gateway.rs           [S1]   # GatewayConfig
│   │   │   ├── route.rs             [S1]   # RouteConfig + RouteMatch
│   │   │   ├── upstream.rs          [S1]   # UpstreamConfig + LoadBalancer
│   │   │   └── provider.rs          [S3]   # LLM Provider 配置（多 provider）
│   │   ├── routing/                 [S1+]  # 路由匹配引擎
│   │   │   ├── mod.rs               [S1]
│   │   │   ├── matcher.rs           [S1]   # 路径 / Method / Host / Header
│   │   │   ├── table.rs             [S1]   # ArcSwap 支持热重载
│   │   │   └── canary.rs            [S3]   # 灰度权重
│   │   ├── proxy/                   [S1+]  # 转发逻辑
│   │   │   ├── mod.rs               [S1]   # Forwarder trait + 默认实现
│   │   │   ├── transformer.rs       [S1]   # 请求/响应 Header 与路径转换
│   │   │   ├── stream.rs            [S1]   # SSE 流式转发（chunked 透传）
│   │   │   └── ws.rs                [S3]   # WebSocket 转发
│   │   ├── auth/                    [S1+]  # 认证抽象
│   │   │   ├── mod.rs               [S1]   # Authenticator trait
│   │   │   ├── apikey.rs            [S1]   # API Key 校验（常量时间比较）
│   │   │   ├── jwt.rs               [S2]   # JWT 校验（HS256 / RS256）
│   │   │   └── oauth2.rs            [S3]   # OAuth2 流程
│   │   ├── ratelimit/               [S1+]  # 限流算法
│   │   │   ├── mod.rs               [S1]   # RateLimiter trait
│   │   │   ├── token_bucket.rs      [S1]   # 令牌桶（允许突发）
│   │   │   ├── sliding_window.rs    [S1]   # 滑动窗口
│   │   │   ├── local_store.rs       [S1]   # 进程内存储（Moka 封装）
│   │   │   └── redis_store.rs       [S3]   # 分布式限流
│   │   ├── breaker/                 [S1]   # 熔断器
│   │   │   ├── mod.rs               [S1]
│   │   │   ├── state.rs             [S1]   # Closed / Open / HalfOpen 状态机
│   │   │   └── breaker.rs           [S1]   # 计数器 + 状态转换
│   │   ├── audit/                   [S2+]  # 审计与计费
│   │   │   ├── mod.rs               [S2]   # AuditEvent 结构
│   │   │   ├── counter.rs           [S2]   # token 计数（SSE 增量累加）
│   │   │   └── sink.rs              [S3]   # ES / Loki / ClickHouse 写入
│   │   ├── observability/           [S1+]  # 可观测性抽象
│   │   │   ├── mod.rs               [S1]
│   │   │   ├── trace.rs             [S2]   # W3C tracecontext
│   │   │   ├── metrics.rs           [S3]   # prometheus 实现
│   │   │   └── otel.rs              [S3]   # OpenTelemetry 导出
│   │   ├── plugins/                 [S3]   # 插件系统
│   │   │   ├── mod.rs               [S3]
│   │   │   ├── trait.rs             [S3]   # Plugin trait
│   │   │   ├── registry.rs          [S3]   # 插件注册表
│   │   │   ├── native.rs            [S3]   # native 插件加载
│   │   │   └── wasm.rs              [S3]   # WASM 沙箱加载
│   │   ├── canary/                  [S3]   # 灰度发布
│   │   │   ├── mod.rs               [S3]
│   │   │   ├── policy.rs            [S3]   # 权重/Header/Cookie 策略
│   │   │   └── sticky.rs            [S3]   # 会话黏性
│   │   └── util/                    [S1+]  # 工具函数
│   │       ├── mod.rs               [S1]
│   │       ├── path.rs              [S1]   # 路径规范化
│   │       ├── hash.rs              [S1]   # 普通哈希
│   │       └── consistent_hash.rs   [S2]   # 一致性哈希
│   │
│   ├── service/                     [S1+]  # 框架集成层（axum + tokio + 文件 I/O）
│   │   ├── mod.rs                   [S1]
│   │   ├── server.rs                [S1+]  # axum::serve + graceful shutdown
│   │   ├── state.rs                 [S1]   # AppState
│   │   ├── handler.rs               [S1]   # 5 个 axum handler
│   │   ├── middleware/              [S1+]  # axum middleware
│   │   │   ├── mod.rs               [S1]
│   │   │   ├── trace.rs             [S1]   # 生成 / 提取 X-Request-Id
│   │   │   ├── auth.rs              [S2]   # 实际校验
│   │   │   ├── ratelimit.rs         [S2]   # 实际限流
│   │   │   ├── audit.rs             [S2]   # 写审计
│   │   │   └── recovery.rs          [S3]   # panic 恢复
│   │   ├── config_loader.rs         [S1]   # YAML 加载 + dotenvy
│   │   ├── hot_reload.rs            [S2]   # ArcSwap + 校验失败回滚
│   │   ├── upstream_pool.rs         [S1]   # reqwest::Client 池
│   │   ├── providers/               [S3]   # 多 provider 适配
│   │   │   ├── mod.rs               [S3]
│   │   │   ├── openai.rs            [S3]
│   │   │   ├── anthropic.rs         [S3]
│   │   │   ├── gemini.rs            [S3]
│   │   │   └── local.rs             [S3]
│   │   ├── config_center/           [S3]   # ETCD / Consul
│   │   │   ├── mod.rs               [S3]
│   │   │   ├── etcd.rs              [S3]
│   │   │   └── consul.rs            [S3]
│   │   ├── admin/                   [S3]   # admin API
│   │   │   ├── mod.rs               [S3]
│   │   │   ├── routes.rs            [S3]
│   │   │   └── auth.rs              [S3]
│   │   ├── error.rs                 [S1]   # ServiceError + IntoResponse
│   │   └── telemetry.rs             [S1]   # tracing 初始化
│   │
│   └── bin/                         [S3]   # CLI 工具
│       └── cli.rs                   [S3]   # rapidgate-cli
│
├── plugins/                         [S3]   # 内置 / 示例插件
│   ├── README.md                    [S3]
│   ├── add-request-id/              [S3]
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── cache-response/              [S3]
│       ├── Cargo.toml
│       └── src/lib.rs
│
├── tests/                           [S1+]  # 集成测试
│   ├── common/
│   │   └── mod.rs                   [S1]   # spawn_app() 工具
│   ├── routing.rs                   [S1]   # 路由匹配优先级
│   ├── proxy_stream.rs              [S2]   # SSE 流式转发
│   ├── auth.rs                      [S2]   # 认证成功/失败/过期
│   ├── ratelimit.rs                 [S2]   # 令牌桶 + 滑动窗口
│   ├── hot_reload.rs                [S2]   # 热重载与回滚
│   ├── error.rs                     [S1]   # 错误响应格式
│   ├── canary.rs                    [S3]   # 灰度权重
│   ├── plugins.rs                   [S3]   # 插件加载与执行
│   ├── providers/                   [S3]   # 多 provider
│   │   ├── openai_compat.rs         [S3]
│   │   └── failover.rs              [S3]
│   └── e2e/                         [S3]   # 端到端
│       ├── openai_compat.rs         [S3]
│       └── failover.rs              [S3]
│
├── benches/                         [S2]   # 基准测试（首版不创建）
│   ├── routing_bench.rs             [S2]
│   ├── proxy_bench.rs               [S2]
│   ├── canary_bench.rs              [S3]
│   └── plugins_bench.rs             [S3]
│
├── deploy/                          [S3]   # 部署与运维
│   ├── docker/
│   │   ├── Dockerfile               [S3]
│   │   └── docker-compose.yaml      [S3]
│   ├── k8s/
│   │   ├── deployment.yaml          [S3]
│   │   ├── service.yaml             [S3]
│   │   ├── ingress.yaml             [S3]
│   │   └── hpa.yaml                 [S3]
│   ├── systemd/
│   │   └── rapidgate.service        [S3]
│   └── prometheus/
│       ├── rules.yaml               [S3]
│       └── alerts.yaml              [S3]
│
├── scripts/                         [S3]   # 运维脚本
│   ├── bench.sh                     [S3]
│   ├── fuzz.sh                      [S3]
│   └── load-test.sh                 [S3]
│
└── .github/                         [S1]   # （已存在，不重复）
    ├── workflows/
    ├── PULL_REQUEST_TEMPLATE.md
    └── release.yml
```

**目录树使用约束**：

- `src/core/` 与 `src/service/` 是**逻辑命名空间**，不是 Cargo crate。`src/main.rs` 通过 `mod core;` 与 `mod service;` 引入。
- 整个项目**始终保持单 crate**（不拆 workspace）。`plugins/` 下的子 crate 由发布时按需启用。
- 阶段 N 的 prompt **禁止**创建本阶段未标注的文件（无论是 `[S1]` 阶段写 `[S3]` 目录，还是反过来）。
- `[Sn+]` 标记表示该阶段创建后，后续阶段可继续增强（**增强** ≠ **删除**或**大改**结构）。
- `benches/` 阶段一**不**创建（spec 旧版曾写"首版不创建 benches/"，本版延续此约束；阶段二首次创建）。

---

## 3. 依赖清单（按用户提示词锁定 · 三阶段分组）

`Cargo.toml` 的 `[dependencies]` 表是**依赖锁定的唯一权威**。本节按**阶段分组**列依赖：

- **§3.1** 阶段一就绪依赖（首版 Cargo.toml **必须**完整列出）
- **§3.2** 阶段二新增依赖（chore(deps) commit 引入）
- **§3.3** 阶段三新增依赖（chore(deps) commit 引入）
- **§3.4** 全程禁止的 crate

### 3.1 阶段一依赖（首版必须完整）

```toml
[package]
name = "rapidgate"
version = "2026.06.1"           # 严格遵循 AGENTS.md §6 的 YYYY.MM.N，禁止 0.x.y
edition = "2021"
license = "Apache-2.0"
description = "High-performance unified LLM API gateway written in Rust"
repository = "https://github.com/SharkMI-0x7E/RapidGate"

[dependencies]
# 异步运行时
tokio = { version = "1.42", features = ["rt", "rt-multi-thread", "macros", "time", "sync", "signal"] }

# HTTP 框架
axum = { version = "0.8", features = ["macros", "ws"] }

# HTTP 客户端
reqwest = { version = "0.12", features = ["json", "stream", "rustls-tls"], default-features = false }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# 配置
dotenvy = "0.15"

# 日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# 错误处理
thiserror = "2.0"
anyhow = "1.0"

# 工具
bytes = "1.10"
http = "1.2"
http-body-util = "0.1"
uuid = { version = "1.12", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
arc-swap = "1.7"             # 路由表无锁热重载
notify = "6.1"               # 配置文件监听
moka = { version = "0.12", features = ["future"] }   # 限流 / token 缓存
sha2 = "0.10"                # API Key 哈希
subtle = "2.6"               # 常量时间比较
regex = "1.11"               # 路由路径正则匹配
tower = { version = "0.5", features = ["util", "timeout", "limit"] }
tower-http = { version = "0.6", features = ["trace", "request-id", "util"] }
hyper = { version = "1.5", features = ["server", "client", "http1", "http2"] }
hyper-util = { version = "0.1", features = ["server", "client", "tokio"] }
futures = "0.3"
futures-util = "0.3"
```

### 3.2 阶段二新增依赖（chore(deps) commit 引入）

阶段一完成后，在新 commit `chore(deps): add stage-2 dependencies` 中追加：

```toml
# JWT 认证（apikey 是 S1 已经实现的）
jsonwebtoken = "9.3"

# 审计日志结构化
serde_with = "3.7"

# 基准测试（dev-dependency，由 benches/ 引入）
criterion = { version = "0.5", features = ["html_reports"] }

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
```

### 3.3 阶段三新增依赖（chore(deps) commit 引入）

阶段二完成后，在新 commit `chore(deps): add stage-3 dependencies` 中追加：

```toml
# 分布式限流
redis = { version = "0.27", features = ["tokio-comp", "connection-manager", "cluster"] }

# 可观测性
prometheus = "0.14"
tracing-opentelemetry = "0.23"
opentelemetry = { version = "0.23", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.23", features = ["grpc-tonic", "metrics"] }
opentelemetry_sdk = { version = "0.23", features = ["rt-tokio"] }

# 配置中心
etcd-client = "0.12"

# 插件沙箱
wasmtime = "19.0"

# admin API GraphQL
async-graphql = "7.0"
async-graphql-axum = "7.0"

# OAuth2
oauth2 = "4.4"

# 端到端负载测试（dev-dependency）
goose = "0.16"
```

### 3.4 全程禁止添加

- `argon2` / `bcrypt` — LLM 网关无密码哈希需求
- `config` crate — 用 `serde_yaml` + `dotenvy` 足矣
- 任何过程宏 crate（`rapidgate-macros` 占位直接禁掉）
- `tokio` 全 features 之外的其他运行时（`async-std` / `smol` 等）
- 任何**未在 §3.1/§3.2/§3.3 出现**的 crate（违反 `AGENTS.md §7.1` 的"依赖锁定唯一源"硬约束）

---

## 4. `core` 模块详解（按文件）

### 4.1 `core/error.rs` — 核心错误

```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("config error: {0}")]              Config(String),
    #[error("route not found: {0}")]           RouteNotFound(String),
    #[error("auth failed: {0}")]               Auth(String),
    #[error("rate limit exceeded")]            RateLimited,
    #[error("circuit breaker open: {0}")]      BreakerOpen(String),
    #[error("upstream unreachable: {0}")]       UpstreamUnreachable(String),
    #[error("upstream timeout after {0:?}")]   UpstreamTimeout(std::time::Duration),
    #[error("invalid request: {0}")]           BadRequest(String),
    #[error("internal error: {0}")]            Internal(String),
}
```

**关键约定**：`CoreError` **不**实现 `IntoResponse`，这是 `service/error.rs` 的事。`core` 层保持纯净。

### 4.2 `core/config/` — 配置数据模型

- 所有结构体均 `#[derive(Deserialize)]` + `#[serde(deny_unknown_fields)]`。
- 字段全部 snake_case。
- 敏感字段（`api_key`）用 `String` 类型，但**不**出现在反序列化默认值里；运行时必须由环境变量注入（`RGD_<UPPER_SNAKE_KEY>` 命名约定）。
- **Provider 配置必须可扩展**：通过 `#[serde(tag = "type", rename_all = "snake_case")]` 的 enum 支持 OpenAI / Anthropic / Gemini 后续直接加 variant。

### 4.3 `core/routing/` — 路由引擎

- `Router` 内部维护 `Arc<ArcSwap<RouteTable>>`，`RouteTable` 不可变（`&self`），切换时整体替换。
- **匹配顺序**：先精确（`path == a && method == b`）→ 再前缀 → 再正则。**冲突时取先注册者**，并在加载时打印 warning。
- **禁止**支持通配符 `*` 路径段（v1 不开放）。
- `matcher.rs` 必须**支持** `Method`、`Path`（精确/前缀/正则）、`Host`、`Header`（精确 + 正则）、`Query` 参数。

### 4.4 `core/proxy/` — 转发

- `Forwarder` trait 接收上游 `reqwest::RequestBuilder`，返回 `reqwest::Response`（流式）。
- **核心要求**：拿到上游 `Response` 后**直接透传 body 流**，禁止 `.bytes().await?` 一次性缓冲。
- `stream.rs` 提供 `into_axum_body()` 把 `reqwest::Body`（实际是 `Body::Stream`）转成 axum 的 `Body`。

### 4.5 `core/auth/`、`core/ratelimit/`、`core/breaker/`、`core/audit/`

- 全部以 **trait + 默认实现** 形式暴露，**不**依赖任何 tokio 句柄或全局状态。
- `RateLimiter::check(key) -> Result<(), CoreError>`，**不**与 axum 耦合。
- `Breaker::call(future) -> Result<T, CoreError>`，**不**与 axum 耦合。
- `AuditEvent` 包含 `trace_id`、`user_id`、`provider`、`model`、`prompt_tokens`、`completion_tokens`、`latency_ms`、`status`。

### 4.6 `core/observability/`

- `TraceId::new()`：生成 32 字节十六进制（等价 W3C tracecontext 16 字节）。
- `Metrics` trait：`inc_request(route, status)`、`observe_latency(route, ms)`，**不**绑死 prometheus，阶段五再加 prometheus 实现。

### 4.7 `core/util/`

- `path.rs`：`normalize("/a/./b/../c") -> "/a/c"`，处理 `..` 与 `.` 与多余 `/`。
- `hash.rs`：实现 `consistent_hash(key, buckets) -> usize`，负载均衡时使用。

---

## 5. `service` 模块详解（按文件）

### 5.1 `service/error.rs` — 网关错误 + IntoResponse

**这是整个网关与 HTTP 协议的唯一耦合点**，签名必须如下：

```rust
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)] Core(#[from] CoreError),
    #[error("upstream http error: status={status}")] Upstream { status: u16, body: Bytes },
    #[error("io error: {0}")] Io(#[from] std::io::Error),
}

impl axum::response::IntoResponse for ServiceError {
    fn into_response(self) -> axum::response::Response {
        let (status, code) = match &self {
            ServiceError::Core(CoreError::RouteNotFound(_))    => (StatusCode::NOT_FOUND, "route_not_found"),
            ServiceError::Core(CoreError::Auth(_))             => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ServiceError::Core(CoreError::RateLimited)         => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            ServiceError::Core(CoreError::BreakerOpen(_))      => (StatusCode::SERVICE_UNAVAILABLE, "breaker_open"),
            ServiceError::Core(CoreError::UpstreamUnreachable(_)) => (StatusCode::BAD_GATEWAY, "upstream_unreachable"),
            ServiceError::Core(CoreError::UpstreamTimeout(_))  => (StatusCode::GATEWAY_TIMEOUT, "upstream_timeout"),
            ServiceError::Core(CoreError::BadRequest(_))       => (StatusCode::BAD_REQUEST, "bad_request"),
            ServiceError::Upstream { status, .. }              => (StatusCode::from_u16(*status).unwrap_or(BAD_GATEWAY), "upstream_error"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        };
        // 统一 JSON 错误体：{ "error": { "code": "...", "message": "..." } }
        let body = serde_json::json!({
            "error": { "code": code, "message": self.to_string() }
        });
        (status, [("content-type", "application/json")], body.to_string()).into_response()
    }
}
```

**所有 handler 的返回类型必须**：`Result<axum::response::Response, ServiceError>`。

### 5.2 `service/state.rs` — AppState

```rust
pub struct AppState {
    pub route_table: arc_swap::ArcSwap<RouteTable>,
    pub upstreams:   moka::future::Cache<UpstreamId, reqwest::Client>,
    pub limiters:    moka::future::Cache<LimitKey, Box<dyn RateLimiter>>,
    pub audit_tx:    tokio::sync::mpsc::UnboundedSender<AuditEvent>,
    pub config_dir:  std::path::PathBuf,
}
```

**所有字段必须 Arc 共享**：`Arc<AppState>` 通过 `axum::extract::State` 注入。

### 5.3 `service/handler.rs`

首版**至少**实现以下路由：

| Method | Path | 描述 |
| --- | --- | --- |
| `POST` | `/v1/chat/completions` | OpenAI 兼容聊天补全（**含 SSE 流式**） |
| `POST` | `/v1/embeddings` | OpenAI 兼容 embedding |
| `GET`  | `/v1/models` | 列出可用模型（聚合所有 provider） |
| `GET`  | `/healthz` | 存活探针（不查上游） |
| `GET`  | `/readyz`  | 就绪探针（检查上游池 + 配置有效性） |

### 5.4 `service/middleware/`

执行顺序（**由外到内**）：

1. `trace`（生成 `X-Request-Id`）
2. `auth`（校验 token / api key）
3. `ratelimit`（按 user_id 限流）
4. `audit`（记录请求开始 / 结束 + token 用量）
5. handler
6. 响应回到 `audit`（落库）→ `ratelimit`（补计）→ `auth`（无操作）→ `trace`（打日志）

### 5.5 `service/config_loader.rs` + `service/hot_reload.rs`

加载流程：

1. 读 `RGD_CONFIG_DIR`（默认 `./config`）。
2. 加载 `default.yaml` → 用 `development.yaml` 或 `production.yaml` 覆盖（按 `RGD_ENV` 选择）→ 加载 `routes/*.yaml`。
3. **校验**（参考 §7）：失败则**保留旧配置**，写 error 日志，**不** panic。
4. `notify` 监听文件变化 → 重新跑 1~3 → 校验通过则 `route_table.store(Arc::new(new))`，失败则**不替换**。

**in-flight 请求处理**：`ArcSwap::load()` 拿到当前快照后，整个请求生命周期都用这个 `Arc`，新配置不会影响已开始的请求。

### 5.6 `service/server.rs` + `main.rs`

```rust
// 启动伪代码（main.rs）
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let _guard = service::telemetry::init();

    let config = service::config_loader::load().await?;
    let state  = service::state::build(config).await?;
    let app    = service::server::router(state.clone());

    let listener = tokio::net::TcpListener::bind(&state.config.listen).await?;
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received, draining...");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}
```

**硬性要求**：

- 监听 `SIGINT` / `SIGTERM`，等待**已建立连接完成**（`axum::serve` 的 graceful shutdown 默认行为）。
- 主进程退出码：成功 `0`，配置错误 `78`（`sysexits.h::EX_CONFIG`），其他错误 `1`。

---

## 6. 配置文件契约

### 6.1 `config/default.yaml`（最小骨架）

```yaml
gateway:
  listen: "0.0.0.0:8080"
  request_timeout_ms: 60000
  max_body_bytes: 52428800       # 50 MiB
  shutdown_timeout_ms: 15000

logging:
  level: "info"                   # trace | debug | info | warn | error
  format: "pretty"                # pretty | json

defaults:
  rate_limit:
    algorithm: "token_bucket"     # token_bucket | sliding_window
    rps: 10
    burst: 20
  breaker:
    failure_threshold: 5
    open_duration_ms: 30000

upstreams: []                     # 由 routes/v1.yaml 填充
```

### 6.2 `config/routes/v1.yaml`（示例）

```yaml
routes:
  - name: "openai-chat"
    match:
      method: POST
      path: "/v1/chat/completions"
    upstream:
      provider: openai
      base_url: "${RGD_OPENAI_BASE_URL}"   # 必须从环境变量解析
      api_key:  "${RGD_OPENAI_API_KEY}"
    auth: { type: "bearer" }
    rate_limit: { rps: 5, burst: 10 }
```

**约定**：

- 任何 `${VAR}` 占位符在加载时由环境变量展开；**缺失则报错并保留旧配置**。
- 路由按数组顺序匹配，**先注册先生效**。
- `upstream.base_url` **必须**通过白名单（`config/default.yaml` 的 `gateway.upstream_allowlist`）校验；不在白名单的 base_url **拒绝加载**。

---

## 7. 配置校验规则（加载时全部执行，失败则拒绝新配置）

| 规则 | 说明 |
| --- | --- |
| R-1 | 所有 YAML 文件能被 `serde_yaml` 解析为对应结构体 |
| R-2 | 所有 `${VAR}` 占位符在 `process::env()` 中能找到 |
| R-3 | `upstream.base_url` 在 `gateway.upstream_allowlist` 内 |
| R-4 | `route.match.method` 是合法 HTTP method |
| R-5 | `route.match.path` 是合法 URL path（不以 `/` 结尾，正则不包含未转义 `.`） |
| R-6 | `route.name` 在同一文件中不重复 |
| R-7 | `route.rate_limit.rps > 0 && burst > 0` |
| R-8 | `upstream.api_key` 长度 ≥ 16 字节 |

**违反任何一条** → 保留旧配置 + `tracing::error!` 输出违规项 + 退出码 `78`。

---

## 8. SSRF 防护（首版硬性要求）

`core/proxy/mod.rs` 在构造上游请求时必须执行：

1. 解析 `base_url` 的 host。
2. DNS 解析 host 拿到所有 A/AAAA 记录。
3. 检查每个 IP 是否在 `RGD_IP_BLOCKLIST`（CIDR 列表）内或属于私有 / 回环 / 链路本地段。
4. **任一 IP 命中** → 拒绝本次请求，返回 `ServiceError::Core(CoreError::BadRequest("blocked upstream"))`。
5. 真实发起请求时使用**校验后的 IP** + `Host` Header（防 DNS rebinding）。

**默认 blocklist**（`config/default.yaml`）：

- `127.0.0.0/8`、`10.0.0.0/8`、`172.16.0.0/12`、`192.168.0.0/16`
- `169.254.0.0/16`（云元数据）
- `::1/128`、`fc00::/7`、`fe80::/10`

---

## 9. 演进路线（3 阶段流水线）

项目按**三阶段流水线**推进，与 `AGENTS.md §8.5` 和 `docs/rapidgate-prompt.md` 一一对应。

| 阶段 | 提示词文件 | 触发条件 | 启用项（对应目录树 [S?] 标注） |
| --- | --- | --- | --- |
| **阶段一** | `rapidgate-prompt.md §2` | 首版（2026.06.x） | 单 crate + 内部 mod + 5 个 handler + 配置加载 + 基础审计 + 简单白名单防护。**对应所有 `[S1]` / `[S1+]` 标注** |
| **阶段二** | `rapidgate-prompt.md §3` | 阶段一 6 步自检全过后 | 性能优化 / 安全加固 / 测试覆盖 / 可观测性完善 / 错误信息可读性 / 跨切关注点。**对应所有 `[S2]` / `[S2+]` 标注**（含 `benches/` 首次创建） |
| **阶段三** | `docs/rapidgate-stage3.md` | 阶段二交付清单确认后 | 多 provider / 插件系统 / 分布式 / 灰度 / admin API / WebSocket / OAuth2 / 部署运维。**对应所有 `[S3]` / `[S3+]` 标注** |

**每阶段交付前**：

- `AGENTS.md §3.1` 6 步自检**全部** PASS
- 阶段 N+1 标注的目录 / 文件**禁止**在阶段 N 提前创建
- 所有变更走 commit（参见 `AGENTS.md §1.1`），阶段二、三的所有 commit body 标注 `Refine: stage-N <commit-sha>`

**每阶段交付后**：

- `Cargo.toml` 的 `version` 按 `YYYY.MM.N` 递增（手动，见 `AGENTS.md §6`）
- `docs/rapidgate-spec.md` 同步更新目录树与依赖表（如有调整）
- `docs/rapidgate-prompt.md` 同步更新阶段允许/禁止内容
- `docs/rapidgate-stage3.md`（阶段三）同步更新具体强化项

**阶段结束判据对照表**：

| 维度 | 阶段一结束 | 阶段二结束 | 阶段三结束 |
| --- | --- | --- | --- |
| 代码可编译 | ✅ | ✅ | ✅ |
| 6 步自检 | ✅ | ✅ | ✅ |
| 单元 + 集成测试 | 骨架（1~2 冒烟） | 完整覆盖 | 完整 + 模糊测试 |
| OpenAI 兼容 | ✅ | ✅ | ✅ + 多 provider |
| SSE 流式 | ✅ | ✅ | ✅ |
| 配置热重载 | ✅ 加载+切换 | ✅ 完整回滚 | ✅ + 配置中心 |
| SSRF 防护 | 白名单 | 完整 | 完整 |
| 限流 | 进程内 | 进程内 | 进程内 + Redis 集群 |
| 鉴权 | API Key | API Key + JWT | + OAuth2 |
| 可观测性 | tracing | + W3C trace | + Prometheus + OTel |
| 插件 | ❌ | ❌ | ✅ trait + native + WASM |
| 灰度 | ❌ | ❌ | ✅ |
| 部署 | 本地 | 本地 | Docker + K8s + systemd |

---

## 10. 与 AGENTS.md 的对齐清单

| AGENTS.md 条款 | 在本文件中的落实位置 |
| --- | --- |
| §1 提交规范 | 提示词文件 §6 强约束 commit 粒度 |
| §2 PR 要求 | `cargo fmt` / `clippy` / `test` / 无 `unsafe` 全在本文件范围内生效 |
| §3 构建命令 | 提示词文件 §7 列出本地校验顺序 |
| §4 安全策略 | §8 SSRF 防护 + 敏感字段环境变量注入 |
| §6 版本号 | §3 `version = "2026.06.1"`，禁止 0.x.y |
| §7 技术栈 | §3 依赖锁定，axum 0.8 / tokio 1.42 / reqwest 0.12（按用户提示词） |
| §8 AI 协作 | 提示词文件整篇覆盖 |

---

## 11. 自检清单（落地前最后一遍）

- [ ] §3 依赖表是**完整**的，`Cargo.toml` 没有超出该表的 crate
- [ ] §2 目录树里**没有** `crates/` / `benches/` / `examples/` / `rapidgate-macros` / `rapidgate-models`
- [ ] `core/` 下**没有**任何 `tokio` / `reqwest` / `notify` 引用
- [ ] 所有 handler 签名都是 `Result<Response, ServiceError>`
- [ ] `ServiceError::IntoResponse` 输出 JSON 格式 `{ "error": { "code", "message" } }`
- [ ] 配置加载失败时**不 panic**，保留旧配置
- [ ] 流式响应路径**不**调用 `.bytes().await?`
- [ ] `version` 是 `YYYY.MM.N` 格式，不是 `0.x.y`
- [ ] `cargo fmt -- --check` 通过
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` 通过
- [ ] `cargo test --all-features` 通过
- [ ] 无 `unsafe` 块
