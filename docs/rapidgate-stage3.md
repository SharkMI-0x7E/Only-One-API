# RapidGate — 阶段三详细任务（规模化）

> 本文件是**三阶段流水线**的**阶段三**任务定义。
> 配合 `AGENTS.md`（宪法）、`docs/rapidgate-spec.md`（图纸）、`docs/rapidgate-prompt.md`（S1+S2 作业指导）使用。
>
> **阶段三的目标**：在阶段一、二基础上把 RapidGate 从"能跑的单机网关"升级为"可生产、可集群、可扩展的多 LLM 统一网关"。

---

## 0. 阶段三的边界

**范围**（按 `spec.md §2` 标注）：

- 所有 `[S3]` 标注的目录与文件
- 对 `[S1+]` / `[S2+]` 标注的现有模块做**增强**（不是删改结构）

**显式不**属于阶段三（避免范围爆炸）：

- ❌ 修改阶段一/二已 commit 的核心数据结构（trait 签名、错误码、配置 schema）
- ❌ 重写 `core/proxy/stream.rs` 的 SSE 透传核心（已稳定）
- ❌ 引入新的运行时（保持 tokio）
- ❌ 拆 workspace（单 crate 始终保持）
- ❌ 实现服务端 LLM 推理（始终是网关/代理，不自研模型）

---

## 1. 完成度判据

阶段三**结束**的标志（**全部满足**才算完成）：

- [ ] 阶段一、二所有交付**仍然通过**（fmt/clippy/test/6 步自检）
- [ ] `spec.md §2` 中**所有 `[S3]` 标注**的目录与文件都已创建
- [ ] 阶段三 §3 8 维度强化清单中**标注 ⭐ 的核心项全部完成**
- [ ] 至少 3 个 provider（OpenAI + 2 个）完成适配并通过 e2e 测试
- [ ] 至少 1 个示例插件（native 或 WASM）跑通
- [ ] 灰度发布：权重 / Header / Cookie 三种策略**任一**可用
- [ ] 至少部署 Docker / K8s **任一**的 manifest 通过 `kubeconform` 校验
- [ ] 至少一份用户文档（`README.md` 或 `OPERATIONS.md`）写完

---

## 2. 阶段三工作流

```
1. 读阶段一 + 阶段二交付清单 + 当前代码（git log + 关键文件）
2. 从 §3 8 个维度中挑本次会话要做的 2~3 个维度
3. 输出"规模化 commit 计划"给用户确认
4. 按维度执行，每个 commit 跑 AGENTS.md §3.1 6 步自检
5. 维度内做完后跑 e2e + 负载测试（如有），对比阶段二基线
6. 全部 commit 完成后给用户"阶段三交付清单"
```

**关键差别**（与 S2 对比）：

- 阶段三引入的**新依赖**必须在 commit 标题中说明用途：`chore(deps): add redis for distributed rate limit`
- 阶段三的所有 commit body **必须**含 `Refine: stage-2 <commit-sha>`（沿用流水线溯源）
- 阶段三可能涉及**配置文件 schema 变更**（如 `config/providers/*.yaml`），schema 变更要单独一个 commit 标注 `feat(config): add providers schema`

---

## 3. 8 维度强化清单

### 维度 1：多 Provider 适配 ⭐ 必做

**目标**：阶段二只支持 OpenAI 兼容，阶段三扩展到 Anthropic / Gemini / 本地推理。

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| ⭐ Provider trait 抽象 | `core/config/provider.rs`（[S3]） | `feat(provider): add ProviderKind enum` |
| ⭐ OpenAI provider 完整 | `service/providers/openai.rs`（[S3]） | `feat(provider): implement OpenAI provider` |
| ⭐ Anthropic provider | `service/providers/anthropic.rs`（[S3]） | `feat(provider): add Anthropic provider` |
| ⭐ Gemini provider | `service/providers/gemini.rs`（[S3]） | `feat(provider): add Gemini provider` |
| 本地 provider（Ollama / vLLM） | `service/providers/local.rs`（[S3]） | `feat(provider): add local LLM provider` |
| Provider 独立配置文件 | `config/providers/*.yaml`（[S3]） | `feat(config): add per-provider config files` |
| Provider 路由分发 | `core/routing/canary.rs`（[S3]） | `feat(routing): route by provider kind` |
| 跨 provider token 计数统一 | `core/audit/counter.rs`（[S2+] 增强） | `feat(audit): unify token counting across providers` |

**关键约定**：

