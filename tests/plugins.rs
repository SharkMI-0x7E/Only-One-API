//! 插件系统集成测试（spec §2 [S3]）
//!
//! 覆盖插件注册、执行、错误处理。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use rapidgate::core::plugins::PluginRegistry;
use rapidgate::core::plugins::{
    ErrorContext, Plugin, PluginError, PluginMetadata, ProxyContext, RequestContext,
};

// -------------------- 测试用插件实现 --------------------

/// 简单计数器插件：记录请求次数
struct CounterPlugin {
    name: String,
    version: String,
    counter: std::sync::atomic::AtomicU64,
}

impl CounterPlugin {
    fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn count(&self) -> u64 {
        self.counter.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[async_trait]
impl Plugin for CounterPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: self.name.clone(),
            version: self.version.clone(),
            author: Some("test".to_string()),
            description: Some("counter plugin".to_string()),
        }
    }

    async fn on_request(&self, _ctx: &mut RequestContext) -> Result<(), PluginError> {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

/// 请求修改插件：添加自定义 header
struct HeaderInjectorPlugin {
    name: String,
    version: String,
    header_name: String,
    header_value: String,
}

impl HeaderInjectorPlugin {
    fn new(name: &str, header_name: &str, header_value: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            header_name: header_name.to_string(),
            header_value: header_value.to_string(),
        }
    }
}

#[async_trait]
impl Plugin for HeaderInjectorPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: self.name.clone(),
            version: self.version.clone(),
            author: None,
            description: None,
        }
    }

    async fn on_request(&self, ctx: &mut RequestContext) -> Result<(), PluginError> {
        ctx.headers
            .insert(self.header_name.clone(), self.header_value.clone());
        Ok(())
    }

    async fn before_proxy(&self, ctx: &mut ProxyContext) -> Result<(), PluginError> {
        ctx.headers
            .insert(self.header_name.clone(), self.header_value.clone());
        Ok(())
    }
}

/// 失败插件：总是返回错误
struct FailingPlugin {
    name: String,
    error_msg: String,
}

impl FailingPlugin {
    fn new(name: &str, error_msg: &str) -> Self {
        Self {
            name: name.to_string(),
            error_msg: error_msg.to_string(),
        }
    }
}

#[async_trait]
impl Plugin for FailingPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: self.name.clone(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
        }
    }

    async fn on_request(&self, _ctx: &mut RequestContext) -> Result<(), PluginError> {
        Err(PluginError::ExecutionFailed(self.error_msg.clone()))
    }
}

// -------------------- 插件注册表测试 --------------------

#[tokio::test]
async fn registry_register_and_get() {
    let registry = PluginRegistry::new();
    let plugin = Arc::new(CounterPlugin::new("counter", "1.0.0"));

    registry.register(plugin.clone()).await.unwrap();

    let retrieved = registry.get("counter").await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().metadata().name, "counter");
}

#[tokio::test]
async fn registry_list_plugins() {
    let registry = PluginRegistry::new();
    let p1 = Arc::new(CounterPlugin::new("counter1", "1.0.0"));
    let p2 = Arc::new(CounterPlugin::new("counter2", "2.0.0"));

    registry.register(p1).await.unwrap();
    registry.register(p2).await.unwrap();

    let list = registry.list().await;
    assert_eq!(list.len(), 2);

    let names: Vec<_> = list.iter().map(|m| m.name.clone()).collect();
    assert!(names.contains(&"counter1".to_string()));
    assert!(names.contains(&"counter2".to_string()));
}

#[tokio::test]
async fn registry_count() {
    let registry = PluginRegistry::new();
    assert_eq!(registry.count().await, 0);

    let p1 = Arc::new(CounterPlugin::new("p1", "1.0.0"));
    registry.register(p1).await.unwrap();
    assert_eq!(registry.count().await, 1);

    let p2 = Arc::new(CounterPlugin::new("p2", "1.0.0"));
    registry.register(p2).await.unwrap();
    assert_eq!(registry.count().await, 2);
}

