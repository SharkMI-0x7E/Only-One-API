# RapidGate 架构文档

本文档描述 RapidGate 的系统架构、核心模块职责与关键设计决策。

---

## 1. 系统架构

### 1.1 整体架构图

```
                        +---------------------+
                        |     Client App      |
                        +----------+----------+
                                   |
                                   v
                        +---------------------+
                        |   Reverse Proxy /   |
                        |   Load Balancer     |
                        +----------+----------+
                                   |
                                   v
              +-----------------------------------------+
              |            RapidGate Gateway             |
              |                                         |
              |  +-----------------------------------+  |
              |  |         Middleware Stack           |  |
              |  |  trace -> auth -> ratelimit ->    |  |
              |  |  audit -> recovery                |  |
              |  +----------------+------------------+  |
              |                   |                     |
              |  +----------------v------------------+  |
              |  |         Routing Engine             |  |
              |  |  (exact / prefix / regex match)    |  |
              |  |  (canary weight / header / cookie) |  |
              |  +----------------+------------------+  |
              |                   |                     |
              |  +----------------v------------------+  |
              |  |        Provider Adapter            |  |
              |  |  OpenAI / Anthropic / Gemini /     |  |
              |  |  Local (Ollama / vLLM)             |  |
              |  +----------------+------------------+  |
              |                   |                     |
              |  +----------------v------------------+  |
              |  |       Streaming Forwarder          |  |
              |  |  SSE chunked passthrough           |  |
              |  |  WebSocket upgrade (Realtime API)  |  |
              |  +----------------+------------------+  |
              |                   |                     |
              |  +-----------------------------------+  |
              |  |        Plugin System (S3)          |  |
              |  |  native / WASM sandbox             |  |
              |  +-----------------------------------+  |
              |                                         |
              +----+------------------+-----------------+
                   |                  |
                   v                  v
         +----------------+  +----------------+
         |  LLM Provider  |  |  LLM Provider  |
         |  (OpenAI etc.) |  |  (Anthropic)   |
         +----------------+  +----------------+
```

### 1.2 请求生命周期

```
Client Request
    |
    v
[trace middleware]  -- 生成/提取 X-Request-Id
    |
    v
[auth middleware]   -- API Key / JWT / OAuth2 校验
    |
    v
[ratelimit middleware] -- 令牌桶/滑动窗口限流
    |
    v
[audit middleware]  -- 记录请求开始
    |
    v
[recovery middleware] -- panic 恢复
    |
    v
[plugin on_request] -- 插件前置钩子
    |
    v
[route matching]    -- 精确 -> 前缀 -> 正则
    |
    v
[canary decision]   -- 权重/Header/Cookie 灰度选择 upstream
    |
    v
[provider adapter]  -- 协议转换 (请求归一化)
    |
    v
[SSRF check]        -- DNS 解析 + IP 白名单校验
    |
    v
[streaming forward] -- SSE/WebSocket 透传上游响应
    |
    v
[plugin after_proxy] -- 插件后置钩子
    |
    v
[audit middleware]  -- 记录 token 用量 + 延迟
    |
    v
Client Response
```

### 1.3 模块分层

```
src/
+-- core/               纯业务逻辑层（无 I/O、无网络、无 tokio）
|   +-- config/         配置数据模型（serde 定义）
|   +-- routing/        路由匹配引擎 + 灰度策略
|   +-- proxy/          转发逻辑（SSE / WebSocket）
|   +-- auth/           认证抽象（API Key / JWT / OAuth2）
|   +-- ratelimit/      限流算法（令牌桶 / 滑动窗口）
|   +-- breaker/        熔断器状态机
|   +-- audit/          审计事件 + token 计数
|   +-- observability/  可观测性抽象（trace / metrics / otel）
|   +-- plugins/        插件 trait + 注册表
|   +-- canary/         灰度发布策略
|   +-- util/           工具函数
|
+-- service/            框架集成层（axum + tokio + 文件 I/O）
|   +-- middleware/     axum 中间件
|   +-- providers/      多 LLM Provider 适配
|   +-- config_center/  ETCD / Consul 配置中心
|   +-- admin/          管理 API（GraphQL）
|   +-- server.rs       HTTP 服务 + 优雅关闭
|   +-- handler.rs      路由处理器
|   +-- state.rs        共享状态（Arc<AppState>）
|   +-- config_loader.rs 配置加载
|   +-- hot_reload.rs   配置热重载 + 回滚
|   +-- error.rs        ServiceError + IntoResponse
|   +-- telemetry.rs    tracing 初始化
```

---

## 2. 核心模块说明

### 2.1 core 模块

`core` 模块封装所有纯业务逻辑，**不依赖** tokio、reqwest、notify 等 I/O crate。这一分层使得核心逻辑可以独立测试，不受运行时约束。