- 所有 provider **必须**实现统一的 `Provider` trait（定义在 `core/config/provider.rs`）
- `Provider::transform_request(req) -> ProviderRequest` 负责协议差异
- `Provider::transform_response(resp) -> StandardResponse` 负责响应归一化
- OpenAI 兼容协议是**默认实现**，其他 provider 通过 trait impl 提供

### 维度 2：插件系统 ⭐ 必做

**目标**：在不修改核心代码的前提下，通过插件扩展网关行为。

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| ⭐ Plugin trait 定义 | `core/plugins/trait.rs`（[S3]） | `feat(plugin): define Plugin trait` |
| ⭐ 插件注册表 | `core/plugins/registry.rs`（[S3]） | `feat(plugin): add plugin registry` |
| ⭐ Native 插件加载 | `core/plugins/native.rs`（[S3]） | `feat(plugin): add native plugin loader` |
| ⭐ 至少 2 个内置插件 | `plugins/add-request-id/` `plugins/cache-response/`（[S3]） | `feat(plugin): add <plugin-name> plugin` |
| WASM 沙箱（可选） | `core/plugins/wasm.rs`（[S3]） | `feat(plugin): add WASM sandbox` |
| 插件配置 schema | `config/default.yaml` 新增 `plugins:` 段 | `feat(config): add plugins schema` |
| 插件热加载 | `service/hot_reload.rs`（[S2+] 增强） | `feat(plugin): hot-reload plugin registry` |
| 插件权限模型 | `core/plugins/trait.rs` | `feat(plugin): add permission model` |

