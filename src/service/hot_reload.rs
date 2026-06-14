//! 配置热重载 — ArcSwap + 校验失败回滚（spec §5.5）
//!
//! 监听配置文件变更 → 加载新配置 → 校验 → 失败保留旧配置 → 成功原子替换。

use std::path::PathBuf;
use std::sync::Arc;

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use crate::core::routing::RouteTable;
use crate::service::config_loader;
use crate::service::state::AppState;

pub struct HotReloader {
    _watcher: RecommendedWatcher,
    _rx: mpsc::UnboundedReceiver<()>,
}

impl HotReloader {
    /// 启动配置文件监听，返回 HotReloader 持有以维持 watcher 生命周期
    pub fn start(config_dir: PathBuf, _state: Arc<AppState>) -> Result<Self, notify::Error> {
        let (tx, rx) = mpsc::unbounded_channel();
        let config_dir_clone = config_dir.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if event.kind.is_modify() || event.kind.is_create() {
                        let _ = tx.send(());
                    }
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(&config_dir_clone, RecursiveMode::Recursive)?;

        tracing::info!(dir = %config_dir.display(), "hot reloader started");

        Ok(Self {
            _watcher: watcher,
            _rx: rx,
        })
    }
}

/// 重新加载配置并原子替换路由表
pub async fn reload(state: &Arc<AppState>) {
    match config_loader::load().await {
        Ok(cfg) => match RouteTable::new(cfg.routes.clone()) {
            Ok(table) => {
                state.route_table.replace(table);
                tracing::info!("config reloaded successfully");
            }
            Err(e) => {
                tracing::error!(error = %e, "route table compile failed, keeping old config");
            }
        },
        Err(e) => {
            tracing::error!(error = %e, "config reload failed, keeping old config");
        }
    }
}
