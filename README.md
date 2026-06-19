# RapidGate

高性能统一 LLM API 网关，基于 Rust 构建，支持多 Provider 接入、流式响应转发、动态路由与灰度发布。

---

## 特性

- **多 Provider 支持**: OpenAI / Anthropic / Gemini / 本地模型统一接入
- **流式优先**: SSE 与 WebSocket 原生支持，零缓冲转发
- **动态路由**: 基于路径、方法、Header 的灵活路由规则，支持热重载
- **灰度发布**: 按权重、Header、Cookie 分流，支持会话黏性
- **安全防护**: SSRF 防护、API Key 常量时间校验、IP 白名单
- **可观测性**: Prometheus 指标导出、OpenTelemetry 追踪、结构化日志
- **插件系统**: 支持 Native 与 WASM 插件扩展
- **高可用**: 熔断器、限流、优雅关闭

---

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

---

## 配置示例

### 路由配置

编辑 `config/routes/v1.yaml`：

```yaml
upstreams:
  - id: "openai-primary"
    provider: "openai"
    base_url: "${RGD_OPENAI_BASE_URL}"
    api_key: "${RGD_OPENAI_API_KEY}"
    models:
      - "gpt-4o"
      - "gpt-4o-mini"

routes:
  - name: "openai-chat"
    match:
      method: POST
      path: "/v1/chat/completions"
    upstream:
      id: "openai-primary"
    auth:
      type: "bearer"
    rate_limit:
      algorithm: "token_bucket"
      rps: 10
      burst: 20
```

### 全局配置

编辑 `config/default.yaml`：

```yaml
gateway:
  listen: "0.0.0.0:8080"
  request_timeout_ms: 60000
  max_body_bytes: 52428800
  shutdown_timeout_ms: 15000

logging:
  level: "info"
  format: "pretty"

defaults:
  rate_limit:
    algorithm: "token_bucket"
    rps: 10
    burst: 20
  breaker:
    failure_threshold: 5
    open_duration_ms: 30000
```

详细配置说明请参考 [架构文档](docs/ARCHITECTURE.md)。

---

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

---

## 架构

RapidGate 采用分层架构设计：

- **core 层**: 纯业务逻辑，无 I/O 依赖，包含路由匹配、限流算法、熔断器、认证等核心模块
- **service 层**: 框架集成，负责 HTTP 处理、配置加载、Provider 适配、插件系统

架构图与设计决策记录详见 [架构文档](docs/ARCHITECTURE.md)。

---

## 监控

RapidGate 在 `/metrics` 端点暴露 Prometheus 格式指标：

- `rapidgate_requests_total`: 请求总数
- `rapidgate_request_duration_seconds`: 请求延迟分布
- `rapidgate_upstream_requests_total`: 上游请求统计
- `rapidgate_tokens_total`: Token 消耗量

预置 Grafana Dashboard 与告警规则位于 `deploy/prometheus/` 目录。

监控配置与故障排查请参考 [运维手册](docs/OPERATIONS.md)。

---

## 开发

### 构建

```bash
cargo build --release
```

### 测试

```bash
cargo test --all-features
```

### 代码检查

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

### 项目结构

```
src/
├── core/           # 核心业务逻辑（无 I/O）
│   ├── config/     # 配置数据模型
│   ├── routing/    # 路由匹配引擎
│   ├── proxy/      # 转发逻辑
│   ├── auth/       # 认证抽象
│   ├── ratelimit/  # 限流算法
│   ├── breaker/    # 熔断器
│   └── plugins/    # 插件系统
└── service/        # 框架集成层
    ├── middleware/ # HTTP 中间件
    ├── providers/  # Provider 适配
    ├── admin/      # 管理 API
    └── handler.rs  # 路由处理器
```

---

## 文档

- [架构文档](docs/ARCHITECTURE.md) - 系统架构与设计决策
- [运维手册](docs/OPERATIONS.md) - 部署、监控与故障排查
- [项目规格](docs/rapidgate-spec.md) - 完整技术规格
- [协作规范](AGENTS.md) - 开发者与 AI 协作规范

---

## 许可证

Apache-2.0

---

## 贡献

欢迎提交 Issue 与 Pull Request。提交前请确保通过所有 CI 检查：

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`

详细贡献指南请参考 [协作规范](AGENTS.md)。
