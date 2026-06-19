# RapidGate 运维手册

本文档提供 RapidGate 网关的部署、监控与故障排查指南。

---

## 1. 部署指南

### 1.1 Docker 部署

#### 构建镜像

```bash
# 多阶段构建（推荐）
docker build -f deploy/docker/Dockerfile -t rapidgate:latest .

# 指定版本
docker build -f deploy/docker/Dockerfile -t rapidgate:2026.06.1 .
```

#### 运行容器

```bash
docker run -d \
  --name rapidgate \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config:/app/config:ro \
  -e RGD_ENV=production \
  -e RGD_CONFIG_DIR=/app/config \
  -e RGD_OPENAI_API_KEY=your-key-here \
  -e RGD_ADMIN_TOKEN=admin-token-here \
  rapidgate:latest
```

#### docker-compose 部署

```yaml
# deploy/docker/docker-compose.yaml
version: '3.8'

services:
  rapidgate:
    image: rapidgate:latest
    ports:
      - "8080:8080"  # API 端口
      - "9090:9090"  # Admin 端口
    volumes:
      - ./config:/app/config:ro
    environment:
      - RGD_ENV=production
      - RGD_CONFIG_DIR=/app/config
      - RGD_OPENAI_API_KEY=${RGD_OPENAI_API_KEY}
      - RGD_ADMIN_TOKEN=${RGD_ADMIN_TOKEN}
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/healthz"]
      interval: 30s
      timeout: 10s
      retries: 3
```

启动：
```bash
cd deploy/docker
docker-compose up -d
```

### 1.2 Kubernetes 部署

#### 部署清单

```yaml
# deploy/k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rapidgate
  labels:
    app: rapidgate
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rapidgate
  template:
    metadata:
      labels:
        app: rapidgate
    spec:
      containers:
      - name: rapidgate
        image: rapidgate:latest
        ports:
        - containerPort: 8080
          name: api
        - containerPort: 9090
          name: admin
        env:
        - name: RGD_ENV
          value: "production"
        - name: RGD_CONFIG_DIR
          value: "/app/config"
        - name: RGD_OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: rapidgate-secrets
              key: openai-api-key
        - name: RGD_ADMIN_TOKEN
          valueFrom:
            secretKeyRef:
              name: rapidgate-secrets
              key: admin-token
        volumeMounts:
        - name: config
          mountPath: /app/config
          readOnly: true
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
        livenessProbe:
          httpGet:
            path: /healthz
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /readyz
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
      volumes:
      - name: config
        configMap:
          name: rapidgate-config
```

#### Service

```yaml
# deploy/k8s/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: rapidgate
spec:
  selector:
    app: rapidgate
  ports:
  - name: api
    port: 8080
    targetPort: 8080
  - name: admin
    port: 9090
    targetPort: 9090
  type: ClusterIP
```

#### Ingress

```yaml
# deploy/k8s/ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: rapidgate
  annotations:
    nginx.ingress.kubernetes.io/proxy-read-timeout: "60"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "60"
spec:
  rules:
  - host: api.example.com
    http:
      paths:
      - path: /
        pathType: Prefix
        backend:
          service:
            name: rapidgate
            port:
              number: 8080
```

#### 应用部署

```bash
# 创建 Secret
kubectl create secret generic rapidgate-secrets \
  --from-literal=openai-api-key=your-key \
  --from-literal=admin-token=your-token

# 创建 ConfigMap
kubectl create configmap rapidgate-config \
  --from-file=config/

# 部署
kubectl apply -f deploy/k8s/
```

### 1.3 systemd 部署

#### 服务文件

```ini
# deploy/systemd/rapidgate.service
[Unit]
Description=RapidGate LLM API Gateway
After=network.target

[Service]
Type=simple
User=rapidgate
Group=rapidgate
WorkingDirectory=/opt/rapidgate
ExecStart=/opt/rapidgate/rapidgate
Restart=always
RestartSec=10

# 环境变量
Environment=RGD_ENV=production
Environment=RGD_CONFIG_DIR=/opt/rapidgate/config
EnvironmentFile=/etc/rapidgate/env

# 安全加固
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
```

#### 安装步骤