**Plugin trait 草案**：

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;

    async fn on_request(&self, ctx: &mut RequestContext) -> Result<(), PluginError> { Ok(()) }
    async fn before_proxy(&self, ctx: &mut ProxyContext) -> Result<(), PluginError> { Ok(()) }
    async fn after_proxy(&self, ctx: &mut ProxyContext) -> Result<(), PluginError> { Ok(()) }
    async fn on_error(&self, ctx: &mut ErrorContext) -> Result<(), PluginError> { Ok(()) }
}
```

### 维度 3：分布式与集群 ⭐ 必做

**目标**：从单机进程内限流升级到 Redis 集群限流；支持从 ETCD / Consul 中心拉取配置。

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| ⭐ Redis 分布式限流 | `core/ratelimit/redis_store.rs`（[S3]） | `feat(ratelimit): add Redis distributed store` |
| ⭐ ETCD 配置中心 | `service/config_center/etcd.rs`（[S3]） | `feat(config-center): add ETCD backend` |
| Consul 配置中心（可选） | `service/config_center/consul.rs`（[S3]） | `feat(config-center): add Consul backend` |
| 限流算法在 Redis 上跑 | `core/ratelimit/token_bucket.rs`（[S1+] 增强） | `feat(ratelimit): port token bucket to Redis` |
| 健康检查集群模式 | `service/handler.rs`（[S1+] 增强） | `feat(health): cluster-aware health check` |
| 上游节点发现 | `core/config/upstream.rs`（[S1+] 增强） | `feat(upstream): add node discovery` |

**关键约定**：

- Redis 限流用 Lua 脚本保证原子性（**禁止** GET/SET 两步走）
- 配置中心拉取的配置**仍走 spec §7 的校验**，校验失败保留旧配置
- **不**做跨节点的 session 共享（保持网关无状态）

### 维度 4：灰度发布 ⭐ 必做

**目标**：支持按权重 / Header / Cookie 将流量分发到不同 upstream。

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| ⭐ 权重灰度 | `core/canary/policy.rs`（[S3]） | `feat(canary): add weight-based policy` |
| ⭐ Header 灰度 | `core/canary/policy.rs` | `feat(canary): add header-based policy` |
| ⭐ Cookie 黏性 | `core/canary/sticky.rs`（[S3]） | `feat(canary): add cookie sticky session` |
| 灰度规则配置 schema | `config/routes/experiments/*.yaml`（[S3]） | `feat(config): add canary schema` |
| 灰度指标 | `core/observability/metrics.rs`（[S3]） | `feat(metrics): add canary stats` |
| 灰度测试 | `tests/canary.rs`（[S3]） | `test(canary): <scenario>` |

**关键约定**：

- 灰度决策**只影响 upstream 选择**，不影响鉴权 / 限流 / 审计
- 灰度权重修改**不**要求 reload（YAML watch 自动捕获）
- 灰度规则冲突（同一请求匹配多条）→ 按注册顺序取第一条

### 维度 5：admin API 与 Web UI

**目标**：提供运行时管理接口（查看路由、限流状态、上游健康、热重载配置）。

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| admin API 路由 | `service/admin/routes.rs`（[S3]） | `feat(admin): add admin API routes` |
| admin 鉴权 | `service/admin/auth.rs`（[S3]） | `feat(admin): add admin auth` |
| GraphQL schema（可选） | `service/admin/routes.rs` | `feat(admin): add GraphQL schema` |
| 路由查看端点 | `service/admin/routes.rs` | `feat(admin): GET /admin/routes` |
| 上游健康端点 | `service/admin/routes.rs` | `feat(admin): GET /admin/upstreams` |
| 限流状态端点 | `service/admin/routes.rs` | `feat(admin): GET /admin/limits` |
| 配置 dump 端点（脱敏） | `service/admin/routes.rs` | `feat(observability): GET /admin/config` |
| admin 审计 | `core/audit/mod.rs`（[S2+] 增强） | `feat(audit): audit admin operations` |

**关键约定**：

- admin API **必须**单独监听端口（默认 `127.0.0.1:9090`），**禁止**暴露在公网
- admin 鉴权用独立 token（`RGD_ADMIN_TOKEN`），与用户 API Key 分开
- admin 操作的每条记录写审计日志

### 维度 6：WebSocket 与 OAuth2

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| WebSocket 转发 | `core/proxy/ws.rs`（[S3]） | `feat(proxy): add WebSocket forwarder` |
| WS 鉴权 | `core/auth/mod.rs`（[S1+] 增强） | `feat(auth): WebSocket auth on upgrade` |
| OAuth2 流程 | `core/auth/oauth2.rs`（[S3]） | `feat(auth): add OAuth2 flow` |
| OAuth2 provider 配置 | `config/default.yaml` | `feat(config): add OAuth2 schema` |
| WS 测试 | `tests/providers/openai_compat.rs`（[S3]） | `test(ws): WebSocket forwarding` |

**关键约定**：

- WebSocket 转发**只**支持 OpenAI Realtime API 风格的协议（其他协议留作插件）
- OAuth2 **仅**实现 authorization_code + client_credentials 两种 grant
- OAuth2 token 缓存用 `moka`（与限流共享 cache 实例）

### 维度 7：可观测性（Prometheus + OpenTelemetry）

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| Prometheus exporter | `core/observability/metrics.rs`（[S3]） | `feat(metrics): add Prometheus exporter` |
| OpenTelemetry tracing | `core/observability/otel.rs`（[S3]） | `feat(otel): add OTLP tracing exporter` |
| OpenTelemetry metrics | `core/observability/otel.rs` | `feat(otel): add OTLP metrics exporter` |
| Prometheus 抓取端点 | `service/handler.rs`（[S1+] 增强） | `feat(metrics): GET /metrics endpoint` |
| Grafana dashboard JSON | `deploy/prometheus/rules.yaml`（[S3]） | `feat(ops): add Grafana dashboard` |
| 告警规则 | `deploy/prometheus/alerts.yaml`（[S3]） | `feat(ops): add Prometheus alerts` |

**关键约定**：

- Prometheus 端点**禁止**放在公网（同 admin 端口）
- 5xx 率 > 1% 持续 5 分钟触发告警（写进 `alerts.yaml`）
- P99 延迟 > 1s 持续 5 分钟触发告警

### 维度 8：部署、运维与文档

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| Dockerfile | `deploy/docker/Dockerfile`（[S3]） | `ci(docker): add multi-stage Dockerfile` |
| docker-compose | `deploy/docker/docker-compose.yaml`（[S3]） | `ci(docker): add docker-compose for dev` |
| K8s manifests | `deploy/k8s/*.yaml`（[S3]） | `ci(k8s): add deployment manifests` |
| systemd unit | `deploy/systemd/rapidgate.service`（[S3]） | `ci(systemd): add systemd service file` |
| ARCHITECTURE.md | `docs/ARCHITECTURE.md`（[S3]） | `docs: write architecture overview` |
| OPERATIONS.md | `docs/OPERATIONS.md`（[S3]） | `docs: write operations runbook` |
| README.md | `README.md`（[S3]） | `docs: write project README` |
| 性能压测脚本 | `scripts/load-test.sh`（[S3]） | `test(perf): add goose load test` |
| 模糊测试脚本 | `scripts/fuzz.sh`（[S3]） | `test(fuzz): add cargo-fuzz script` |
| 端到端测试 | `tests/e2e/openai_compat.rs` `tests/e2e/failover.rs`（[S3]） | `test(e2e): <scenario>` |
| CLI 工具 | `src/bin/cli.rs`（[S3]） | `feat(cli): add rapidgate-cli` |
| 暴露 lib 入口 | `src/lib.rs`（[S3]） | `feat(lib): expose rapidgate as library` |

**关键约定**：

- Dockerfile **必须**多阶段构建，最终镜像基于 `gcr.io/distroless/cc-debian12` 或 `alpine`
- K8s manifests **必须**通过 `kubeconform` 校验
- README.md **必须**含：项目简介 / 快速开始 / 配置示例 / 部署指南 / 排障链接

---

## 4. 阶段三工作约束

### 4.1 允许的内容

- ✅ 创建 `spec.md §2` 中**所有 `[S3]` / `[S3+]` 标注**的目录与文件
- ✅ 引入 `spec.md §3.3` 列出的阶段三依赖
- ✅ 增强（≠ 删除/大改） `[S1+]` / `[S2+]` 标注的现有文件
- ✅ 修改 `spec.md` 的**非约束性**部分（如阶段三强化项的细节）
- ✅ 修改 `config/default.yaml` 增量添加新 schema（**不**删旧字段）

### 4.2 禁止的内容

- ❌ 修改阶段一/二已 commit 的核心数据结构
- ❌ 修改 `Cargo.toml` 的 `version` 字段（发版另开任务）
- ❌ 创建 `docs/` 下**新**的 .md 文件，**除** spec §2 中 `[S3]` 标注的（`ARCHITECTURE.md` / `OPERATIONS.md` / `stage3.md` / `README.md`）
- ❌ 修改 CI workflow（CI 改另开任务）
- ❌ 拆 workspace（保持单 crate）
- ❌ 引入新运行时（保持 tokio）

### 4.3 新增依赖的额外审查

阶段三引入的依赖较多（11 个），每个新依赖**必须**在 commit body 注明：

```text
chore(deps): add <crate> = "<ver>"

- 用途: <具体作用>
- 维护活跃度: <最近 6 月 commit 数 / 最新版本日期>
- 依赖树影响: <直接依赖数 / 编译时增长>
- 许可证: <SPDX 标识>
- 替代方案: <如果存在，给出未选原因>

Refine: stage-2 <commit-sha>
```

---

## 5. 阶段三交付清单格式

```markdown
## 阶段三交付清单（基于阶段一 <sha1> + 阶段二 <sha2>）

### 规模化维度
- 维度 1 多 provider: 完成 5/8 项（OpenAI / Anthropic / Gemini / Local 全部完成）
- 维度 2 插件系统: 完成 5/8 项（native 加载 + 2 个内置插件，WASM 留作未来）
- 维度 3 分布式: 完成 3/6 项（Redis + ETCD 完成，Consul 跳过）
- 维度 4 灰度发布: 完成 4/6 项（权重 + Header + Cookie 完成）
- 维度 5 admin API: 完成 6/8 项（GraphQL 跳过）
- 维度 6 WebSocket + OAuth2: 完成 3/6 项
- 维度 7 可观测性: 完成 4/6 项
- 维度 8 部署运维: 完成 8/13 项（Docker + K8s + 文档完成，systemd + 模糊测试留作 v2）

### 完成的 commit（共 N 个）
1. <hash> chore(deps): add stage-3 dependencies
2. <hash> feat(provider): add Anthropic provider
...

### 6 步自检结果
- cargo fmt --check: PASS
- cargo clippy -D warnings: PASS
- cargo build: PASS
- cargo test: PASS
- cargo bench: 路由匹配 p99 = 0.3µs
- unsafe check: PASS (0 hits)
- secret check: PASS (0 hits)

### 多 provider 验证
- OpenAI 兼容: PASS（与 stage-1 baseline 一致）
- Anthropic: PASS（messages API 完整转发）
- Gemini: PASS（generateContent 转换）
- 本地 Ollama: PASS

### 部署验证
- Docker 镜像构建: PASS（`rapidgate:2026.06.x`）
- docker-compose up: PASS（standalone 模式跑通）
- K8s manifests: PASS（`kubeconform` 校验通过）
- 负载测试（goose 1000 RPS）: PASS（P99 < 50ms）

### 已知限制（留给 v2）
- WASM 插件沙箱（仅实现 trait，未实装加载）
- 跨节点 session 共享（保持无状态）
- ...

### 建议进入 v2 阶段
- Kubernetes Gateway API 集成
- 跨区域 active-active 部署
- 模型路由（按 prompt 内容选 model）
```

---

## 6. 一句话总结

> **阶段三把阶段一/二的玩具网关升级为生产级统一 LLM 入口。**
> **多 provider / 插件 / 分布式 / 灰度 / admin / 部署 = 8 个维度的规模化。**
> **每维度必做 ⭐ 项 + 可选项，按团队节奏分批交付。**
