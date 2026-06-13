# RapidGate 阶段一（基础落地）Spec

## Why

把 `docs/rapidgate-spec.md` §2 中**所有 `[S1]` / `[S1+]` 标注**的目录与文件变成**可编译、有签名、有最小逻辑**的代码骨架，使 `cargo build --all-targets --all-features`、`cargo test --all-features`、`AGENTS.md §3.1` 6 步自检全部 PASS，并为阶段二强化提供稳定地基。

## What Changes

- **新增** 完整的 `src/core/` 模块骨架（`error` / `config` / `routing` / `proxy` / `auth` / `ratelimit` / `breaker` / `observability` / `util`），无 I/O / 无 tokio / 无网络。
- **新增** 完整的 `src/service/` 模块骨架（`server` / `state` / `handler` / `middleware` / `config_loader` / `upstream_pool` / `error` / `telemetry`），负责 axum 集成、配置加载、文件监听。
- **新增** `src/main.rs` 程序入口（信号、启动、优雅关闭骨架）。
- **新增** 配置文件 `config/default.yaml` + `config/development.yaml` + `config/production.yaml` + `config/routes/v1.yaml`，运行时由 `RGD_CONFIG_DIR` 解析。
- **新增** 集成测试骨架 `tests/common/mod.rs` + `tests/routing.rs` + `tests/error.rs`（happy-path 冒烟）。
- **新增** `Cargo.toml` + `Cargo.lock` + `rustfmt.toml` + `rust-toolchain.toml` + `.env.example` + `.gitignore`，依赖**严格**按 spec §3.1 锁定。
- **新增** `docs/CI-CD-SETUP.md` 占位（[S1] 标注已存在，不动）。

**未变更**：阶段二/三的 `[S2]` / `[S3]` 标注文件**禁止**在阶段一创建。

## Impact

- **新增 specs 能力**：
  - `core-error`（`CoreError` 枚举）
  - `core-config`（`GatewayConfig` / `RouteConfig` / `UpstreamConfig`）
  - `core-routing`（`Router` / `RouteTable` / `Matcher`）
  - `core-proxy`（`Forwarder` trait + 默认实现 + `into_axum_body`）
  - `core-auth`（`Authenticator` trait + `ApiKey` 常量时间比较）
  - `core-ratelimit`（`RateLimiter` trait + 令牌桶 + 滑动窗口 + Moka 存储）
  - `core-breaker`（`Breaker` + Closed/Open/HalfOpen 状态机）
  - `core-observability`（`TraceId` + `Metrics` trait）
  - `core-util`（路径规范化 + 普通哈希）
  - `service-state`（`AppState`）
  - `service-server`（axum::serve + graceful shutdown）
  - `service-handler`（5 个 axum handler：chat / embeddings / models / healthz / readyz）
  - `service-middleware-trace`（X-Request-Id）
  - `service-config-loader`（YAML + dotenvy + ArcSwap）
  - `service-upstream-pool`（reqwest::Client 池 + SSRF 白名单基础版）
  - `service-error`（`ServiceError` + `IntoResponse`）
  - `service-telemetry`（tracing 初始化）
- **新增代码文件**：
  - `Cargo.toml` / `Cargo.lock` / `rustfmt.toml` / `rust-toolchain.toml` / `.env.example` / `.gitignore`
  - `src/main.rs`
  - `src/core/**/*.rs`（约 15 个文件）
  - `src/service/**/*.rs`（约 10 个文件）
  - `config/*.yaml`（4 个）
  - `tests/common/mod.rs` + `tests/routing.rs` + `tests/error.rs`
- **未涉及**：`tests/proxy_stream.rs` / `tests/auth.rs` / `tests/ratelimit.rs` / `tests/hot_reload.rs`（[S2]）；`benches/`（[S2] 首次创建）；`core/audit/`（[S2+]）；`service/providers/` `service/admin/` `service/config_center/`（[S3]）；`plugins/` `deploy/` `scripts/`（[S3]）。

## ADDED Requirements

### Requirement: 依赖锁定 spec §3.1

`Cargo.toml` 的 `[dependencies]` 表**必须**严格按 spec §3.1 列出。**禁止**添加 spec §3.1 之外的任何 crate（含 `argon2` / `bcrypt` / `config` / `rapidgate-macros` 等 §3.4 禁项）。

#### Scenario: 依赖审查
- **WHEN** 提交 `chore(deps): update Cargo.toml per spec §3.1`
- **THEN** `Cargo.toml` 仅含 spec §3.1 列出的依赖；`cargo tree` 无未列出的传递依赖被 `features` 显式拉入。

