//! Token 计数器 — 从 SSE chunk 增量累加（spec §4.8）

use serde::Deserialize;

/// 从 SSE chunk 中提取 token 用量
#[derive(Debug, Default)]
pub struct TokenCounter {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Deserialize)]
struct UsageBlock {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
}

#[derive(Deserialize)]
struct SseData {
    #[serde(default)]
    usage: Option<UsageBlock>,
}

impl TokenCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从一个 SSE chunk 累加 token 用量
    pub fn accumulate(&mut self, chunk: &str) {
        for line in chunk.lines() {
            let data = line.strip_prefix("data: ").unwrap_or(line);
            if data == "[DONE]" {
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<SseData>(data) {
                if let Some(usage) = parsed.usage {
                    self.prompt_tokens = self.prompt_tokens.saturating_add(usage.prompt_tokens);
                    self.completion_tokens = self
                        .completion_tokens
                        .saturating_add(usage.completion_tokens);
                }
            }
        }
    }

    pub fn total(&self) -> u64 {
        self.prompt_tokens.saturating_add(self.completion_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulate_from_sse_chunk() {
        let mut counter = TokenCounter::new();
        let chunk = "data: {\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":20}}\n\n";
        counter.accumulate(chunk);
        assert_eq!(counter.prompt_tokens, 10);
        assert_eq!(counter.completion_tokens, 20);
    }

    #[test]
    fn accumulate_multiple_chunks() {
        let mut counter = TokenCounter::new();
        counter.accumulate("data: {\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":10}}\n\n");
        counter.accumulate("data: {\"usage\":{\"prompt_tokens\":3,\"completion_tokens\":7}}\n\n");
        assert_eq!(counter.prompt_tokens, 8);
        assert_eq!(counter.completion_tokens, 17);
    }

    #[test]
    fn skip_done_marker() {
        let mut counter = TokenCounter::new();
        counter.accumulate("data: [DONE]\n\n");
        assert_eq!(counter.total(), 0);
    }
}
