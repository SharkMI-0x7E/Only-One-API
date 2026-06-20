# RapidGate

[English](./README.md) | [中文](./README_cn.md)

> 高性能统一 LLM API 网关，Rust 实现。

灵感来自 [songquanpeng/one-api](https://github.com/songquanpeng/one-api) — 用 Rust 重写。

## 项目状态

- [x] **Stage 1 — 基础落地**（commit `d9086df`）：23 个 commit，6/6 自检全过，44/44 测试通过
- [x] **Stage 2 — 强化**（commit `7428e24`）：性能优化、安全加固、测试覆盖、可观测性完善
- [x] **Stage 3 — 规模化**（commit `74f6e53`）：多 Provider 支持、插件系统、灰度发布、分布式限流、Admin API、部署配置

完整规划：[`docs/rapidgate-spec.md`](./docs/rapidgate-spec.md)

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

## AI 辅助开发

本项目采用 **AI 协作**模式完成，使用 [Trae SOLO](https://www.trae.ai/) 和 [opencode](https://github.com/opencode-ai/opencode) 辅助开发。维护者（[SharkMI](https://github.com/SharkMI-0x7E)）负责架构设计、审批每个 spec、**亲自跑所有 `cargo` 命令** — AI 负责敲代码与写 commit。提交节奏为「一个功能 = 一个 commit」（见 `AGENTS.md §1.1` 与 `§8.6`）。

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
