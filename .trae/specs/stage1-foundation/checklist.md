# 阶段一交付检查清单

> 全部 `[ ]` 勾上 = 阶段一可交付。任一未勾 = 阻塞项，必须先解决再交付。

## 编译与自检

- [ ] `cargo fmt -- --check` 通过
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` 通过
- [ ] `cargo build --all-targets --all-features` 通过
- [ ] `cargo test --all-features` 通过
- [ ] `grep -rn "unsafe" src/ --include="*.rs"` 输出 0 行
- [ ] `grep -rEn "(api[_-]?key|token|secret)\s*[:=]\s*\"[A-Za-z0-9_\-]{16,}\"" src/ --include="*.rs"` 输出 0 行

## 越阶段检查

- [ ] `find src -name "*.rs"` 列出的所有文件**仅含** [S1] / [S1+] 标注，无 [S2] / [S3]
- [ ] `find config -name "*.yaml"` 列出的所有文件**仅含** [S1] 标注
- [ ] `find tests -name "*.rs"` 列出的所有文件**仅含** [S1] 标注
- [ ] 未创建 `benches/` / `plugins/` / `deploy/` / `scripts/` 目录
- [ ] `Cargo.toml` 依赖**严格**等于 spec §3.1 列表（无多无少）

## 5 个 handler 冒烟

- [ ] `POST /v1/chat/completions` 响应非 5xx（含 SSE 流式）
- [ ] `POST /v1/embeddings` 响应非 5xx
- [ ] `GET /v1/models` 响应非 5xx
- [ ] `GET /healthz` 响应非 5xx
- [ ] `GET /readyz` 响应非 5xx

## 配置契约

- [ ] `config/default.yaml` 可被 `serde_yaml` 解析
- [ ] `config/development.yaml` 可被加载并覆盖 default
- [ ] `config/production.yaml` 可被加载并覆盖 default
- [ ] `config/routes/v1.yaml` 可被加载
- [ ] `${RGD_XXX}` 占位符缺失时**不 panic**，保留旧配置
- [ ] spec §7 R-1~R-8 校验规则**全部**实现

## 错误响应格式

- [ ] `CoreError::RouteNotFound` → 404 + JSON `{ "error": { "code": "route_not_found", ... } }`
- [ ] `CoreError::Auth` → 401 + JSON `unauthorized`（**不**区分"key 不存在" vs "key 错误"）
- [ ] `CoreError::RateLimited` → 429 + JSON `rate_limited`
- [ ] `CoreError::BreakerOpen` → 503 + JSON `breaker_open`
- [ ] `CoreError::UpstreamUnreachable` → 502 + JSON `upstream_unreachable`
- [ ] `CoreError::UpstreamTimeout` → 504 + JSON `upstream_timeout`
- [ ] `CoreError::BadRequest` → 400 + JSON `bad_request`

## 安全约束

- [ ] API Key / token / HMAC 等敏感字符串**全部**用 `subtle::ConstantTimeEq` 比较（无 `String::eq` / `==`）
- [ ] `core/` 模块下无 `use tokio` / `use reqwest` / `use notify` 引用
- [ ] `base_url` 经 `gateway.upstream_allowlist` 白名单校验（基础版）
- [ ] `.env` 文件未提交（仅 `.env.example` 提交）
- [ ] 配置文件示例里所有 `api_key` / `base_url` 都是 `${RGD_XXX}` 形式

## 集成测试

- [ ] `tests/common/mod.rs::spawn_app()` 可启动服务
- [ ] `tests/routing.rs` 5 路由冒烟通过
- [ ] `tests/error.rs` 错误响应格式断言通过

## 提交规范

- [ ] 每个 commit 标题符合 Conventional Commits（type 小写、scope 精确、单数动词）
- [ ] 一个 commit 只做一件事（无 "and" / "also" / "顺便"）
- [ ] 未直接 `git push`（用户明确指示前不推送）
- [ ] 未主动创建 README.md / CHANGELOG.md / 其他 .md 文档
- [ ] 未在源码 / commit message 中堆 emoji

## 交付前清单

- [ ] 给用户输出**阶段一交付清单**（含 commit hash 列表 + 自检结果 + 越阶段检查 + 已知 TODO + 进入阶段二建议）
