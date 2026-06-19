//! 插件系统性能基准测试
//!
//! 测试阶段三新增的插件系统性能：
//! - PluginRegistry 注册/注销操作
//! - PluginRegistry 查询操作
//! - PluginRegistry 列表操作
//! - 插件生命周期钩子调用性能

use async_trait::async_trait;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rapidgate::core::plugins::PluginRegistry;
use rapidgate::core::plugins::{Plugin, PluginError, PluginMetadata, RequestContext};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// 测试用插件实现
#[derive(Debug)]
struct TestPlugin {
    name: String,
    version: String,
}

impl TestPlugin {
    fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
        }
    }
}

#[async_trait]
impl Plugin for TestPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: self.name.clone(),
            version: self.version.clone(),
            author: Some("Test Author".to_string()),
            description: Some("Test plugin for benchmarking".to_string()),
        }
    }

    async fn on_request(&self, _ctx: &mut RequestContext) -> Result<(), PluginError> {
        // 模拟插件逻辑
        Ok(())
    }
}

/// 创建测试用的请求上下文
fn make_test_request_context() -> RequestContext {
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    headers.insert("authorization".to_string(), "Bearer test-token".to_string());

    let mut query_params = HashMap::new();
    query_params.insert("model".to_string(), "gpt-4".to_string());

    RequestContext {
        method: "POST".to_string(),
        path: "/v1/chat/completions".to_string(),
        headers,
        query_params,
        body: Some(b"{\"messages\":[]}".to_vec()),
        metadata: HashMap::new(),
    }
}

/// 测试插件注册性能
fn bench_plugin_register(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("plugin_register", |b| {
        b.iter(|| {
            rt.block_on(async {
                let registry = PluginRegistry::new();
                for i in 0..10 {
                    let plugin = Arc::new(TestPlugin::new(&format!("plugin-{}", i), "1.0.0"));
                    let _ = registry.register(plugin).await;
                }
                black_box(registry.count().await)
            })
        })
    });
}

/// 测试插件查询性能
fn bench_plugin_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // 预注册 100 个插件
    let registry = rt.block_on(async {
        let registry = PluginRegistry::new();
        for i in 0..100 {
            let plugin = Arc::new(TestPlugin::new(&format!("plugin-{}", i), "1.0.0"));
            let _ = registry.register(plugin).await;
        }
        registry
    });

    c.bench_function("plugin_get", |b| {
        b.iter(|| rt.block_on(async { black_box(registry.get("plugin-50").await) }))
    });
}

/// 测试插件列表性能
fn bench_plugin_list(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // 预注册 50 个插件
    let registry = rt.block_on(async {
        let registry = PluginRegistry::new();
        for i in 0..50 {
            let plugin = Arc::new(TestPlugin::new(&format!("plugin-{}", i), "1.0.0"));
            let _ = registry.register(plugin).await;
        }
        registry
    });

    c.bench_function("plugin_list", |b| {
        b.iter(|| rt.block_on(async { black_box(registry.list().await) }))
    });
}

/// 测试插件注销性能
fn bench_plugin_unregister(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("plugin_unregister", |b| {
        b.iter(|| {
            rt.block_on(async {
                let registry = PluginRegistry::new();
                // 注册 10 个插件
                for i in 0..10 {
                    let plugin = Arc::new(TestPlugin::new(&format!("plugin-{}", i), "1.0.0"));
                    let _ = registry.register(plugin).await;
                }
                // 注销所有插件
                for i in 0..10 {
                    let _ = registry.unregister(&format!("plugin-{}", i)).await;
                }
                black_box(registry.count().await)
            })
        })
    });
}

/// 测试插件 on_request 钩子调用性能
fn bench_plugin_on_request(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let plugin = Arc::new(TestPlugin::new("test-plugin", "1.0.0"));
    let mut ctx = make_test_request_context();

    c.bench_function("plugin_on_request", |b| {
        b.iter(|| rt.block_on(async { black_box(plugin.on_request(&mut ctx).await) }))
    });
}

/// 测试插件 metadata 调用性能
fn bench_plugin_metadata(c: &mut Criterion) {
    let plugin = TestPlugin::new("test-plugin", "1.0.0");

    c.bench_function("plugin_metadata", |b| {
        b.iter(|| black_box(plugin.metadata()))
    });
}

/// 测试高并发场景下的插件查询性能
fn bench_plugin_concurrent_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // 预注册 100 个插件
    let registry = Arc::new(rt.block_on(async {
        let registry = PluginRegistry::new();
        for i in 0..100 {
            let plugin = Arc::new(TestPlugin::new(&format!("plugin-{}", i), "1.0.0"));
            let _ = registry.register(plugin).await;
        }
        registry
    }));

    c.bench_function("plugin_concurrent_get", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = vec![];
                // 并发查询 10 次
                for _ in 0..10 {
                    let registry_clone = registry.clone();
                    let handle = tokio::spawn(async move { registry_clone.get("plugin-50").await });
                    handles.push(handle);
                }
                // 等待所有查询完成
                for handle in handles {
                    let _ = handle.await;
                }
            })
        })
    });
}

criterion_group!(
    benches,
    bench_plugin_register,
    bench_plugin_get,
    bench_plugin_list,
    bench_plugin_unregister,
    bench_plugin_on_request,
    bench_plugin_metadata,
    bench_plugin_concurrent_get,
);
criterion_main!(benches);
