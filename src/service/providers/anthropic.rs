//! Anthropic Provider 实现
//!
//! 支持 Anthropic Messages API 格式，包括 Claude 系列模型。
//! 阶段三新增（spec §2 [S3]）。

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::core::config::ProviderKind;
use crate::core::error::CoreError;
use crate::service::providers::{Provider, ProviderRequest, ProviderResponse};

/// Anthropic Provider
pub struct AnthropicProvider;

#[async_trait]
impl Provider for AnthropicProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }

    fn transform_request(&self, req: &ProviderRequest) -> Result<Value, CoreError> {
        // 将 OpenAI 格式转换为 Anthropic Messages API 格式
        // OpenAI: { "model": "gpt-4", "messages": [...], "stream": true }
        // Anthropic: { "model": "claude-3", "messages": [...], "stream": true, "max_tokens": 4096 }

        let mut anthropic_req = req.body.clone();

        // 添加 max_tokens（Anthropic 必需）
        if anthropic_req.get("max_tokens").is_none() {
            anthropic_req["max_tokens"] = json!(4096);
        }

        // 转换 messages 格式（如果需要）
        // OpenAI 和 Anthropic 的 messages 格式基本兼容，但 Anthropic 不支持 system role
        if let Some(messages) = anthropic_req.get_mut("messages") {
            if let Some(messages_arr) = messages.as_array_mut() {
                for msg in messages_arr.iter_mut() {
                    if let Some(role) = msg.get("role") {
                        if role == "system" {
                            // Anthropic 不支持 system role，转换为 user
                            msg["role"] = json!("user");
                        }
                    }
                }
            }
        }

        Ok(anthropic_req)
    }

    fn transform_response(&self, resp: &ProviderResponse) -> Result<Value, CoreError> {
        // Anthropic 响应格式转换为 OpenAI 格式
        // Anthropic 流式：event: message_start / content_block_delta / message_stop
        // OpenAI 流式：data: { "choices": [{"delta": {...}}] }

        if resp.is_stream {
            // 流式响应需要解析 SSE events
            let body_str = String::from_utf8_lossy(&resp.body);
            let mut openai_chunks = Vec::new();

            for line in body_str.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(anthropic_chunk) = serde_json::from_str::<Value>(data) {
                        // 转换 Anthropic chunk 到 OpenAI chunk
                        if let Some(openai_chunk) = self.transform_streaming_chunk(&anthropic_chunk)
                        {
                            openai_chunks.push(openai_chunk);
                        }
                    }
                }
            }

            Ok(json!(openai_chunks))
        } else {
            // 非流式响应
            let anthropic_resp: Value = serde_json::from_slice(&resp.body).map_err(|e| {
                CoreError::Internal(format!("failed to parse Anthropic response: {e}"))
            })?;

            // 转换 Anthropic 响应到 OpenAI 格式
            // Anthropic: { "id": "...", "content": [{"type": "text", "text": "..."}], "usage": {...} }
            // OpenAI: { "id": "...", "choices": [{"message": {"content": "..."}}], "usage": {...} }

            let content = anthropic_resp
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|block| block.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("");

            let openai_resp = json!({
                "id": anthropic_resp.get("id").cloned().unwrap_or(json!("")),
                "object": "chat.completion",
                "created": anthropic_resp.get("created").cloned().unwrap_or(json!(0)),
                "model": anthropic_resp.get("model").cloned().unwrap_or(json!("")),
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": content
                    },
                    "finish_reason": "stop"
                }],
                "usage": anthropic_resp.get("usage").cloned().unwrap_or(json!({}))
            });

            Ok(openai_resp)
        }
    }

    fn api_path(&self) -> &str {
        "/v1/messages"
    }
}

impl AnthropicProvider {
    /// 转换 Anthropic 流式 chunk 到 OpenAI 格式
    fn transform_streaming_chunk(&self, chunk: &Value) -> Option<Value> {
        let event_type = chunk.get("type")?.as_str()?;

        match event_type {
            "message_start" => {
                // 消息开始
                Some(json!({
                    "id": chunk.get("message").and_then(|m| m.get("id")).cloned().unwrap_or(json!("")),
                    "object": "chat.completion.chunk",
                    "created": chunk.get("message").and_then(|m| m.get("created")).cloned().unwrap_or(json!(0)),
                    "model": chunk.get("message").and_then(|m| m.get("model")).cloned().unwrap_or(json!("")),
                    "choices": [{
                        "index": 0,
                        "delta": {
                            "role": "assistant"
                        },
                        "finish_reason": null
                    }]
                }))
            }
            "content_block_delta" => {
                // 内容增量
                let delta = chunk.get("delta")?;
                let text = delta.get("text")?.as_str()?;

                Some(json!({
                    "id": "",
                    "object": "chat.completion.chunk",
                    "created": 0,
                    "model": "",
                    "choices": [{
                        "index": 0,
                        "delta": {
                            "content": text
                        },
                        "finish_reason": null
                    }]
                }))
            }
            "message_stop" => {
                // 消息结束
                Some(json!({
                    "id": "",
                    "object": "chat.completion.chunk",
                    "created": 0,
                    "model": "",
                    "choices": [{
                        "index": 0,
                        "delta": {},
                        "finish_reason": "stop"
                    }]
                }))
            }
            _ => None,
        }
    }
}
