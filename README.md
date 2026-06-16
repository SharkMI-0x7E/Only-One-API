# RapidGate

[English](./README.md) | [中文](./README_cn.md)

> A high-performance unified LLM API gateway written in Rust.
> 高性能统一 LLM API 网关，Rust 实现。

Inspired by [songquanpeng/one-api](https://github.com/songquanpeng/one-api) — a "one-api" rewrite in Rust.
灵感来自 [songquanpeng/one-api](https://github.com/songquanpeng/one-api) — 用 Rust 重写。

## AI-Assisted Development / AI 辅助开发

This project is built through **AI collaboration** — not pure human, not pure AI. Two tools share the work, with the maintainer keeping final say:
本项目由 **AI 协作**完成（非纯手写也非纯 AI 写），两个工具分工，维护者保留最终决策权：

| Tool | Role / 角色 |
|---|---|
| [**Trae SOLO**](https://www.trae.ai/) | Multi-agent orchestration: spec planning, task breakdown, review checkpoints, three-stage pipeline coordination |
| [**opencode**](https://github.com/opencode-ai/opencode) | Terminal AI coding agent: file editing, test writing, single-file commit hygiene, push automation |

The maintainer ([SharkMI](https://github.com/SharkMI-0x7E)) drives the architecture, approves every spec, and **runs every `cargo` command by hand** — the agents type. Commit cadence is `one feature = one commit` (see `AGENTS.md §1.1` and `§8.6`).
维护者负责架构设计、审批每个 spec、**亲自跑所有 cargo 命令** — AI 负责敲代码与写 commit。提交节奏为「一个功能 = 一个 commit」（见 `AGENTS.md §1.1` 与 `§8.6`）。

## Build & Test / 构建与测试

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

## Project Layout / 目录结构

See [`docs/rapidgate-spec.md` §2](./docs/rapidgate-spec.md) for the full tree.
完整目录见 [`docs/rapidgate-spec.md` §2](./docs/rapidgate-spec.md)。

```
src/
├── core/      # 业务核心：error / config / routing / proxy / auth / ratelimit / breaker / observability / util
└── service/   # 服务层：state / telemetry / error / middleware / config_loader / upstream_pool / handler / server
config/        # YAML 配置（default / development / production + routes/）
tests/         # 集成测试
```

## License / 许可证

[Apache-2.0](./Cargo.toml)
