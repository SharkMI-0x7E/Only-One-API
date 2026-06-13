# 阶段一任务清单（基础落地）

> 对应 `spec.md` 全部 ADDED Requirements。
> 每个任务**必须**满足：列出具体文件清单 → 写代码 → 跑 `cargo check` → 跑 §6 步自检 → 标完成。
> **禁止**写 `TODO` / `FIXME` / `unimplemented!()` / `panic!()` 在提交代码里（占位用 `tracing::info!("not implemented")`）。

---

## Task 1: 锁定依赖 + 工具链配置

- [ ] 创建 `Cargo.toml`（[S1]），严格按 spec §3.1 列出依赖，`version = "2026.06.1"`，`license = "Apache-2.0"`
- [ ] 创建 `rustfmt.toml`（[S1]，max_width = 100，edition = 2021）
- [ ] 创建 `rust-toolchain.toml`（[S1]，固定 stable channel）
- [ ] 创建 `.gitignore`（[S1]，target/ Cargo.lock? 不忽略 / .env / .idea/）
- [ ] 创建 `.env.example`（[S1]），仅占位 `RGD_*` 变量名，**无真实值**
- [ ] `cargo build` 跑通（首次会自动下载依赖）

**commit 标题**：`chore(deps): update Cargo.toml per spec §3.1`

---

## Task 2: core/error.rs + core/mod.rs

- [ ] `src/core/mod.rs`（[S1]）：声明所有子模块
- [ ] `src/core/error.rs`（[S1]）：定义 `CoreError` 枚举，按 spec §4.1 列出 9 个变体，`thiserror` derive
- [ ] 实现 `From<serde_yaml::Error>` / `From<std::io::Error>` 等常用转换

**commit 标题**：`feat(core): add CoreError enum`

---

## Task 3: core/config/*（gateway / route / upstream）

- [ ] `src/core/config/mod.rs`（[S1]）：声明子模块 + 重导出
- [ ] `src/core/config/gateway.rs`（[S1]）：`GatewayConfig` 结构（listen / request_timeout_ms / max_body_bytes / shutdown_timeout_ms / upstream_allowlist / logging）
- [ ] `src/core/config/route.rs`（[S1]）：`RouteConfig` + `RouteMatch` 结构（method / path / host / headers / query / upstream / auth / rate_limit）
- [ ] `src/core/config/upstream.rs`（[S1]）：`UpstreamConfig` + `LoadBalancer` 枚举
- [ ] 全部 `#[derive(Deserialize)]` + `#[serde(deny_unknown_fields)]`

**commit 标题**：`feat(core): add config data models`

---

## Task 4: core/routing/*（matcher / table）

- [ ] `src/core/routing/mod.rs`（[S1]）：声明子模块
- [ ] `src/core/routing/matcher.rs`（[S1]）：`Matcher` 枚举（Exact / Prefix / Regex）+ `match_request(req, route) -> bool`
- [ ] `src/core/routing/table.rs`（[S1]）：`RouteTable` 不可变结构 + `Router` 持 `Arc<ArcSwap<RouteTable>>`
- [ ] 匹配顺序：精确 → 前缀 → 正则；冲突时取先注册者，加载时 warn

**commit 标题**：`feat(core): add routing engine`

---

## Task 5: core/proxy/*（mod / transformer / stream）

- [ ] `src/core/proxy/mod.rs`（[S1]）：`Forwarder` trait + 默认实现
- [ ] `src/core/proxy/transformer.rs`（[S1]）：请求/响应 Header 与路径转换
- [ ] `src/core/proxy/stream.rs`（[S1]）：`into_axum_body()` 把 `reqwest::Response` 流转 axum `Body`
- [ ] **关键约束**：禁止 `.bytes().await?`，必须透传 `Body::Stream`

**commit 标题**：`feat(core): add proxy forwarder with streaming`

---

## Task 6: core/auth/*（mod / apikey）

- [ ] `src/core/auth/mod.rs`（[S1]）：`Authenticator` trait
- [ ] `src/core/auth/apikey.rs`（[S1]）：`ApiKeyAuthenticator` 用 `subtle::ConstantTimeEq` 常量时间比较
- [ ] **关键约束**：禁止 `String::eq` / `==` 直接比较 API Key；错误响应**不**区分"key 不存在"和"key 错误"

**commit 标题**：`feat(core): add API key authenticator`

---

## Task 7: core/ratelimit/*（mod / token_bucket / sliding_window / local_store）