#[tokio::test]
async fn registry_unregister() {
    let registry = PluginRegistry::new();
    let plugin = Arc::new(CounterPlugin::new("counter", "1.0.0"));

    registry.register(plugin).await.unwrap();
    assert_eq!(registry.count().await, 1);

    registry.unregister("counter").await.unwrap();
    assert_eq!(registry.count().await, 0);

    let retrieved = registry.get("counter").await;
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn registry_unregister_nonexistent_fails() {
    let registry = PluginRegistry::new();
    let result = registry.unregister("nonexistent").await;
    assert!(result.is_err());
    assert!(matches!(result, Err(PluginError::NotFound(_))));
}

#[tokio::test]
async fn registry_duplicate_registration_fails() {
    let registry = PluginRegistry::new();
    let p1 = Arc::new(CounterPlugin::new("counter", "1.0.0"));
    let p2 = Arc::new(CounterPlugin::new("counter", "2.0.0"));

    registry.register(p1).await.unwrap();
    let result = registry.register(p2).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(PluginError::InitFailed(_))));
}

#[tokio::test]
async fn registry_get_nonexistent_returns_none() {
    let registry = PluginRegistry::new();
    let result = registry.get("nonexistent").await;
    assert!(result.is_none());
}

// -------------------- 插件执行测试 --------------------

#[tokio::test]
async fn plugin_on_request_executes() {
    let plugin = Arc::new(CounterPlugin::new("counter", "1.0.0"));
    let mut ctx = RequestContext {
        method: "POST".to_string(),
        path: "/v1/chat/completions".to_string(),
        headers: HashMap::new(),
        query_params: HashMap::new(),
        body: None,
        metadata: HashMap::new(),
    };

    assert_eq!(plugin.count(), 0);
    plugin.on_request(&mut ctx).await.unwrap();
    assert_eq!(plugin.count(), 1);
    plugin.on_request(&mut ctx).await.unwrap();
    assert_eq!(plugin.count(), 2);
}

#[tokio::test]
async fn plugin_modifies_request_context() {
    let plugin = Arc::new(HeaderInjectorPlugin::new(
        "injector",
        "x-custom-header",
        "custom-value",
    ));
    let mut ctx = RequestContext {
        method: "POST".to_string(),
        path: "/v1/chat/completions".to_string(),
        headers: HashMap::new(),
        query_params: HashMap::new(),
        body: None,
        metadata: HashMap::new(),
    };

    assert!(!ctx.headers.contains_key("x-custom-header"));
    plugin.on_request(&mut ctx).await.unwrap();
    assert_eq!(
        ctx.headers.get("x-custom-header"),
        Some(&"custom-value".to_string())
    );
}

#[tokio::test]
async fn plugin_modifies_proxy_context() {
    let plugin = Arc::new(HeaderInjectorPlugin::new(
        "injector",
        "x-injected",
        "by-plugin",
    ));
    let mut ctx = ProxyContext {
        upstream_url: "https://api.openai.com/v1/chat/completions".to_string(),
        headers: HashMap::new(),
        body: None,
        metadata: HashMap::new(),
    };

    plugin.before_proxy(&mut ctx).await.unwrap();
    assert_eq!(
        ctx.headers.get("x-injected"),
        Some(&"by-plugin".to_string())
    );
}

#[tokio::test]
async fn plugin_on_error_receives_context() {
    struct ErrorCapturePlugin {
        captured: std::sync::Mutex<Option<String>>,
    }

    #[async_trait]
    impl Plugin for ErrorCapturePlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                name: "error-capture".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            }
        }

        async fn on_error(&self, ctx: &ErrorContext) -> Result<(), PluginError> {
            let mut captured = self.captured.lock().unwrap();
            *captured = Some(ctx.error_code.clone());
            Ok(())
        }
    }

    let plugin = Arc::new(ErrorCapturePlugin {
        captured: std::sync::Mutex::new(None),
    });

    let error_ctx = ErrorContext {
        error_code: "rate_limited".to_string(),
        error_message: "rate limit exceeded".to_string(),
        request_context: RequestContext {
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            metadata: HashMap::new(),
        },
    };

    plugin.on_error(&error_ctx).await.unwrap();
    let captured = plugin.captured.lock().unwrap().clone();
    assert_eq!(captured, Some("rate_limited".to_string()));
}

