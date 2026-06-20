# RapidGate

[English](./README.md) | [中文](./README_cn.md)

> 高性能统一 LLM API 网关，Rust 实现。

灵感来自 [songquanpeng/one-api](https://github.com/songquanpeng/one-api) — 用 Rust 重写。

## 关于

RapidGate 是一个统一 LLM API 网关，目标是让你用一套接口对接 OpenAI、Anthropic、Gemini、本地模型等多种 LLM 服务，同时获得限流、灰度、熔断、插件扩展等生产级能力。

项目目前处于**早期阶段**。核心骨架已搭建完成，多 Provider 适配、插件系统、灰度发布、分布式限流等模块已落地，但部分功能（如 Provider 转发层的真实串联、WASM 插件沙箱的完整实现）仍在推进中。这意味着 API 接口、配置格式、内部架构都可能在后续迭代中发生变化 — 如果你打算在生产环境使用，建议锁定版本并关注变更日志。

## 团队

本项目由 **1 名人类开发者 + 多个 AI 智能体** 协作完成。人类负责架构决策、审批规范、跑所有 `cargo` 命令；AI 智能体负责代码编写、测试覆盖和 commit 卫生。开发过程使用 [Trae SOLO](https://www.trae.ai/) 和 [opencode](https://github.com/opencode-ai/opencode) 协同。

这种"一人 + 多 AI"的模式让我们能以极小的团队快速迭代，但也意味着项目节奏和传统开源项目不同 — 你会看到大量细粒度的 commit（一个功能一个 commit），以及严格的 spec 驱动开发流程。

## 特性

- **多 Provider 支持**: OpenAI / Anthropic / Gemini / 本地模型统一接入
- **流式优先**: SSE 与 WebSocket 原生支持，零缓冲转发
- **动态路由**: 基于路径、方法、Header 的灵活路由规则，支持热重载
- **灰度发布**: 按权重、Header、Cookie 分流，支持会话黏性
- **安全防护**: SSRF 防护、API Key 常量时间校验、IP 白名单
- **可观测性**: Prometheus 指标导出、OpenTelemetry 追踪、结构化日志
- **插件系统**: 支持 Native 与 WASM 插件扩展
- **高可用**: 熔断器、限流、优雅关闭
- **分布式**: Redis 限流、ETCD/Consul 配置中心

## 快速开始

### 1. 克隆项目

```bash
git clone https://github.com/SharkMI-0x7E/RapidGate.git
cd RapidGate
```

### 2. 配置环境变量

```bash
cp .env.example .env
```

编辑 `.env` 文件，填入上游 Provider 的 API Key：

```bash
RGD_OPENAI_API_KEY=sk-your-key-here
RGD_ADMIN_TOKEN=your-admin-token
```

### 3. 启动服务

```bash
cargo run
```

服务默认监听 `0.0.0.0:8080`，可通过 `RGD_LISTEN` 环境变量修改。

### 验证

```bash
# 健康检查
curl http://localhost:8080/healthz

# 列出可用模型
curl http://localhost:8080/v1/models

# 聊天补全（流式）
curl -N http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'
```

## 构建与测试

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

## 部署

### Docker

```bash
docker build -f deploy/docker/Dockerfile -t rapidgate:latest .
docker run -p 8080:8080 -v $(pwd)/config:/app/config:ro rapidgate:latest
```

### Kubernetes

```bash
kubectl apply -f deploy/k8s/
```

### systemd

```bash
sudo cp deploy/systemd/rapidgate.service /etc/systemd/system/
sudo systemctl enable --now rapidgate
```

详细部署指南请参考 [运维手册](docs/OPERATIONS.md)。

## 目录结构

完整目录见 [`docs/rapidgate-spec.md` §2](./docs/rapidgate-spec.md)。

```
src/
├── core/      # 业务核心：error / config / routing / proxy / auth / ratelimit / breaker / canary / plugins / observability / util
└── service/   # 服务层：state / telemetry / error / middleware / config_loader / upstream_pool / handler / server / providers / admin / config_center
config/        # YAML 配置（default / development / production + routes/ + providers/）
tests/         # 集成测试
benches/       # 基准测试
plugins/       # 内置插件
deploy/        # 部署配置（Docker / K8s / systemd / Prometheus）
scripts/       # 运维脚本
```

## 文档

- [架构文档](docs/ARCHITECTURE.md) - 系统架构与设计决策
- [运维手册](docs/OPERATIONS.md) - 部署、监控与故障排查
- [项目规格](docs/rapidgate-spec.md) - 完整技术规格
- [协作规范](AGENTS.md) - 开发者与 AI 协作规范

## 许可证

[Apache-2.0](./Cargo.toml)
