# RapidGate

[English](./README.md) | [中文](./README_cn.md)

> High-performance unified LLM API gateway, built with Rust.

Inspired by [songquanpeng/one-api](https://github.com/songquanpeng/one-api) — rewritten in Rust.

## Project Status

- [x] **Stage 1 — Foundation** (commit `d9086df`): 23 commits, 6/6 checks passed, 44/44 tests passed
- [x] **Stage 2 — Hardening** (commit `7428e24`): Performance optimization, security hardening, test coverage, observability
- [x] **Stage 3 — Scaling** (commit `74f6e53`): Multi-provider support, plugin system, canary release, distributed rate limiting, admin API, deployment configs

Full roadmap: [`docs/rapidgate-spec.md`](./docs/rapidgate-spec.md)

## Features

- **Multi-Provider Support**: Unified access for OpenAI / Anthropic / Gemini / local models
- **Streaming First**: Native SSE and WebSocket support with zero-buffer forwarding
- **Dynamic Routing**: Flexible routing rules based on path, method, and headers with hot-reload
- **Canary Release**: Traffic splitting by weight, header, or cookie with session stickiness
- **Security**: SSRF protection, constant-time API key validation, IP allowlist
- **Observability**: Prometheus metrics, OpenTelemetry tracing, structured logging
- **Plugin System**: Extensible with native and WASM plugins
- **High Availability**: Circuit breaker, rate limiting, graceful shutdown
- **Distributed**: Redis rate limiting, ETCD/Consul config center

## AI-Assisted Development

This project is developed using AI collaboration with [Trae SOLO](https://www.trae.ai/) and [opencode](https://github.com/opencode-ai/opencode). The maintainer ([SharkMI](https://github.com/SharkMI-0x7E)) handles architecture design, approves every spec, and **runs all `cargo` commands personally** — AI handles code writing and commits. Commit rhythm follows "one feature = one commit" (see `AGENTS.md §1.1` and `§8.6`).

## Quick Start

### 1. Clone the project

```bash
git clone https://github.com/SharkMI-0x7E/RapidGate.git
cd RapidGate
```

### 2. Configure environment variables

```bash
cp .env.example .env
```

Edit `.env` file with your upstream Provider API keys:

```bash
RGD_OPENAI_API_KEY=sk-your-key-here
RGD_ADMIN_TOKEN=your-admin-token
```

### 3. Start the service

```bash
cargo run
```

Service listens on `0.0.0.0:8080` by default, configurable via `RGD_LISTEN` environment variable.

### Verify

```bash
# Health check
curl http://localhost:8080/healthz

# List available models
curl http://localhost:8080/v1/models

# Chat completion (streaming)
curl -N http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-4o",
    "messages": [{"role": "user", "content": "Hello"}],
    "stream": true
  }'
```

## Build & Test

```bash
cargo fmt -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```

## Deployment

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

See [Operations Manual](docs/OPERATIONS.md) for detailed deployment guide.

## Project Structure

Full directory tree: [`docs/rapidgate-spec.md` §2](./docs/rapidgate-spec.md).

```
src/
├── core/      # Business core: error / config / routing / proxy / auth / ratelimit / breaker / canary / plugins / observability / util
└── service/   # Service layer: state / telemetry / error / middleware / config_loader / upstream_pool / handler / server / providers / admin / config_center
config/        # YAML configs (default / development / production + routes/ + providers/)
tests/         # Integration tests
benches/       # Benchmark tests
plugins/       # Built-in plugins
deploy/        # Deployment configs (Docker / K8s / systemd / Prometheus)
scripts/       # Operations scripts
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md) - System architecture and design decisions
- [Operations Manual](docs/OPERATIONS.md) - Deployment, monitoring, and troubleshooting
- [Project Spec](docs/rapidgate-spec.md) - Complete technical specification
- [Collaboration Guide](AGENTS.md) - Developer and AI collaboration guidelines

## License

[Apache-2.0](./Cargo.toml)