- [ ] `src/core/ratelimit/mod.rs`（[S1]）：`RateLimiter` trait（`check(key) -> Result<(), CoreError>`）
- [ ] `src/core/ratelimit/token_bucket.rs`（[S1]）：令牌桶算法
- [ ] `src/core/ratelimit/sliding_window.rs`（[S1]）：滑动窗口算法
- [ ] `src/core/ratelimit/local_store.rs`（[S1]）：Moka 封装的进程内存储
- [ ] **约束**：不绑死 axum；纯算法层

**commit 标题**：`feat(core): add rate limiter algorithms`

---

## Task 8: core/breaker/*（mod / state / breaker）

- [ ] `src/core/breaker/mod.rs`（[S1]）
- [ ] `src/core/breaker/state.rs`（[S1]）：`BreakerState` 枚举（Closed / Open / HalfOpen）
- [ ] `src/core/breaker/breaker.rs`（[S1]）：计数器 + 状态转换，`call(future) -> Result<T, CoreError>`

**commit 标题**：`feat(core): add circuit breaker`

---

## Task 9: core/observability/*（mod + trace 占位）

- [ ] `src/core/observability/mod.rs`（[S1]）
- [ ] `src/core/observability/trace.rs`（[S1]）：`TraceId::new()` 生成 32 字节十六进制；W3C tracecontext 实现**留** [S2]
- [ ] `src/core/observability/metrics.rs`（[S3] 标注）— **本阶段不创建**（核对 spec §2，确认 [S3] 标注）

**commit 标题**：`feat(core): add observability trace id`

---

## Task 10: core/util/*（mod / path / hash）

- [ ] `src/core/util/mod.rs`（[S1]）
- [ ] `src/core/util/path.rs`（[S1]）：`normalize("/a/./b/../c") -> "/a/c"`，处理 `..` / `.` / 多余 `/`
- [ ] `src/core/util/hash.rs`（[S1]）：普通哈希 + `consistent_hash(key, buckets) -> usize`（一致性哈希留 [S2]）

**commit 标题**：`feat(core): add path normalization and hash`

---

## Task 11: service/state.rs + service/telemetry.rs + service/error.rs

- [ ] `src/service/mod.rs`（[S1]）
- [ ] `src/service/state.rs`（[S1]）：`AppState` 结构（route_table / upstreams / limiters / audit_tx / config_dir），全部 `Arc` 共享
- [ ] `src/service/telemetry.rs`（[S1]）：`init()` 初始化 tracing-subscriber（env-filter + pretty/json）
- [ ] `src/service/error.rs`（[S1]）：`ServiceError` + `From<CoreError>` + `IntoResponse` 输出 spec §5.1 统一 JSON

**commit 标题**：`feat(service): add AppState and error type`

---

## Task 12: service/middleware/trace.rs

- [ ] `src/service/middleware/mod.rs`（[S1]）
- [ ] `src/service/middleware/trace.rs`（[S1]）：axum middleware，生成 / 提取 `X-Request-Id`，写入 tracing span
- [ ] 后续 `auth` / `ratelimit` / `audit` / `recovery` **禁止**在阶段一创建（[S2] / [S3]）

**commit 标题**：`feat(middleware): add request id propagation`

---

## Task 13: service/config_loader.rs（基础版）

- [ ] 读 `RGD_CONFIG_DIR`（默认 `./config`）
- [ ] 加载 `default.yaml` → `development.yaml` 或 `production.yaml`（按 `RGD_ENV`）→ `routes/*.yaml`
- [ ] `${VAR}` 占位符展开（缺失则 `CoreError::Config`）
- [ ] 校验 spec §7 R-1~R-8
- [ ] 校验失败 → **保留旧配置** + tracing::error，**不 panic**
- [ ] 校验通过 → 返回新 `GatewayConfig`
- [ ] 阶段一**不**实现 ArcSwap 热切换（[S2] 完整回滚）

**commit 标题**：`feat(config): add YAML loader with placeholder expansion`

---

## Task 14: service/upstream_pool.rs（白名单基础版）

- [ ] reqwest::Client 池（`moka::future::Cache<UpstreamId, reqwest::Client>`）
- [ ] SSRF 白名单基础版：检查 `base_url` host 是否在 `gateway.upstream_allowlist` 内
- [ ] 阶段一**不**实现 DNS 解析 + IP 段检查（[S2] 完整 SSRF 防护）
- [ ] 阶段一**不**实现完整 API Key 常量时间比较逻辑（已在 `core/auth` 落地）

**commit 标题**：`feat(upstream): add basic upstream pool and allowlist`

---

## Task 15: service/handler.rs（5 个路由）

