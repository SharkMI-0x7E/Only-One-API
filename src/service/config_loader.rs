//! service/config_loader — YAML 加载 + dotenvy + 占位符展开（spec §5.5）
//!
//! 阶段一实现：
//! - 读 `RGD_CONFIG_DIR`（默认 `./config`）
//! - 加载 `default.yaml` → `development.yaml` / `production.yaml` 覆盖 → `routes/*.yaml`
//! - `${VAR}` 占位符展开；缺失则 `CoreError::Config`
//! - spec §7 R-1~R-8 校验
//! - 校验失败：返回 Result Err，**不 panic**
//!
//! 阶段二 [S2+] 增强：ArcSwap 热重载 + 校验失败回滚到旧配置

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;

use crate::core::config::gateway::GatewayConfig;
use crate::core::config::route::RouteConfig;
use crate::core::error::CoreError;

/// 加载结果：routes 在顶层（来自 routes/*.yaml 合并）
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadedConfig {
    #[serde(flatten)]
    pub gateway: GatewayConfig,
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
}

pub struct ConfigPaths {
    pub config_dir: PathBuf,
    pub default: PathBuf,
    pub env_overlay: Option<PathBuf>,
    pub routes_dir: PathBuf,
}

/// 解析配置目录与文件路径
pub fn resolve_paths() -> Result<ConfigPaths, CoreError> {
    let dir = std::env::var("RGD_CONFIG_DIR").unwrap_or_else(|_| "./config".to_string());
    let dir = PathBuf::from(dir);
    if !dir.exists() {
        return Err(CoreError::Config(format!(
            "config dir not found: {}",
            dir.display()
        )));
    }
    let default = dir.join("default.yaml");
    if !default.exists() {
        return Err(CoreError::Config(format!(
            "default.yaml missing in {}",
            dir.display()
        )));
    }
    let env = std::env::var("RGD_ENV").unwrap_or_else(|_| "development".to_string());
    let env_overlay = dir.join(format!("{env}.yaml"));
    let env_overlay = if env_overlay.exists() {
        Some(env_overlay)
    } else {
        None
    };
    let routes_dir = dir.join("routes");
    Ok(ConfigPaths {
        config_dir: dir,
        default,
        env_overlay,
        routes_dir,
    })
}

/// 主加载入口
pub async fn load() -> Result<Arc<LoadedConfig>, ServiceErrorWrapper> {
    let paths = resolve_paths().map_err(ServiceErrorWrapper)?;
    let mut text = read_to_string(&paths.default).map_err(ServiceErrorWrapper)?;
    if let Some(overlay) = &paths.env_overlay {
        text.push('\n');
        text.push_str(&read_to_string(overlay).map_err(ServiceErrorWrapper)?);
    }

    // 合并 routes/*.yaml
    let routes_yaml = collect_routes(&paths.routes_dir).map_err(ServiceErrorWrapper)?;
    for r in routes_yaml {
        text.push('\n');
        text.push_str(&r);
    }

    // 占位符展开
    let expanded = expand_placeholders(&text).map_err(ServiceErrorWrapper)?;

    // 解析：serde_yaml::Error -> CoreError（用已 derive 的 From）-> ServiceErrorWrapper
    let cfg: LoadedConfig = serde_yaml::from_str(&expanded)
        .map_err(CoreError::from)
        .map_err(ServiceErrorWrapper)?;
    validate(&cfg).map_err(ServiceErrorWrapper)?;
    Ok(Arc::new(cfg))
}

fn read_to_string(p: &Path) -> Result<String, CoreError> {
    std::fs::read_to_string(p).map_err(|e| CoreError::Config(format!("read {}: {e}", p.display())))
}

fn collect_routes(dir: &Path) -> Result<Vec<String>, CoreError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        out.push(read_to_string(&path)?);
    }
    Ok(out)
}