### Requirement: `core` 模块无 I/O / 无网络 / 无 tokio

`src/core/**/*.rs` **禁止**出现 `use tokio` / `use reqwest` / `use notify` / `use std::net` / `use hyper` 等 I/O 引用。模块**只**做数据建模、trait 抽象、纯算法。

#### Scenario: core 纯净性验证
- **WHEN** 执行 `grep -rn "use tokio\|use reqwest\|use notify" src/core/`
- **THEN** 输出为空。

### Requirement: `ServiceError::IntoResponse` 输出统一 JSON

所有 axum handler 的返回类型**必须**为 `Result<axum::response::Response, ServiceError>`。`ServiceError` 实现 `IntoResponse` 输出 `{ "error": { "code": "...", "message": "..." } }`。

#### Scenario: 错误响应格式
- **WHEN** 命中 `CoreError::RouteNotFound` 走 handler
- **THEN** 响应体 JSON 含 `"code": "route_not_found"`，HTTP 状态码 404。

### Requirement: 配置加载失败保留旧配置

`service/config_loader::load()` 必须实现：读 `RGD_CONFIG_DIR`（默认 `./config`）→ 加载 `default.yaml` → 用 `development.yaml` 或 `production.yaml` 覆盖（按 `RGD_ENV`）→ 加载 `routes/*.yaml` → 校验（spec §7 全部 R-1~R-8）→ **失败则保留旧配置 + 写 error 日志，**不 panic**。

#### Scenario: 占位符缺失
- **WHEN** `routes/v1.yaml` 含 `${RGD_OPENAI_API_KEY}` 但环境变量未设
- **THEN** 校验失败，**保留旧配置**，tracing::error 输出违规项，**进程不退出**。

### Requirement: 5 个 handler 至少返回非 5xx

`service/handler.rs` **必须**实现以下 5 个路由的 handler（happy path 至少返回 200/4xx，**不**返回 5xx）：

| Method | Path | 状态 |
| --- | --- | --- |
| `POST` | `/v1/chat/completions` | OpenAI 兼容聊天补全（含 SSE 流式） |
| `POST` | `/v1/embeddings` | OpenAI 兼容 embedding |
| `GET`  | `/v1/models` | 列出可用模型 |
| `GET`  | `/healthz` | 存活探针（不查上游） |
| `GET`  | `/readyz`  | 就绪探针（检查配置有效性） |

#### Scenario: 5 路由冒烟
- **WHEN** `tests/routing.rs` 启动服务 + curl 5 路由
- **THEN** 全部响应状态码 ∈ {200, 400, 401, 404}，**不出现 5xx**。

### Requirement: 流式响应不缓冲

`core/proxy/stream.rs` **必须**提供 `into_axum_body()` 把 `reqwest::Response` 的 `Body`（实际是 `Body::Stream`）转成 axum 的 `Body`，**禁止**调用 `.bytes().await?` 一次性缓冲。

#### Scenario: SSE 透传
- **WHEN** 上游返回 chunked + SSE
- **THEN** 响应 `content-type: text/event-stream`，chunk 边收边转发，**不**整段 collect 到内存。

### Requirement: 6 步自检全部 PASS

按 `AGENTS.md §3.1` 跑：

1. `cargo fmt -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo build --all-targets --all-features`
4. `cargo test --all-features`
5. `grep -rn "unsafe" src/ --include="*.rs"` → 0 hits
6. `grep -rEn "(api[_-]?key|token|secret)\s*[:=]\s*\"[A-Za-z0-9_\-]{16,}\"" src/ --include="*.rs"` → 0 hits

**任何一步失败** → 修复后重跑，**禁止**用"在我环境里能跑"敷衍。

#### Scenario: 自检通过
- **WHEN** 阶段一完成、准备交付
- **THEN** 6 步全部 PASS，输出对用户可见。

### Requirement: 越阶段创建检查

阶段一交付前**必须**执行 `find src config tests -name "*.rs" -o -name "*.yaml"` 列出所有创建的文件，**逐一核对** spec §2 阶段标注，**禁止**出现 `[S2]` / `[S3]` 标注的文件。

#### Scenario: 越阶段清单为空
- **WHEN** 跑越阶段检查
- **THEN** `[S2]` 标注文件数 = 0；`[S3]` 标注文件数 = 0。

## MODIFIED Requirements

无（首版无既有约定）。

## REMOVED Requirements

无。
