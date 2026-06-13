# RapidGate

[English](./README.md) | [中文](./README_cn.md)

> 高性能统一 LLM API 网关，Rust 实现。

灵感来自 [songquanpeng/one-api](https://github.com/songquanpeng/one-api) — 用 Rust 重写。

## 项目状态

- [x] **Stage 1 — 基础落地**（commit `d9086df`）：23 个 commit，6/6 自检全过，44/44 测试通过
- [ ] **Stage 2 — 强化**：真实 LLM provider、完整 SSRF 防护、metrics、配置热重载
- [ ] **Stage 3 — 规模化**：多实例、分布式限流、可观测性栈

完整规划：[`docs/rapidgate-spec.md`](./docs/rapidgate-spec.md) · [阶段一 spec](.trae/specs/stage1-foundation/spec.md)

## AI 辅助开发

本项目采用 **AI 协作**模式完成（既非纯手写也非纯 AI 写），两个工具分工，维护者保留最终决策权：

| 工具 | 角色 |
|---|---|
| [**Trae SOLO**](https://www.trae.ai/) | 多智能体编排：spec 规划、任务拆解、review checkpoint、三阶段流水线协调 |
| [**opencode**](https://github.com/opencode-ai/opencode) | 终端 AI 编码代理：文件编辑、写测试、单文件 commit 卫生、push 自动化 |

维护者（[SharkMI](https://github.com/SharkMI-0x7E)）负责架构设计、审批每个 spec、**亲自跑所有 `cargo` 命令** — AI 负责敲代码与写 commit。提交节奏为「一个功能 = 一个 commit」（见 `AGENTS.md §1.1` 与 `§8.6`）。

## 构建与测试

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

## 目录结构

完整目录见 [`docs/rapidgate-spec.md` §2](./docs/rapidgate-spec.md)。

```
src/
├── core/      # 业务核心：error / config / routing / proxy / auth / ratelimit / breaker / observability / util
└── service/   # 服务层：state / telemetry / error / middleware / config_loader / upstream_pool / handler / server
config/        # YAML 配置（default / development / production + routes/）
tests/         # 集成测试
```

## 许可证

[Apache-2.0](./Cargo.toml)
