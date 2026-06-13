# Rust 项目 CI/CD 全套工作流生成提示词（rapidgate 定制版）

> 本文件是为 AI 准备的"提示词模板"，用于一次性生成项目的全套 GitHub Actions 配置与协作规范。请按本文件中的所有要求严格执行，不要省略、简化或自由发挥。

---

## 0. 角色

你是一名精通 Rust 项目工程化的 CI/CD 专家，擅长 GitHub Actions 配置、安全合规与自动化发布。

## 1. 项目背景

- **项目名**：`rapidgate`
- **仓库**：https://github.com/SharkMI-0x7E/RapidGate
- **二进制名**：`rapidgate`
- **许可证**：Apache-2.0
- **技术栈**：Rust 2021 edition；依赖 axum 0.7、tokio（full features）、reqwest、serde / serde_json、tracing / tracing-subscriber
- **项目类型**：单仓库、可执行 Rust 程序（API 网关类服务）

## 2. 必须遵守的核心原则

1. 所有第三方 Action 必须固定到精确的提交 SHA，注释中注明版本号。
2. 所有作业默认权限设置为 `{}`，仅在需要时单独声明。
3. **必须使用 `concurrency` 避免同一 PR/branch 重复运行**：
   - `ci.yml`：`cancel-in-progress: ${{ github.event_name == 'pull_request' }}`
   - `release.yml`：`cancel-in-progress: false`（tag 不可被取消）
   - `release-plz.yml`：`cancel-in-progress: false`
4. **lint/test 步骤必须跑在 `stable` 和 `beta` 两个 Rust 频道**，`fail-fast: false`。`stable` 覆盖三平台（ubuntu/windows/macos），`beta` 仅在 ubuntu-latest 上跑以节省资源。
5. 构建产物是 Rust 原生二进制文件，不做容器镜像、不做 .pyz 打包、不上 crates.io。
6. **版本号控制权完全在开发者手中**：
   - release-plz 只跑 `release-pr` 命令（生成发版 PR），**不跑** `release` 命令
   - release-plz 创建的 PR 不会被自动合并，必须人工 review
   - 开发者可在合并前修改 PR 中 `Cargo.toml` 的 version 字段
   - 开发者也可关闭该 PR，自行修改 `Cargo.toml` 后手动打 `v*` tag 触发 `release.yml`
   - 禁止任何工具自动修改 `Cargo.toml` 的 version 字段后再自动发版
7. 禁止任何占位符（如 `{PROJECT_NAME}`）。项目名一律 `rapidgate`，二进制名一律 `rapidgate`。

## 3. 需要生成的文件

1. `.github/workflows/ci.yml` — 持续集成（fmt、clippy、test、unsafe 检查、发布构建冒烟测试）
2. `.github/workflows/release.yml` — 按 tag 发布多平台二进制，创建 GitHub Release
3. `.github/workflows/security.yml` — 定时依赖漏洞扫描与许可证检查
4. `.github/workflows/release-plz.yml` — 配合 release-plz 自动生成发版 PR（**仅生成，不自动发布**）
5. `deny.toml` — cargo-deny 配置（项目根目录）
6. `.github/release.yml` — release-plz 变更日志分类配置
7. `.github/PULL_REQUEST_TEMPLATE.md` — PR 模板
8. `AGENTS.md` — 协作规范（项目根目录）

## 4. 各文件内容模板

### 4.1 `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main]
    paths-ignore:
      - "**.md"
      - "docs/**"
      - ".trae/**"
  pull_request:
    branches: [main]
    paths-ignore:
      - "**.md"
      - "docs/**"
      - ".trae/**"

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}

permissions: {}