```bash
# 创建用户
sudo useradd -r -s /bin/false rapidgate

# 安装二进制
sudo cp target/release/rapidgate /opt/rapidgate/
sudo chown rapidgate:rapidgate /opt/rapidgate/rapidgate

# 复制配置
sudo cp -r config /opt/rapidgate/
sudo chown -R rapidgate:rapidgate /opt/rapidgate/config

# 创建环境变量文件
sudo tee /etc/rapidgate/env > /dev/null <<EOF
RGD_OPENAI_API_KEY=your-key-here
RGD_ADMIN_TOKEN=your-token-here
EOF
sudo chmod 600 /etc/rapidgate/env

# 安装服务
sudo cp deploy/systemd/rapidgate.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable rapidgate
sudo systemctl start rapidgate

# 查看状态
sudo systemctl status rapidgate
```

---

## 2. 监控配置

### 2.1 Prometheus 指标

RapidGate 在 `/metrics` 端点暴露 Prometheus 格式指标（默认 9090 端口）。

#### 关键指标

| 指标名称 | 类型 | 说明 |
|---------|------|------|
| `rapidgate_requests_total` | Counter | 总请求数（按 route、status 分组） |
| `rapidgate_request_duration_seconds` | Histogram | 请求延迟分布 |
| `rapidgate_upstream_requests_total` | Counter | 上游请求数（按 provider、status 分组） |
| `rapidgate_upstream_duration_seconds` | Histogram | 上游响应延迟 |
| `rapidgate_active_connections` | Gauge | 当前活跃连接数 |
| `rapidgate_rate_limit_rejects_total` | Counter | 限流拒绝次数 |
| `rapidgate_circuit_breaker_state` | Gauge | 熔断器状态（0=closed, 1=open, 2=half-open） |
| `rapidgate_tokens_total` | Counter | Token 消耗总量（按 provider、model 分组） |

#### Prometheus 配置

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'rapidgate'
    scrape_interval: 15s
    metrics_path: /metrics
    static_configs:
      - targets: ['rapidgate:9090']
```

### 2.2 Grafana Dashboard

导入预置 Dashboard：

```bash
# 下载 Dashboard JSON
curl -O https://raw.githubusercontent.com/your-repo/rapidgate/main/deploy/grafana/dashboard.json

