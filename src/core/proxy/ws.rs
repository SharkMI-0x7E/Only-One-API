//! WebSocket 转发（spec §4.4 · [S3]）
//!
//! **关键约束**：只支持 OpenAI Realtime API 风格的 WebSocket 协议。
//! 双向透传消息，零缓冲；连接生命周期用 tracing 记录。
//!
//! **为什么 ws.rs 是 core 层的例外**：WebSocket 升级握手强绑定 axum 的
//! `WebSocketUpgrade` 提取器，无法抽象到 service 层而不引入大量胶水代码。
//! 这里允许直接依赖 axum/tokio，但仅限于此文件。

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use futures_util::{SinkExt, StreamExt};
use tracing::{error, info, warn};

use crate::core::auth::Authenticator;
use crate::core::error::CoreError;

/// WebSocket 转发器：管理客户端到上游的双向消息流
pub struct WsForwarder {
    upstream_url: String,
}

impl WsForwarder {
    /// 创建转发器实例
    ///
    /// `upstream_url` 是上游 WebSocket 端点（如 `wss://api.openai.com/v1/realtime`）
    pub fn new(upstream_url: String) -> Self {
        Self { upstream_url }
    }

    /// 处理 WebSocket 升级请求
    ///
    /// **为什么在 upgrade 时鉴权**：WebSocket 握手后无法返回 HTTP 状态码，
    /// 必须在升级前完成认证，失败时直接返回 401。
    pub async fn handle(
        &self,
        ws: WebSocketUpgrade,
        auth: Arc<dyn Authenticator>,
        token: String,
    ) -> Result<impl axum::response::IntoResponse, CoreError> {
        // 鉴权：失败时返回 CoreError::Auth，由 ServiceError 映射为 401
        auth.verify(&token).await?;

        let upstream_url = self.upstream_url.clone();

        // 升级到 WebSocket 后进入双向转发
        Ok(ws.on_upgrade(move |socket| Self::run_forwarder(socket, upstream_url)))
    }

    /// 执行双向消息转发
    ///
    /// **为什么用 tokio::select**：客户端和上游可能同时关闭连接，
    /// 用 select 监听两个方向，任一结束则整体结束，避免半开连接。
    async fn run_forwarder(client_socket: WebSocket, upstream_url: String) {
        let (mut client_sink, mut client_stream) = client_socket.split();

        // 连接上游 WebSocket
        let (upstream_socket, _) = match tokio_tungstenite::connect_async(&upstream_url).await {
            Ok(conn) => conn,
            Err(e) => {
                error!(error = %e, "failed to connect upstream");
                return;
            }
        };

        let (mut upstream_sink, mut upstream_stream) = upstream_socket.split();

        info!("websocket connection established");

        // 客户端 -> 上游
        let client_to_upstream = async {
            while let Some(msg) = client_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        // axum Utf8Bytes -> tungstenite Utf8Bytes
                        let tungstenite_msg =
                            tokio_tungstenite::tungstenite::Message::Text(text.as_str().into());
                        if upstream_sink.send(tungstenite_msg).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Binary(data)) => {
                        let tungstenite_msg = tokio_tungstenite::tungstenite::Message::Binary(data);
                        if upstream_sink.send(tungstenite_msg).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        // 透传 ping，保持连接活性
                        let tungstenite_msg = tokio_tungstenite::tungstenite::Message::Ping(data);
                        if upstream_sink.send(tungstenite_msg).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Pong(_)) => {
                        // 忽略 pong（由底层自动处理）
                    }
                    Ok(Message::Close(frame)) => {
                        info!("client closed connection");
                        // 转发关闭帧给上游
                        if let Some(f) = frame {
                            let close_frame = tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(
                                    f.code,
                                ),
                                reason: f.reason.as_str().into(),
                            };
                            let tungstenite_msg =
                                tokio_tungstenite::tungstenite::Message::Close(Some(close_frame));
                            let _ = upstream_sink.send(tungstenite_msg).await;
                        }
                        break;
                    }
                    Err(e) => {
                        warn!(error = %e, "client stream error");
                        break;
                    }
                }
            }
        };

        // 上游 -> 客户端
        let upstream_to_client = async {
            while let Some(msg) = upstream_stream.next().await {
                match msg {
                    Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                        // tungstenite Utf8Bytes -> axum Utf8Bytes
                        if client_sink
                            .send(Message::Text(text.as_str().into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(tokio_tungstenite::tungstenite::Message::Binary(data)) => {
                        if client_sink.send(Message::Binary(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(tokio_tungstenite::tungstenite::Message::Ping(data)) => {
                        if client_sink.send(Message::Ping(data)).await.is_err() {
                            break;
                        }
                    }
                    Ok(tokio_tungstenite::tungstenite::Message::Pong(_)) => {}
                    Ok(tokio_tungstenite::tungstenite::Message::Close(frame)) => {
                        info!("upstream closed connection");
                        // 转发关闭帧给客户端
                        if let Some(f) = frame {
                            let close_frame = axum::extract::ws::CloseFrame {
                                code: f.code.into(),
                                reason: f.reason.as_str().into(),
                            };
                            let _ = client_sink.send(Message::Close(Some(close_frame))).await;
                        }
                        break;
                    }
                    Ok(tokio_tungstenite::tungstenite::Message::Frame(_)) => {
                        // 原始帧不处理（由底层自动处理）
                    }
                    Err(e) => {
                        warn!(error = %e, "upstream stream error");
                        break;
                    }
                }
            }
        };

        // 并发运行两个方向，任一结束则整体结束
        tokio::select! {
            _ = client_to_upstream => {},
            _ = upstream_to_client => {},
        }

        info!("websocket forwarder exited");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarder_constructs() {
        let fwd = WsForwarder::new("wss://example.com/ws".to_string());
        assert_eq!(fwd.upstream_url, "wss://example.com/ws");
    }

    #[tokio::test]
    async fn auth_failure_returns_error() {
        use crate::core::auth::apikey::ApiKeyAuthenticator;

        let auth = Arc::new(ApiKeyAuthenticator::new());
        auth.register("sk-test-123".to_string());

        // 验证错误凭据会返回错误
        let result = auth.verify("sk-wrong").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn auth_success_returns_ok() {
        use crate::core::auth::apikey::ApiKeyAuthenticator;

        let auth = Arc::new(ApiKeyAuthenticator::new());
        auth.register("sk-test-123".to_string());

        // 验证正确凭据会成功
        let result = auth.verify("sk-test-123").await;
        assert!(result.is_ok());
    }
}