| 子模块 | 职责 | 关键类型 |
|--------|------|----------|
| `config` | 配置数据模型定义 | `GatewayConfig`, `RouteConfig`, `UpstreamConfig`, `ProviderConfig` |
| `routing` | 路由匹配引擎，支持精确/前缀/正则匹配 | `RouteTable`, `Router`, `Matcher` |
| `proxy` | 请求转发，SSE 流式透传 | `Forwarder` trait, `StreamForwarder` |
| `auth` | 认证抽象，常量时间比较 | `Authenticator` trait, `ApiKeyAuth`, `JwtAuth` |
| `ratelimit` | 限流算法实现 | `RateLimiter` trait, `TokenBucket`, `SlidingWindow` |
| `breaker` | 熔断器状态机 | `CircuitBreaker`, `BreakerState` |
| `audit` | 审计事件与 token 计数 | `AuditEvent`, `TokenCounter` |
| `observability` | 可观测性抽象接口 | `Metrics` trait, `TraceId` |
| `plugins` | 插件系统 trait 与注册表 | `Plugin` trait, `PluginRegistry` |
| `canary` | 灰度发布策略 | `CanaryPolicy`, `StickySession` |

### 2.2 service 模块

`service` 模块负责框架集成，包含所有 I/O 操作、HTTP 处理、配置加载与文件监听。

| 子模块 | 职责 | 关键类型 |
|--------|------|----------|
| `middleware` | axum 中间件链 | trace, auth, ratelimit, audit, recovery |
| `providers` | 多 LLM Provider 协议适配 | `OpenAIProvider`, `AnthropicProvider`, `GeminiProvider` |
| `config_center` | 外部配置中心集成 | `EtcdBackend`, `ConsulBackend` |
| `admin` | 管理 API（独立端口） | GraphQL schema, admin auth |
| `server` | HTTP 服务启动与优雅关闭 | `axum::serve` + graceful shutdown |
| `handler` | 路由处理器（5 个端点） | `chat_completions`, `embeddings`, `list_models`, `healthz`, `readyz` |
| `state` | 全局共享状态 | `AppState`（Arc 共享） |
| `config_loader` | YAML 配置加载 + 环境变量展开 | 加载 default -> env overlay -> routes |
| `hot_reload` | 配置热重载 + 校验失败回滚 | `ArcSwap<RouteTable>` |
| `error` | 统一错误响应 | `ServiceError` -> JSON `{ "error": { "code", "message" } }` |

### 2.3 配置体系

```
config/
+-- default.yaml          基础配置（监听地址、超时、默认限流）
+-- development.yaml      开发环境覆盖
+-- production.yaml       生产环境覆盖
+-- providers/            各 Provider 独立配置
|   +-- openai.yaml
|   +-- anthropic.yaml
|   +-- gemini.yaml
|   +-- local.yaml
+-- routes/               路由配置（可独立热重载）
|   +-- v1.yaml           OpenAI 兼容路由
|   +-- v2.yaml           v2 路由
|   +-- experiments/      灰度实验配置
|       +-- chat-v2-rollout.yaml
```

配置加载顺序：`default.yaml` -> `{env}.yaml` 覆盖 -> `routes/*.yaml` 合并。所有 `${VAR}` 占位符在加载时由环境变量展开，缺失则保留旧配置。

---

## 3. 设计决策记录

### ADR-001: 选择 axum 作为 HTTP 框架

**状态**: 已采纳

**背景**: 需要一个高性能、原生支持异步和 SSE 流式的 HTTP 框架。

**决策**: 选择 axum 0.8。

**理由**:
- 由 tokio 团队维护，与 tokio 运行时深度集成
- 原生支持 SSE、WebSocket、流式响应
- 类型安全的提取器（extractor）机制，减少运行时错误
- tower 中间件生态兼容
- 零成本抽象，性能接近 hyper 裸写

**替代方案**:
- actix-web: 性能优秀但生态相对封闭，tower 不兼容
- warp: 函数式风格，但错误处理和流式支持不如 axum 直观
- rocket: 同步优先，不适合高并发网关场景

---

### ADR-002: 选择 tokio 作为异步运行时

**状态**: 已采纳

**背景**: 需要高性能够用的异步运行时，支持信号处理、定时器、连接池。

**决策**: 选择 tokio（full features）。

**理由**:
- axum 和 hyper 的底层运行时，天然兼容
- 成熟的多线程调度器，经过大规模生产验证
- 完善的信号处理（SIGINT/SIGTERM）支持优雅关闭
- 丰富的原语（mpsc、watch、Mutex、RwLock）

**替代方案**:
- async-std: 社区活跃度下降，生态不如 tokio
- smol: 轻量但缺乏企业级支持

---

### ADR-003: 单 crate + 内部 mod 分层

**状态**: 已采纳

**背景**: 项目需要在 core/service 之间做清晰分层，同时避免早期 workspace 带来的复杂性。

**决策**: 首版使用单 crate + `core` / `service` 内部模块分层，阶段三评估是否拆分 `rapidgate-core` crate。