jobs:
  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with:
          persist-credentials: false
      - uses: dtolnay/rust-toolchain@a54c7afa936fefeb4456b2dd8068152619aa8d5b # stable
        with:
          components: rustfmt
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.7
      - run: cargo fmt -- --check

  lint-test:
    name: Clippy & Test (${{ matrix.rust }} on ${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable, beta]
        exclude:
          - os: windows-latest
            rust: beta
          - os: macos-latest
            rust: beta
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with: { persist-credentials: false }
      - uses: dtolnay/rust-toolchain@a54c7afa936fefeb4456b2dd8068152619aa8d5b # stable
        with:
          toolchain: ${{ matrix.rust }}
          components: clippy
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.7
      - run: cargo clippy --all-targets --all-features -- -D warnings
      - run: cargo build --all-targets --all-features
      - run: cargo test --all-features

  unsafe-check:
    name: Unsafe Code Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with: { persist-credentials: false }
      - name: Check for unsafe code
        run: |
          if grep -rn "unsafe" src/ --include="*.rs"; then
            echo "::error::Found 'unsafe' keyword in src/ directory!"
            exit 1
          fi
          echo "PASS: No unsafe code found in src/"

  release-build:
    name: Release Build
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with: { persist-credentials: false }
      - uses: dtolnay/rust-toolchain@a54c7afa936fefeb4456b2dd8068152619aa8d5b # stable
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.7
      - run: cargo build --release
```

### 4.2 `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: false

permissions:
  contents: write

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            ext: .exe
            archive-ext: zip
          - target: aarch64-pc-windows-msvc
            os: windows-latest
            ext: .exe
            archive-ext: zip
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            ext: ""
            archive-ext: tar.gz
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            ext: ""
            archive-ext: tar.gz
          - target: x86_64-apple-darwin
            os: macos-latest
            ext: ""
            archive-ext: zip
          - target: aarch64-apple-darwin
            os: macos-latest
            ext: ""
            archive-ext: zip
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with: { persist-credentials: false }
      - uses: dtolnay/rust-toolchain@a54c7afa936fefeb4456b2dd8068152619aa8d5b # stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.7
      - run: cargo build --release --target ${{ matrix.target }}
      - name: Package binary
        shell: bash
        id: package
        run: |
          BINARY_NAME="rapidgate${{ matrix.ext }}"
          SOURCE="target/${{ matrix.target }}/release/${BINARY_NAME}"
          STAGING="rapidgate-${{ github.ref_name }}-${{ matrix.target }}"
          mkdir -p "$STAGING"
          cp "$SOURCE" "$STAGING/"
          if [ "${{ matrix.archive-ext }}" = "tar.gz" ]; then
            tar -czf "${STAGING}.tar.gz" "$STAGING"
            echo "artifact=${STAGING}.tar.gz" >> "$GITHUB_OUTPUT"
          else
            zip -r "${STAGING}.zip" "$STAGING"
            echo "artifact=${STAGING}.zip" >> "$GITHUB_OUTPUT"
          fi
      - uses: actions/upload-artifact@4cec3d8aa04e39d1a68397de0c4cd6fb9dce8ec1 # v4.6.1
        with:
          name: binary-${{ matrix.target }}
          path: ${{ steps.package.outputs.artifact }}

  release:
    name: Create Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with: { fetch-depth: 0, persist-credentials: false }
      - uses: actions/download-artifact@cc203385981b70ca67e1cc39263f7d9bd4616c83 # v4.1.9
        with: { path: artifacts }
      - run: |
          mkdir -p release-files
          find artifacts -type f \( -name "*.zip" -o -name "*.tar.gz" \) -exec cp {} release-files/ \;
      - run: cd release-files && sha256sum * > SHA256SUMS
      - id: prerelease
        run: |
          TAG="${{ github.ref_name }}"
          if [[ "$TAG" =~ ^v0\. ]]; then echo "is-prerelease=true" >> "$GITHUB_OUTPUT";
          else echo "is-prerelease=false" >> "$GITHUB_OUTPUT"; fi
      - uses: softprops/action-gh-release@c95fe1489396fe8a9eb87c0abf8aa5b2ef267fda # v2.2.1
        with:
          generate_release_notes: true
          prerelease: ${{ steps.prerelease.outputs.is-prerelease }}
          files: release-files/**
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### 4.3 `.github/workflows/security.yml`

```yaml
name: Security Audit

on:
  schedule:
    - cron: "0 6 * * *"
  pull_request:
    branches: [main]
    paths:
      - "Cargo.lock"
      - "Cargo.toml"
      - "deny.toml"

env:
  CARGO_TERM_COLOR: always

permissions: {}

jobs:
  audit:
    name: Dependency Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with: { persist-credentials: false }
      - uses: dtolnay/rust-toolchain@a54c7afa936fefeb4456b2dd8068152619aa8d5b # stable
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.7
      - uses: taiki-e/install-action@a2f3489041e887ce15e3cb759da0b7b1d2d65c25 # v2
        with: { tool: cargo-audit }
      - run: cargo audit

  deny:
    name: License Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with: { persist-credentials: false }
      - uses: dtolnay/rust-toolchain@a54c7afa936fefeb4456b2dd8068152619aa8d5b # stable
      - uses: Swatinem/rust-cache@23bce251a8cd2ffc3c1075eaa2367cf899916d84 # v2.7.7
      - uses: taiki-e/install-action@a2f3489041e887ce15e3cb759da0b7b1d2d65c25 # v2
        with: { tool: cargo-deny }
      - run: cargo deny check
```

### 4.4 `.github/workflows/release-plz.yml`

```yaml
name: Release Plz

on:
  push:
    branches: [main]
    paths-ignore:
      - "**.md"
      - "docs/**"
      - ".trae/**"

concurrency:
  group: release-plz-${{ github.ref }}
  cancel-in-progress: false

permissions:
  contents: write
  pull-requests: write

jobs:
  release-plz:
    # 注意：本工作流仅生成发版 PR（含 changelog 与版本号建议），不会自动发布。
    # 开发者必须人工 review PR 中的 Cargo.toml version 字段，确认无误后再合并。
    # 合并后 PR 才会真正修改版本号；也完全可以拒绝该 PR，
    # 由开发者自行修改 Cargo.toml 后手动打 v* tag 触发 release.yml。
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4.2.2
        with: { fetch-depth: 0, persist-credentials: false }
      - uses: dtolnay/rust-toolchain@a54c7afa936fefeb4456b2dd8068152619aa8d5b # stable
      - uses: release-plz/action@v0
        with:
          # 仅生成发版 PR，绝不使用 release 命令
          command: release-pr
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### 4.5 `deny.toml`（项目根目录）

```toml
[graph]
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
]
all-features = true

[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
yanked = "warn"
ignore = []

[bans]
multiple-versions = "warn"
wildcards = "allow"
highlight = "all"

[licenses]
unlicensed = "deny"
allow-osi-fsf-free = "neither"
copyleft = "warn"
confidence-threshold = 0.8
allow = [
    "Apache-2.0",
    "MIT",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
    "MPL-2.0",
    "OpenSSL",
    "Apache-2.0 WITH LLVM-exception",
    "Unicode-DFS-2016",
    "BSL-1.0",
]

exceptions = [
    { allow = ["CC0-1.0"], crate = "tiny-keccak" },
]

[sources]
unknown-registry = "warn"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

### 4.6 `.github/release.yml`

```yaml
changelog:
  exclude:
    labels:
      - ignore-for-release
    authors:
      - github-actions
  categories:
    - title: Security
      labels:
        - security
    - title: Breaking Changes
      labels:
        - breaking
    - title: Features
      labels:
        - feat
        - feature
        - enhancement
    - title: Bug Fixes
      labels:
        - fix
        - bug
    - title: Documentation
      labels:
        - docs
        - documentation
    - title: Performance
      labels:
        - perf
        - performance
    - title: Other Changes
      labels:
        - "*"
```

### 4.7 `.github/PULL_REQUEST_TEMPLATE.md`

```markdown
## What
<!-- Describe what this PR does -->

## Why
<!-- Explain why this change is needed -->

## How
<!-- Describe how the change was implemented (for complex logic or design decisions) -->

## Testing
<!-- Describe how this was tested, what scenarios are covered -->

## Checklist
- [ ] `cargo fmt -- --check` passes locally
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes locally
- [ ] `cargo test --all-features` passes locally
- [ ] Code contains no `unsafe` blocks
- [ ] Sensitive configuration is loaded from environment variables, not hardcoded
- [ ] Commit messages follow Conventional Commits format
- [ ] No unnecessary dependencies added
```

### 4.8 `AGENTS.md`（项目根目录）

```markdown
# AGENTS.md — rapidgate 协作规范

本文档面向所有参与 rapidgate 项目的开发者与 AI 编码助手，约束开发、协作、CI/CD 行为。

## 1. 提交规范

本项目遵循 [Conventional Commits](https://www.conventionalcommits.org/)，提交信息格式：

    <type>(<scope>): <subject>

允许的 type：

- `feat` — 新功能
- `fix` — Bug 修复
- `docs` — 仅文档变更
- `chore` — 构建/工具/杂项
- `ci` — CI/CD 配置变更
- `test` — 测试新增或修改
- `perf` — 性能优化
- `refactor` — 代码重构（非功能变更、非 Bug 修复）
- `style` — 代码格式（不影响语义）

## 2. PR 要求

**必须**通过 CI 所有检查：

- `fmt` 作业（`cargo fmt -- --check`）
- `lint-test` 作业（`cargo clippy --all-targets --all-features -- -D warnings`、`cargo build --all-targets --all-features`、`cargo test --all-features`）
- `unsafe-check` 作业
- `release-build` 作业

**代码硬性约束**：

- 禁止出现 `unsafe` 代码块（除非已充分评审并在 PR 描述中写明原因）
- 敏感配置（API key、token、base URL 等）必须通过环境变量读取，不得硬编码
- 遵循 `.github/PULL_REQUEST_TEMPLATE.md`，写清 What / Why / How / Testing

## 3. 构建说明

```bash
# 本地开发运行
cargo run

# 运行所有测试
cargo test --all-features

# 格式化检查
cargo fmt -- --check

# 静态分析
cargo clippy --all-targets --all-features -- -D warnings

# 发布构建
cargo build --release

# 依赖漏洞扫描
cargo install cargo-audit --locked
cargo audit

# 许可证/重复依赖/公告检查
cargo install cargo-deny --locked
cargo deny check
```

## 4. 安全策略

- **依赖漏洞**：通过 `cargo audit`（GitHub Actions 中的 `audit` 作业）持续监控，每天 06:00 UTC 自动扫描
- **许可证合规**：通过 `cargo deny` 持续检查，禁止引入未在 `deny.toml` 白名单中的许可证
- **新增依赖评估**：必须评估其维护活跃度（最近 6 个月有提交）、依赖树影响（直接依赖数量、总编译时间）、许可证兼容性

## 5. CI/CD 工作流说明

| 工作流 | 触发条件 | 作用 |
| --- | --- | --- |
| `ci.yml` | push/PR 到 main（排除 .md、docs/、.trae/） | 格式、clippy、test、unsafe 检查、发布构建冒烟 |
| `release.yml` | 推送 `v*` tag | 多平台构建 + 创建 GitHub Release |
| `release-plz.yml` | push 到 main | 自动生成发版 PR（含 changelog 与版本号建议），**不自动发布** |
| `security.yml` | 每天 06:00 UTC + PR 改动 Cargo 文件 | `cargo audit` 漏洞扫描 + `cargo deny` 许可证检查 |

## 6. 版本号管理策略

**版本号控制权完全在开发者手中**。本项目不使用 release-plz 自动发布，只用它生成发版建议。

发版流程：

1. 开发者按 Conventional Commits 规范提交代码
2. `release-plz.yml` 在 main 推送后自动创建发版 PR，含 changelog 与版本号建议
3. **开发者 review PR**：
   - 若同意建议的版本号 → 合并 PR（合并后 `Cargo.toml` 的 version 字段才会被真正修改）
   - 若希望使用其他版本号 → 在 PR 中手动编辑 `Cargo.toml` 的 version，再合并
   - 若完全想跳过 release-plz → 关闭该 PR，开发者自行修改 `Cargo.toml` 后手动打 `v*` tag 触发 `release.yml`
4. 手动打 `v*` tag（例如 `git tag v0.2.0 && git push origin v0.2.0`）触发 `release.yml` 构建并发布

**禁止**：依赖任何工具自动修改 `Cargo.toml` 的 version 字段后再自动发布。

## 7. 技术栈约束

- **Web 框架**：axum 0.7（路由、状态提取、响应）
- **异步运行时**：tokio（full features）
- **HTTP 客户端**：reqwest（json + stream）
- **日志**：tracing + tracing-subscriber
- **序列化**：serde / serde_json
- **Rust 版本**：stable（CI 还覆盖 beta 频道）

## 8. AI 协作注意事项

- 生成代码中**禁止**包含 `unsafe` 块
- 遵守现有 axum handler 模式、`AppError` → `IntoResponse` 的错误处理风格
- 严禁在源码中硬编码任何敏感字符串（API key、token、base URL）
- 新增依赖前必须评估其许可证、维护活跃度、依赖树大小
- 修改后必须运行 `cargo fmt`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all-features` 全部通过
- 遵循现有文件命名与模块组织风格
```

## 5. 输出要求

- 确保文件内容完整、可直接使用，仅将项目名/二进制名替换为 `rapidgate`，其余与第 4 节模板完全一致
- 不得简化、省略任何步骤，必须包含注释中的版本 SHA 和说明