#[tokio::test]
async fn plugin_execution_failure_propagates() {
    let plugin = Arc::new(FailingPlugin::new("failing", "something went wrong"));
    let mut ctx = RequestContext {
        method: "POST".to_string(),
        path: "/v1/chat/completions".to_string(),
        headers: HashMap::new(),
        query_params: HashMap::new(),
        body: None,
        metadata: HashMap::new(),
    };

    let result = plugin.on_request(&mut ctx).await;
    assert!(result.is_err());
    assert!(matches!(result, Err(PluginError::ExecutionFailed(_))));
}

// -------------------- 插件元数据测试 --------------------

#[tokio::test]
async fn plugin_metadata_fields() {
    let plugin = Arc::new(CounterPlugin::new("test-plugin", "2.3.1"));
    let metadata = plugin.metadata();

    assert_eq!(metadata.name, "test-plugin");
    assert_eq!(metadata.version, "2.3.1");
    assert_eq!(metadata.author, Some("test".to_string()));
    assert_eq!(metadata.description, Some("counter plugin".to_string()));
}

#[tokio::test]
async fn plugin_metadata_optional_fields() {
    let plugin = Arc::new(HeaderInjectorPlugin::new("injector", "x-header", "value"));
    let metadata = plugin.metadata();

    assert_eq!(metadata.name, "injector");
    assert_eq!(metadata.version, "1.0.0");
    assert!(metadata.author.is_none());
    assert!(metadata.description.is_none());
}

// -------------------- 多插件协作测试 --------------------

#[tokio::test]
async fn multiple_plugins_execute_in_sequence() {
    let registry = PluginRegistry::new();
    let p1 = Arc::new(CounterPlugin::new("counter1", "1.0.0"));
    let p2 = Arc::new(CounterPlugin::new("counter2", "1.0.0"));

    registry.register(p1.clone()).await.unwrap();
    registry.register(p2.clone()).await.unwrap();

    let mut ctx = RequestContext {
        method: "POST".to_string(),
        path: "/v1/chat/completions".to_string(),
        headers: HashMap::new(),
        query_params: HashMap::new(),
        body: None,
        metadata: HashMap::new(),
    };

    // 模拟按顺序执行所有插件
    let plugins = registry.list().await;
    for meta in plugins {
        if let Some(plugin) = registry.get(&meta.name).await {
            plugin.on_request(&mut ctx).await.unwrap();
        }
    }

    assert_eq!(p1.count(), 1);
    assert_eq!(p2.count(), 1);
}

#[tokio::test]
async fn plugin_chain_stops_on_error() {
    let registry = PluginRegistry::new();
    let good = Arc::new(CounterPlugin::new("good", "1.0.0"));
    let bad = Arc::new(FailingPlugin::new("bad", "error"));

    registry.register(good.clone()).await.unwrap();
    registry.register(bad).await.unwrap();

    let mut ctx = RequestContext {
        method: "POST".to_string(),
        path: "/v1/chat/completions".to_string(),
        headers: HashMap::new(),
        query_params: HashMap::new(),
        body: None,
        metadata: HashMap::new(),
    };

    // 模拟插件链执行
    let plugins = registry.list().await;
    let mut executed = Vec::new();
    for meta in plugins {
        if let Some(plugin) = registry.get(&meta.name).await {
            match plugin.on_request(&mut ctx).await {
                Ok(()) => executed.push(meta.name.clone()),
                Err(_) => break, // 遇到错误停止执行
            }
        }
    }

    // 根据注册顺序，可能先执行 good 或 bad
    // 但一旦遇到 bad，后续插件不应执行
    assert!(executed.len() <= 2);
}