/// 展开 `${VAR}` 占位符；缺失则报错
fn expand_placeholders(input: &str) -> Result<String, CoreError> {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'$' && bytes[i + 1] == b'{' {
            // 找 }
            if let Some(end) = input[i + 2..].find('}') {
                let name = &input[i + 2..i + 2 + end];
                let val = std::env::var(name)
                    .map_err(|_| CoreError::Config(format!("placeholder ${{{name}}} not set")))?;
                out.push_str(&val);
                i = i + 2 + end + 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    Ok(out)
}

/// spec §7 校验 R-1~R-8
fn validate(cfg: &LoadedConfig) -> Result<(), CoreError> {
    // R-1：serde_yaml 已经解析过
    // R-2：expand_placeholders 已经处理
    // R-3：upstream.base_url 在 allowlist
    for up in &cfg.gateway.upstreams {
        check_base_url_allowed(&up.base_url, &cfg.gateway.upstream_allowlist.hosts)?;
    }
    // R-4：method 合法
    // R-5：path 合法
    // R-6：name 唯一
    let mut seen = std::collections::HashSet::new();
    for r in &cfg.routes {
        if !seen.insert(r.name.clone()) {
            return Err(CoreError::Config(format!(
                "duplicate route name: {}",
                r.name
            )));
        }
        if !is_valid_method(&r.match_rule.method) {
            return Err(CoreError::Config(format!(
                "route '{}': invalid method {}",
                r.name, r.match_rule.method
            )));
        }
        if !is_valid_path(&r.match_rule.path) {
            return Err(CoreError::Config(format!(
                "route '{}': invalid path {}",
                r.name, r.match_rule.path
            )));
        }
    }
    // R-7：rate_limit rps > 0 && burst > 0
    if cfg.gateway.defaults.rate_limit.rps == 0 || cfg.gateway.defaults.rate_limit.burst == 0 {
        return Err(CoreError::Config(
            "default rate_limit.rps/burst must be > 0".into(),
        ));
    }
    // R-8：api_key 长度
    for up in &cfg.gateway.upstreams {
        if up.api_key.len() < 16 {
            return Err(CoreError::Config(format!(
                "upstream '{}': api_key length < 16",
                up.id
            )));
        }
    }
    Ok(())
}

fn check_base_url_allowed(base_url: &str, allowlist: &[String]) -> Result<(), CoreError> {
    // 占位符已展开，直接用 url::Host 解析
    let host = base_url
        .split("://")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .and_then(|s| s.split(':').next())
        .ok_or_else(|| CoreError::Config(format!("invalid base_url: {base_url}")))?;
    if allowlist.iter().any(|h| h.eq_ignore_ascii_case(host)) {
        Ok(())
    } else {
        Err(CoreError::Config(format!(
            "base_url host '{host}' not in upstream_allowlist"
        )))
    }
}

fn is_valid_method(m: &str) -> bool {
    matches!(
        m.to_uppercase().as_str(),
        "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" | "ANY"
    )
}

fn is_valid_path(p: &str) -> bool {
    p.starts_with('/') && !p.ends_with('/')
}

/// 占位用的 ServiceError 包装器（避免与 service::error 循环依赖）
#[derive(Debug)]
pub struct ServiceErrorWrapper(pub CoreError);

impl From<CoreError> for ServiceErrorWrapper {
    fn from(e: CoreError) -> Self {
        Self(e)
    }
}

impl std::fmt::Display for ServiceErrorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ServiceErrorWrapper {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_simple() {
        std::env::set_var("RGD_TEST_PLACEHOLDER", "hello");
        let out = expand_placeholders("a ${RGD_TEST_PLACEHOLDER} b").unwrap();
        assert_eq!(out, "a hello b");
    }

    #[test]
    fn expand_missing_raises() {
        std::env::remove_var("RGD_TEST_MISSING");
        let r = expand_placeholders("${RGD_TEST_MISSING}");
        assert!(r.is_err());
    }

    #[test]
    fn check_allowlist() {
        assert!(
            check_base_url_allowed("https://api.openai.com/v1", &["api.openai.com".into()]).is_ok()
        );
        assert!(check_base_url_allowed("https://evil.com/v1", &["api.openai.com".into()]).is_err());
    }
}
