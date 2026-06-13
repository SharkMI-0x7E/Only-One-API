# RapidGate — 三阶段流水线提示词

> 本文件是**阶段一 + 阶段二**的作业指导。
> 配合 `AGENTS.md`（宪法）与 `docs/rapidgate-spec.md`（图纸）使用。
> **阶段三的详细任务**见 [docs/rapidgate-stage3.md](file:///e:/pythonxiangmuwenjianjia/RapidGate/docs/rapidgate-stage3.md)。
>
> **关键约定**：每阶段**只**创建 `rapidgate-spec.md §2` 中**本阶段标注**的目录与文件（`[S1]` / `[S2]` / `[S3]`）。
> 单次 AI 会话**只跑一个阶段**。所有通用约束见 `AGENTS.md`，本文件不重复。

---

## 0. 四份文档的关系

```
AGENTS.md                          docs/rapidgate-spec.md              docs/rapidgate-prompt.md              docs/rapidgate-stage3.md
   (宪法)                              (图纸)                              (S1 + S2 作业指导)                      (S3 详细任务)
   - 提交规范                          - 终极目录树 (带 [S1][S2][S3])        - 阶段一任务                            - 阶段三任务
   - PR 要求                          - 依赖三阶段分组                      - 阶段二任务                            - 阶段三强化维度
   - 构建 + 自检                      - 配置契约                            - 阶段切换礼仪                          - 阶段三交付清单
   - 安全策略                          - SSRF 防护                          - 不重复 AGENTS.md 通用规则              - 部署 / 插件 / 灰度
   - 版本号                            - 错误响应格式
   - 代码风格                          - 演进路线 (3 阶段)
   - AI 协作工作流
```

**优先级冲突时**：`AGENTS.md` > `rapidgate-spec.md` > `rapidgate-prompt.md` / `rapidgate-stage3.md`。

---

## 1. 三阶段总览

| 阶段 | 提示词文件 | 范围（按 spec §2 标注） | 产出 | 典型 commit 数 |
| --- | --- | --- | --- | --- |
| **阶段一：基础落地** | 本文件 §2 | 所有 `[S1]` / `[S1+]` | 可 `cargo build` 通过、6 步自检全过的代码骨架 | 8~15 |
| **阶段二：强化** | 本文件 §3 | 所有 `[S2]` / `[S2+]` | 性能 / 安全 / 测试 / 可观测 / 错误 / 跨切 6 维强化 | 10~25 |
| **阶段三：规模化** | `rapidgate-stage3.md` | 所有 `[S3]` / `[S3+]` | 多 provider / 插件 / 分布式 / 灰度 / admin / 部署 | 20~40 |

**三阶段硬约束**（来自 `AGENTS.md §8.5`）：

- 阶段 N+1 **禁止**在阶段 N 提前创建
- 阶段 N+1 **禁止**修改阶段 N 已 commit 的历史（cherry-pick 友好）
- 阶段 N+1 **禁止**添加 `spec.md §3` 本阶段未列出的新依赖
- 阶段 N+1 的所有 commit body **必须**标注 `Refine: stage-N <commit-sha>`
- 单次会话**只跑一个阶段**，跨阶段必须开新会话

---

## 2. 阶段一：基础落地

### 2.1 任务定义

把 `spec.md` 中**所有 `[S1]` / `[S1+]` 标注**的目录与文件变成**可编译、有签名、有最小逻辑**的代码骨架。

**显式禁止**（阶段一**不**创建）：

- ❌ 任何 `[S2]` 标注的文件（如 `core/auth/jwt.rs`、`service/middleware/auth.rs`、`benches/`）
- ❌ 任何 `[S3]` 标注的文件（如 `core/plugins/`、`service/providers/`、`plugins/`、`deploy/`、`scripts/`）
- ❌ 任何 `[S1]` 之外的"自由发挥"目录

### 2.2 完成度判据

阶段一**结束**的标志（**全部满足**才能进入阶段二）：

- [ ] `Cargo build --all-targets --all-features` 通过
- [ ] `Cargo test --all-features` 通过（即使只是空测试，0 个也算通过）
- [ ] `AGENTS.md §3.1` 6 步自检**全部** PASS
- [ ] `spec.md §2` 中**所有 `[S1]` 标注**的目录与文件都已创建（占位 `.rs` 允许 `tracing::info!("not implemented")`，**禁止** `todo!()` / `unimplemented!()`）
- [ ] `spec.md §5.5` 的 5 个路由（chat / embeddings / models / healthz / readyz）**至少**能返回非 5xx
- [ ] 配置文件 `config/default.yaml` + `config/routes/v1.yaml` 可加载（即使所有上游都用 mock 桩）
- [ ] **不**创建任何 `[S2]` / `[S3]` 标注的文件（用 `find` 命令验证）

### 2.3 阶段一工作流

```
1. 读 AGENTS.md §8.1 必读清单
2. 读 spec.md 全文（一次性读完，不要边写边翻）
3. 从 spec §2 中筛出**所有 [S1] / [S1+] 标注**作为本阶段范围
4. 列出"阶段一全部 commit 计划"（按模块拆），给用户确认
5. 按顺序执行：
   a. chore(deps): update Cargo.toml per spec §3.1   # 第 1 个 commit
   b. feat(core): 落地 [S1] 标注的 core/ 模块        # 1 个或多个 commit
   c. feat(service): 落地 [S1] 标注的 service/ 模块  # 1 个或多个 commit
   d. feat(handler): 实现 5 个路由的 handler
   e. feat(middleware): 实现 trace 中间件（[S1] 部分）
   f. feat(config): 实现 config_loader
   g. test: 集成测试骨架（tests/common + 1~2 个示例）
   h. chore: 添加 .env.example + config/default.yaml + config/routes/v1.yaml
6. 每完成一个 commit 跑 AGENTS.md §3.1 6 步自检
7. 全部 commit 完成后用 find 命令验证未越阶段创建文件
8. 给用户"阶段一交付清单"
```

### 2.4 阶段一允许的内容

- ✅ 创建 spec §2 中**所有 `[S1]` / `[S1+]` 标注**的目录与文件
- ✅ 实现 spec §4/§5 中各模块的**签名 + 最小逻辑**（如 `tracing::info!("not implemented")`）
- ✅ 业务实现可简化：限流只实现令牌桶，鉴权只支持 API Key（JWT 留 `[S2]`），SSRF 只做白名单
- ✅ 集成测试只放 1~2 个冒烟用例，覆盖 happy path

### 2.5 阶段一禁止的内容

- ❌ 创建 spec §2 中**任何 `[S2]` 标注**的文件
- ❌ 创建 spec §2 中**任何 `[S3]` 标注**的文件
- ❌ 添加 spec §3.1 之外的依赖
- ❌ 写完整测试覆盖（阶段二补）
- ❌ 写性能优化（阶段二补）
- ❌ 实现配置热重载回滚逻辑（**只**实现加载+切换，校验失败回滚放阶段二）
- ❌ 实现完整 SSRF 防护（**只**实现白名单校验，DNS 解析+IP 段检查放阶段二）
- ❌ 实现审计与计费（**只**留 `core/audit/mod.rs` 接口，不写存储；接口在 `[S2+]` 落地）
- ❌ 写 .md 文档（除用户明确要求）

### 2.6 阶段一交付清单格式

完成阶段一后，必须给用户以下内容：

```markdown
## 阶段一交付清单

### 完成的 commit（共 N 个）
1. <hash> chore(deps): update Cargo.toml per spec §3.1
2. <hash> feat(core): error + config models
...

### 目录树（仅 [S1] 范围）
（`find . -type d -not -path './target*' -not -path './.git*' | sort` 输出）

### 6 步自检结果
- cargo fmt --check: PASS
- cargo clippy -D warnings: PASS
- cargo build: PASS
- cargo test: PASS
- unsafe check: PASS (0 hits)
- secret check: PASS (0 hits)

### 越阶段创建检查
- [S2] 标注文件数: 0
- [S3] 标注文件数: 0

### 已知 TODO（留给阶段二）
- 列表项 1
- 列表项 2
...

### 进入阶段二的建议
- 优先强化项：...
- 风险提示：...
```

---

## 3. 阶段二：强化

### 3.1 任务定义

在阶段一骨架上做**强化**，对应 `spec.md §2` 中**所有 `[S2]` / `[S2+]` 标注**的目录与文件。

**显式禁止**（阶段二**不**创建）：

- ❌ 任何 `[S3]` 标注的文件
- ❌ 任何"超出 §3.2 强化维度清单"的功能（如多 provider 适配、插件系统）

### 3.2 完成度判据

- [ ] 阶段一所有交付**仍然通过**（fmt/clippy/test/6 步自检）
- [ ] `spec.md §2` 中**所有 `[S2]` 标注**的目录与文件都已创建
- [ ] 阶段二 §3.3 6 维度强化清单中**标注 ⭐ 的核心项全部完成**
- [ ] **不**创建任何 `[S3]` 标注的文件

### 3.3 强化维度清单（仅本阶段）

#### 维度 1：性能优化 ⭐ 必做

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| ⭐ 连接池调参 | `service/upstream_pool.rs` | `perf(upstream): tune connection pool` |
| ⭐ SSE chunk 合并 | `core/proxy/stream.rs` | `perf(stream): batch SSE chunks` |
| ⭐ 路由匹配零分配 | `core/routing/matcher.rs` | `perf(routing): zero-alloc match` |
| 一致性哈希 | `core/util/consistent_hash.rs`（[S2]） | `perf(util): add consistent hash` |
| 路由表预编译 | `core/routing/table.rs` | `perf(routing): precompile matchers` |
| 关键路径 `#[inline]` | `core/routing/matcher.rs` | `perf(routing): mark hot fns inline` |

#### 维度 2：安全加固 ⭐ 必做

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| ⭐ SSRF 完整实现 | `service/upstream_pool.rs` | `feat(security): full SSRF guard` |
| ⭐ 配置热重载回滚 | `service/hot_reload.rs`（[S2]） | `feat(config): rollback on validation failure` |
| ⭐ 速率限制实际生效 | `service/middleware/ratelimit.rs`（[S2]） | `feat(middleware): enforce rate limit` |
| ⭐ Bearer / API Key 实际校验 | `service/middleware/auth.rs`（[S2]） | `feat(auth): implement constant-time compare` |
| ⭐ 敏感 Header 脱敏 | `service/telemetry.rs` | `feat(observability): redact sensitive headers` |
| JWT 完整实现 | `core/auth/jwt.rs`（[S2]） | `feat(auth): add JWT support` |
| 请求体大小限制 | `service/server.rs` | `feat(security): enforce body size limit` |

#### 维度 3：测试覆盖 ⭐ 必做

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| ⭐ 单元测试 | `core/` 下每个模块 | `test(core): unit tests for <module>` |
| ⭐ 集成测试 | `tests/routing.rs` `tests/error.rs`（[S1]）+ `tests/proxy_stream.rs` `tests/auth.rs` `tests/ratelimit.rs` `tests/hot_reload.rs`（[S2]） | `test(integration): <scenario>` |
| ⭐ 流式响应测试 | `tests/proxy_stream.rs` | `test(proxy): SSE integrity` |
| 路由冲突测试 | `tests/routing.rs` | `test(routing): conflict resolution` |
| 配置回滚测试 | `tests/hot_reload.rs` | `test(config): rollback on bad config` |
| 错误响应格式测试 | `tests/error.rs` | `test(error): response format` |
| 基准测试 | `benches/routing_bench.rs` `benches/proxy_bench.rs`（[S2] 首次创建） | `test(bench): <scenario>` |

#### 维度 4：可观测性完善 ⭐ 必做

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| ⭐ W3C tracecontext | `core/observability/trace.rs`（[S2]） | `feat(trace): propagate W3C tracecontext` |
| ⭐ 审计日志结构化 | `core/audit/mod.rs`（[S2]） | `feat(audit): structured audit log` |
| ⭐ token 用量统计 | `core/audit/counter.rs`（[S2]） | `feat(audit): count tokens from SSE` |
| 慢请求日志 | `service/telemetry.rs` | `feat(observability): slow request log` |

#### 维度 5：错误信息可读性

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| 错误码规范化 | `service/error.rs` | `feat(error): stable error codes` |
| 错误诊断字段 | `service/error.rs` | `feat(error): include request id` |
| 用户友好消息 | `service/error.rs` | `feat(error): user-friendly messages` |

#### 维度 6：跨切关注点

| 强化项 | 涉及文件 | commit 建议 |
| --- | --- | --- |
| graceful shutdown | `service/server.rs`（增强 [S1+]） | `feat(server): graceful shutdown` |
| 健康检查深化 | `service/handler.rs` | `feat(server): deep readyz` |
| 启动横幅 | `service/server.rs` | `feat(server): startup banner` |

### 3.4 阶段二工作流

```
1. 读阶段一交付清单 + 当前代码（git log + 关键文件）
2. 从 §3.3 6 个维度中挑本次会话要做的 2~3 个维度（不要全做）
3. 输出"强化 commit 计划"给用户确认
4. 按维度执行，每个 commit 跑 AGENTS.md §3.1 6 步自检
5. 维度内做完后跑 cargo bench（如果创建了 bench），对比阶段一基线
6. 用 find 验证未创建 [S3] 标注的文件
7. 全部 commit 完成后给用户"阶段二交付清单"
```

### 3.5 阶段二允许的内容

- ✅ 创建 `spec.md §2` 中**所有 `[S2]` / `[S2+]` 标注**的目录与文件
- ✅ 引入 `spec.md §3.2` 列出的阶段二依赖
- ✅ 增强（≠ 删除/大改） `[S1+]` 标注的现有文件
- ✅ 修改 `spec.md` 的**非约束性**部分（如阶段二强化项的细节）
- ❌ **禁止**修改阶段一已 commit 的历史
- ❌ **禁止**创建任何 `[S3]` 标注的文件

### 3.6 阶段二禁止的内容

- ❌ 修改 `Cargo.toml` 的 `version` 字段（发版另开任务）
- ❌ 在 `docs/` 下创建**新**的 .md 文件（用户没要求）
- ❌ 修改 CI workflow（CI 改另开任务）
- ❌ 引入新的 LLM provider 实现（Anthropic / Gemini 是阶段三的事）
- ❌ 实现集群 / 分布式功能（阶段三的事）
- ❌ 创建插件系统（阶段三的事）

### 3.7 阶段二交付清单格式

```markdown
## 阶段二交付清单（基于阶段一 <tag-or-sha>）

### 强化维度
- 维度 1 性能优化：完成 5/6 项
- 维度 2 安全加固：完成 6/7 项
- 维度 3 测试覆盖：完成 5/7 项
- 维度 4 可观测性：完成 3/4 项
- 维度 5 错误信息：完成 2/3 项
- 维度 6 跨切：完成 2/3 项

### 完成的 commit（共 N 个）
1. <hash> chore(deps): add stage-2 dependencies
2. <hash> perf(upstream): tune connection pool
...

### 6 步自检结果
（同阶段一格式）

### 越阶段创建检查
- [S3] 标注文件数: 0

### 性能基线对比（如有 bench）
| 指标 | 阶段一 | 阶段二 | 变化 |
| --- | --- | --- | --- |
| 路由匹配 p99 | 1.2µs | 0.4µs | -66% |

### 建议进入阶段三
- 接入 Anthropic / Gemini provider
- 插件系统 / 灰度发布 / 分布式限流
- ...
```

---

## 4. 阶段切换的礼仪

**阶段 N → 阶段 N+1**：

1. 阶段 N 交付清单经用户确认
2. 用户明确说"进入阶段 N+1"
3. 开**新会话**，新会话开头明确说："我进入了阶段 N+1，基于 <sha>"
4. 重读 `AGENTS.md` + `spec.md` + 阶段 N commit log（不要靠记忆）
5. 按本文件 §2/§3（或 `stage3.md`）的对应阶段任务开始

**从阶段 N+1 回到阶段 N**（发现阶段 N 有 bug）：

1. **不要**直接改阶段 N 已 commit 的文件
2. **不要** revert 或 amend 阶段 N commit
3. 在阶段 N+1 **新 commit** 中修复，commit body 标注 `Fix: stage-N <sha>`
4. 下次阶段 N 重构时一并清理

**永远不要**做的事：

- ❌ 同一会话内既跑阶段 N 又跑阶段 N+1（上下文污染）
- ❌ 阶段 N+1 修改阶段 N 已 commit 的历史
- ❌ 阶段 N+1 跳过 spec 走变更流程就加依赖
- ❌ 阶段 N 创建 spec §2 中标注为 `N+1` / `N+2` 的文件

---

## 5. 一句话总结

> **阶段一照 spec [S1] 标注定骨架，阶段二在 [S2] 标注上做强化，阶段三在 [S3] 标注上规模化。**
> **一次一个阶段，跨阶段开新会话，commit body 标注溯源。**
> **AGENTS.md 是宪法，spec.md 是图纸，本文件管 S1+S2，stage3.md 管 S3。**