# 通过 Grafana UI 导入
# Dashboard -> Import -> Upload JSON file
```

#### 关键面板

- **请求速率**: `rate(rapidgate_requests_total[5m])`
- **延迟分布**: `histogram_quantile(0.99, rate(rapidgate_request_duration_seconds_bucket[5m]))`
- **错误率**: `rate(rapidgate_requests_total{status=~"5.."}[5m]) / rate(rapidgate_requests_total[5m])`
- **Token 消耗**: `sum(rate(rapidgate_tokens_total[5m])) by (provider)`

### 2.3 告警规则

```yaml
# deploy/prometheus/alerts.yaml
groups:
  - name: rapidgate
    rules:
      - alert: HighErrorRate
        expr: |
          rate(rapidgate_requests_total{status=~"5.."}[5m]) 
          / rate(rapidgate_requests_total[5m]) > 0.01
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "RapidGate 错误率过高 (>1%)"
          description: "5xx 错误率持续 5 分钟超过 1%"

      - alert: HighLatency
        expr: |
          histogram_quantile(0.99, 
            rate(rapidgate_request_duration_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "RapidGate P99 延迟过高 (>1s)"
          description: "P99 延迟持续 5 分钟超过 1 秒"

      - alert: CircuitBreakerOpen
        expr: rapidgate_circuit_breaker_state == 1
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "熔断器打开"
          description: "上游 {{ $labels.upstream }} 熔断器已打开"

      - alert: RateLimitRejects
        expr: rate(rapidgate_rate_limit_rejects_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "限流拒绝过多"
          description: "每分钟限流拒绝超过 600 次"
```

### 2.4 日志配置

#### 日志级别

通过环境变量调整：

```bash
# 开发环境（详细日志）
RGD_LOG_LEVEL=debug
RGD_LOG_FORMAT=pretty

# 生产环境（结构化 JSON）
RGD_LOG_LEVEL=info
RGD_LOG_FORMAT=json
```

#### 日志收集

```bash
# Docker 日志
docker logs -f rapidgate

# Kubernetes 日志
kubectl logs -f deployment/rapidgate

# systemd 日志
journalctl -u rapidgate -f
```

#### 结构化日志字段

JSON 格式日志包含以下字段：

```json
{
  "timestamp": "2026-06-19T10:30:00Z",
  "level": "INFO",
  "message": "request handled",
  "trace_id": "abc123",
  "route": "openai-chat",
  "method": "POST",
  "path": "/v1/chat/completions",
  "status": 200,
  "duration_ms": 1234,
  "upstream": "openai-primary",
  "provider": "openai",
  "prompt_tokens": 100,
  "completion_tokens": 50
}
```

---

## 3. 常见故障排查

### 3.1 启动失败

#### 配置加载失败

**症状**: 退出码 78

**排查步骤**:

```bash
# 查看详细错误
journalctl -u rapidgate -n 50 --no-pager

# 检查配置文件语法
cat config/default.yaml | python -c "import sys, yaml; yaml.safe_load(sys.stdin)"

# 检查环境变量
env | grep RGD_
```

**常见原因**:
- YAML 语法错误
- 环境变量缺失（`${RGD_*}` 未定义）
- 路由配置冲突（重复的 route name）
- upstream base_url 不在白名单

### 3.2 路由匹配失败

**症状**: 返回 404 `route_not_found`

**排查步骤**:

```bash
# 查看已加载路由
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:9090/admin/routes

# 检查路由配置
cat config/routes/v1.yaml

# 测试请求匹配
curl -v http://localhost:8080/v1/chat/completions \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4","messages":[]}'
```

**常见原因**:
- 路由未加载（配置文件未监听）
- Method/Path 不匹配
- 路由优先级冲突

### 3.3 上游连接失败

**症状**: 返回 502 `upstream_unreachable` 或 504 `upstream_timeout`

**排查步骤**:

```bash
# 检查上游健康状态
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:9090/admin/upstreams

# 测试上游连通性
curl -v https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# 检查 DNS 解析
nslookup api.openai.com

# 检查 SSRF 防护
# 查看日志是否触发 IP 黑名单
journalctl -u rapidgate | grep "blocked upstream"
```

**常见原因**:
- 上游 API Key 无效
- 网络不通（防火墙/代理）
- SSRF 防护拦截（私有 IP 段）
- 上游超时（调整 `upstream.timeout_ms`）

### 3.4 限流触发

**症状**: 返回 429 `rate_limited`

**排查步骤**:

```bash
# 查看限流状态
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:9090/admin/limits

# 调整限流配置
# config/routes/v1.yaml
routes:
  - name: "openai-chat"
    rate_limit:
      algorithm: "token_bucket"
      rps: 10      # 增加每秒请求数
      burst: 20    # 增加突发容量
```

**常见原因**:
- 限流阈值过低
- 客户端未实现退避重试
- 多实例未使用分布式限流（Redis）

### 3.5 熔断器打开

**症状**: 返回 503 `breaker_open`

**排查步骤**:

```bash
# 查看熔断器状态
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:9090/admin/upstreams

# 检查上游错误率
# Grafana 面板: rapidgate_circuit_breaker_state

# 手动重置熔断器（需要重启）
systemctl restart rapidgate
```

**常见原因**:
- 上游服务故障
- 上游错误率过高（超过 `failure_threshold`）
- 网络抖动

**恢复**:
- 等待 `open_duration_ms`（默认 30s）后自动进入 half-open
- 修复上游问题
- 调整熔断器阈值（`config/default.yaml`）

### 3.6 配置热重载失败

**症状**: 修改配置后未生效

**排查步骤**:

```bash
# 检查文件监听
ls -l config/routes/

# 查看日志
journalctl -u rapidgate | grep "hot reload"

# 手动触发重载（通过 admin API）
curl -X POST \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:9090/admin/reload

# 检查配置校验
# 日志会输出校验失败原因
```

**常见原因**:
- 配置文件权限问题（rapidgate 用户无读权限）
- 配置校验失败（保留旧配置）
- notify 监听未启动

### 3.7 性能问题

**症状**: 延迟升高、吞吐量下降

**排查步骤**:

```bash
# 查看 Prometheus 指标
curl http://localhost:9090/metrics | grep rapidgate_request_duration

# 检查系统资源
top -p $(pgrep rapidgate)
ss -s

# 检查连接池
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:9090/admin/upstreams

# 调整参数
# config/default.yaml
gateway:
  request_timeout_ms: 60000
  max_body_bytes: 52428800

defaults:
  rate_limit:
    rps: 100    # 提高限流
    burst: 200
```

**常见原因**:
- 上游响应慢
- 连接池耗尽
- 限流阈值过低
- 内存不足（检查容器 limits）

### 3.8 优雅关闭失败

**症状**: 重启时丢失请求

**排查步骤**:

```bash
# 检查关闭超时
systemctl stop rapidgate
journalctl -u rapidgate | grep "draining"

# 调整超时
# config/default.yaml
gateway:
  shutdown_timeout_ms: 30000  # 增加到 30s
```

**常见原因**:
- `shutdown_timeout_ms` 过短
- 客户端未实现重试
- 长连接未断开（WebSocket）

---

## 4. 性能调优

### 4.1 系统参数

```bash
# /etc/sysctl.conf
# 增加文件描述符限制
fs.file-max = 1000000

# TCP 连接优化
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 65535
net.ipv4.tcp_tw_reuse = 1

# 增加端口范围
net.ipv4.ip_local_port_range = 1024 65535
```

应用：
```bash
sudo sysctl -p
```

### 4.2 进程参数

```bash
# systemd 服务
[Service]
LimitNOFILE=1000000
LimitNPROC=65535

# Docker
docker run --ulimit nofile=1000000:1000000 ...

# Kubernetes
spec:
  containers:
  - name: rapidgate
    resources:
      limits:
        cpu: "2"
        memory: 2Gi
```

### 4.3 应用参数

```yaml
# config/production.yaml
gateway:
  request_timeout_ms: 60000
  shutdown_timeout_ms: 30000

defaults:
  rate_limit:
    rps: 100
    burst: 200
  breaker:
    failure_threshold: 10
    open_duration_ms: 30000

upstreams:
  - id: "openai-primary"
    pool:
      idle_timeout_secs: 120
      max_idle_per_host: 50  # 增加连接池
```

---

## 5. 安全加固

### 5.1 网络隔离

```bash
# 仅允许内网访问 admin 端口
iptables -A INPUT -p tcp --dport 9090 -s 10.0.0.0/8 -j ACCEPT
iptables -A INPUT -p tcp --dport 9090 -j DROP

# Kubernetes NetworkPolicy
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: rapidgate-admin
spec:
  podSelector:
    matchLabels:
      app: rapidgate
  ingress:
  - from:
    - ipBlock:
        cidr: 10.0.0.0/8
    ports:
    - port: 9090
```

### 5.2 TLS 终止

```bash
# Nginx 反向代理
server {
    listen 443 ssl;
    server_name api.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://rapidgate:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        
        # SSE 支持
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 300s;
    }
}
```

### 5.3 API Key 轮换

```bash
# 1. 生成新 Key
NEW_KEY=$(openssl rand -hex 32)

# 2. 更新 Secret
kubectl create secret generic rapidgate-secrets \
  --from-literal=openai-api-key=$NEW_KEY \
  --dry-run=client -o yaml | kubectl apply -f -

# 3. 滚动重启
kubectl rollout restart deployment/rapidgate

# 4. 验证
kubectl rollout status deployment/rapidgate
```

---

## 6. 备份与恢复

### 6.1 配置备份

```bash
# 备份配置文件
tar -czf rapidgate-config-$(date +%Y%m%d).tar.gz config/

# 备份环境变量
cp /etc/rapidgate/env rapidgate-env-$(date +%Y%m%d).bak
```

### 6.2 恢复步骤

```bash
# 1. 恢复配置
tar -xzf rapidgate-config-20260619.tar.gz -C /opt/rapidgate/

# 2. 恢复环境变量
cp rapidgate-env-20260619.bak /etc/rapidgate/env

# 3. 重启服务
systemctl restart rapidgate
```

---

## 7. 升级指南

### 7.1 滚动升级（Kubernetes）

```bash
# 1. 更新镜像
kubectl set image deployment/rapidgate rapidgate=rapidgate:v2026.06.2

# 2. 观察滚动状态
kubectl rollout status deployment/rapidgate

# 3. 验证健康
kubectl get pods -l app=rapidgate
curl http://rapidgate:8080/readyz
```

### 7.2 蓝绿部署

```bash
# 1. 部署新版本（绿色）
kubectl apply -f deploy/k8s/deployment-green.yaml

# 2. 验证绿色环境
curl http://rapidgate-green:8080/readyz

# 3. 切换流量
kubectl patch service rapidgate -p '{"spec":{"selector":{"version":"green"}}}'

# 4. 删除旧版本（蓝色）
kubectl delete deployment rapidgate-blue
```

---

## 8. 联系支持

- GitHub Issues: https://github.com/SharkMI-0x7E/RapidGate/issues
- 文档: https://github.com/SharkMI-0x7E/RapidGate/tree/main/docs
