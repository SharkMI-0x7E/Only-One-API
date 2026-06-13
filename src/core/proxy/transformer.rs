//! 请求/响应 Header 与路径转换（spec §4.4）
//!
//! 阶段一仅占位签名；阶段三 [S3] 在 providers/* 落地具体协议差异。

use http::{HeaderMap, Method};

/// 转换客户端请求到上游请求
pub struct TransformedRequest {
    pub method: Method,
    pub path: String,
    pub headers: HeaderMap,
}

pub fn transform_request(method: &Method, path: &str, headers: &HeaderMap) -> TransformedRequest {
    TransformedRequest {
        method: method.clone(),
        path: path.to_string(),
        headers: headers.clone(),
    }
}

/// 提取上游响应头里需要透传的子集
pub fn transform_response_headers(headers: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (k, v) in headers.iter() {
        // 阶段一：透传除 hop-by-hop 之外的所有 header
        if !is_hop_by_hop(k.as_str()) {
            out.insert(k.clone(), v.clone());
        }
    }
    out
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;

    #[test]
    fn strips_hop_by_hop() {
        let mut h = HeaderMap::new();
        h.insert(
            "content-type",
            HeaderValue::from_static("text/event-stream"),
        );
        h.insert("connection", HeaderValue::from_static("close"));
        let out = transform_response_headers(&h);
        assert!(out.contains_key("content-type"));
        assert!(!out.contains_key("connection"));
    }
}