- [ ] `chat_completions` (POST `/v1/chat/completions`) — OpenAI 兼容 + SSE 流式
- [ ] `embeddings` (POST `/v1/embeddings`)
- [ ] `list_models` (GET `/v1/models`)
- [ ] `healthz` (GET `/healthz`) — 不查上游
- [ ] `readyz` (GET `/readyz`) — 检查配置有效性
- [ ] 全部返回 `Result<axum::response::Response, ServiceError>`
- [ ] mock 上游可跑通（占位返回 200 + 提示 "not implemented" 的 JSON body）

**commit 标题**：`feat(handler): add 5 OpenAI-compatible handlers`

---

## Task 16: service/server.rs + src/main.rs

- [ ] `src/service/server.rs`（[S1+]）：`router(state: Arc<AppState>) -> axum::Router`，把 middleware 与 5 个 handler 装上
- [ ] `src/main.rs`（[S1+]）：`#[tokio::main]`，信号监听、`with_graceful_shutdown`，退出码（成功 0 / 配置 78 / 其他 1）
- [ ] 阶段一**不**实现完整 graceful shutdown 细节（[S1+] 增强）

**commit 标题**：`feat(server): add axum router and main entrypoint`

---

## Task 17: 配置文件

- [ ] `config/default.yaml`（[S1]）：按 spec §6.1 最小骨架
- [ ] `config/development.yaml`（[S1]）：开发覆盖（debug 日志级别）
- [ ] `config/production.yaml`（[S1]）：生产覆盖（info 日志 + JSON 格式）
- [ ] `config/routes/v1.yaml`（[S1]）：1 条示例路由（openai-chat），`base_url` / `api_key` 用 `${RGD_XXX}` 占位
- [ ] 阶段一**不**创建 `config/providers/` / `config/routes/v2.yaml` / `config/routes/experiments/`（[S3]）

**commit 标题**：`chore(config): add default and v1 route configuration`

---

## Task 18: 集成测试骨架

- [ ] `tests/common/mod.rs`（[S1]）：`spawn_app() -> TestApp` 工具（启动服务 + 提供 base URL）
- [ ] `tests/routing.rs`（[S1]）：路由匹配优先级 / 5 路由冒烟
- [ ] `tests/error.rs`（[S1]）：错误响应格式（JSON 结构 + 状态码）
- [ ] 阶段一**不**创建 `tests/proxy_stream.rs` / `auth.rs` / `ratelimit.rs` / `hot_reload.rs`（[S2]）
- [ ] 阶段一**不**创建 `tests/canary.rs` / `plugins.rs` / `providers/` / `e2e/`（[S3]）

**commit 标题**：`test(integration): add routing and error smoke tests`

---

## Task 19: 越阶段检查 + 6 步自检

- [ ] `find src config tests -name "*.rs" -o -name "*.yaml"` 列清单，**逐一核对** spec §2 阶段标注
- [ ] `[S2]` 文件数 = 0
- [ ] `[S3]` 文件数 = 0
- [ ] `cargo fmt -- --check` PASS
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` PASS
- [ ] `cargo build --all-targets --all-features` PASS
- [ ] `cargo test --all-features` PASS
- [ ] `grep -rn "unsafe" src/ --include="*.rs"` → 0 hits
- [ ] `grep -rEn "(api[_-]?key|token|secret)\s*[:=]\s*\"[A-Za-z0-9_\-]{16,}\"" src/ --include="*.rs"` → 0 hits
- [ ] 全部 PASS 后输出**阶段一交付清单**给用户

**commit 标题**：（本任务不出新 commit，阶段一所有 commit 完成后做最终自检）

---

## Task Dependencies

- Task 2（error）→ Task 3（config）→ Task 11（service/error.rs 依赖 core/error）
- Task 4（routing）→ Task 11（service/state 依赖 Router）
- Task 5（proxy）→ Task 14（upstream_pool 依赖 Forwarder）
- Task 6（auth）→ Task 15（handler 依赖 Authenticator）
- Task 7（ratelimit）→ Task 11（state 依赖 limiters）
- Task 8（breaker）→ Task 14（upstream_pool 依赖 Breaker）
- Task 9（observability）→ Task 12（middleware/trace 依赖 TraceId）
- Task 10（util）→ Task 4（matcher 路径匹配用 normalize）
- Task 11 → Task 13（config_loader 用 AppState）
- Task 13 → Task 15（handler 用 config 加载结果）
- Task 14 → Task 15（handler 用 upstream pool）
- Task 15 → Task 16（server 装 handler）
- Task 16 → Task 17（配置文件提供环境变量）
- 全部 → Task 18（测试）
- 全部 → Task 19（自检）

**并行机会**：Task 2~10（core/*）内部各文件互相独立；Task 11~14（service/*）大部分独立；Task 17（config）与代码 Task 2~16 可并行。