**理由**:
- 单 crate 编译速度快，依赖管理简单
- 内部 mod 分层已足够实现"core 无 I/O"的约束
- 避免 workspace 带来的版本同步、发布流程复杂性
- 阶段三若需插件独立编译，再拆 core crate

**替代方案**:
- 一开始就拆 workspace: 增加维护成本，首版不需要
- 完全扁平结构: 无法强制 core/service 分层约束

---

### ADR-004: 流式优先（Streaming First）

**状态**: 已采纳

**背景**: LLM 网关 90% 流量是 SSE 流式响应，必须避免将响应体一次性缓冲到内存。

**决策**: 所有转发路径直接透传上游 `reqwest::Body` 流到 axum `Body`，禁止 `.bytes().await?` 一次性缓冲。

**理由**:
- LLM 响应可能持续数十秒，缓冲会导致内存暴涨
- SSE 协议要求逐 chunk 推送
- reqwest 的 `Body` 实现了 `Stream`，可直接转为 axum 的 `Body`

**替代方案**:
- 先缓冲再转发: 简单但不可接受（内存 + 延迟）

---

### ADR-005: 配置热重载使用 ArcSwap

**状态**: 已采纳

**背景**: 路由配置需要支持运行时热重载，且 in-flight 请求必须用旧配置跑完。

**决策**: 使用 `arc_swap::ArcSwap<RouteTable>` 实现无锁热重载。

**理由**:
- `ArcSwap` 读操作无锁（原子指针交换），适合高频读场景
- 整体替换保证路由表不可变，切换时不影响已加载的请求
- 校验失败时不替换，天然支持回滚

**替代方案**:
- `RwLock<RouteTable>`: 读操作也需要获取锁，高并发下性能差
- `lazy_static` + `OnceLock`: 不支持运行时替换

---

### ADR-006: SSRF 防护采用 IP 白名单

**状态**: 已采纳

**背景**: LLM 网关天然是 SSRF 跳板（用户配置 base_url -> 内部网络），必须阻止对内网地址的请求。

**决策**: 在转发前解析 base_url 的 DNS，检查所有 A/AAAA 记录是否在 IP 黑名单（私有段/回环/链路本地）内。

**理由**:
- DNS rebinding 攻击可以通过校验后 IP + Host Header 防御
- 默认黑名单覆盖 RFC 1918 私有段、云元数据地址（169.254.0.0/16）
- 在网关层拦截，避免每个 provider 各自实现

**替代方案**:
- 仅校验域名白名单: 无法防止 DNS rebinding
- 不校验: 安全风险不可接受

---

### ADR-007: 错误响应统一 JSON 格式

**状态**: 已采纳

**背景**: 网关需要统一的错误响应格式，方便客户端解析和监控系统集成。

**决策**: 所有错误统一输出 `{ "error": { "code": "...", "message": "..." } }` 格式，HTTP 状态码与 code 字段对应。

**理由**:
- 与 OpenAI API 错误格式兼容
- `code` 字段用于程序化判断，`message` 用于人类阅读
- `ServiceError` 是唯一实现 `IntoResponse` 的类型，保证一致性

**替代方案**:
- 各 handler 自定义错误格式: 不一致，客户端难以统一处理

---

### ADR-008: 敏感字段环境变量注入

**状态**: 已采纳

**背景**: API Key、Token 等敏感信息不能硬编码在配置文件中。

**决策**: 配置文件使用 `${RGD_*}` 占位符，加载时由环境变量展开。缺失则配置加载失败，保留旧配置。

**理由**:
- 配置文件可安全提交到版本控制
- `.env` 文件已在 `.gitignore` 中
- 与 Docker/K8s 的 Secret 管理机制兼容

**替代方案**:
- 配置文件直接写明文: 泄露风险
- Vault 集成: 首版不需要，增加复杂度

---

### ADR-009: 插件系统采用 trait + native/WASM 双轨

**状态**: 已采纳

**背景**: 需要在不修改核心代码的前提下扩展网关行为。

**决策**: 定义 `Plugin` trait，支持 native（动态库）和 WASM（沙箱）两种加载方式。

**理由**:
- native 插件性能最优，适合内部团队开发
- WASM 沙箱隔离，适合第三方插件
- trait 抽象统一两种加载方式

**替代方案**:
- 仅支持 native: 安全风险
- 仅支持 WASM: 性能开销
- Lua 脚本: 生态不如 WASM

---

### ADR-010: 版本号采用 YYYY.MM.N 格式

**状态**: 已采纳

**背景**: 需要直观反映发布时间的版本号方案。

**决策**: 使用 `YYYY.MM.N` 格式（如 `2026.06.1`），禁止 `0.x.y` semver。

**理由**:
- 年份+月份直观反映发布周期
- N 为当月内发布序号，适合快速迭代
- CI 自动校验格式，避免混乱

**替代方案**:
- 标准 semver: 无法直观反映发布时间
- 纯日期: 无法区分同一天多次发布
