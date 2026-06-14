//! 流式响应透传（spec §4.4）
//!
//! **关键约束**：拿到上游 `Response` 后**直接透传 body 流**，禁止 `.bytes().await?` 一次性缓冲。
//! 阶段二增强：相邻小 chunk（< 1KB）合并为一次写入。

use axum::body::Body;
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;
use http::Response;

const BATCH_THRESHOLD: usize = 1024;

/// 把 `reqwest::Response` 转为 axum `Response<Body>`，body 透传 + 小 chunk 合并
pub fn into_axum_response(resp: reqwest::Response) -> Response<Body> {
    let status = resp.status();
    let headers = resp.headers().clone();
    let stream = resp.bytes_stream();

    let body_stream = stream
        .map(|result: Result<Bytes, reqwest::Error>| result.map_err(std::io::Error::other))
        .scan(
            BytesMut::with_capacity(BATCH_THRESHOLD),
            |buf, chunk_result| match chunk_result {
                Ok(chunk) => {
                    buf.extend_from_slice(&chunk);
                    if buf.len() >= BATCH_THRESHOLD {
                        let out = buf.split().freeze();
                        futures_util::future::ready(Some(Ok::<Bytes, std::io::Error>(out)))
                    } else {
                        futures_util::future::ready(None)
                    }
                }
                Err(e) => {
                    let out = if buf.is_empty() {
                        Err(e)
                    } else {
                        let flushed = buf.split().freeze();
                        Ok(flushed)
                    };
                    futures_util::future::ready(Some(out))
                }
            },
        );

    let mut response = Response::new(Body::from_stream(body_stream));
    *response.status_mut() = status;
    if let Some(ct) = headers.get(http::header::CONTENT_TYPE) {
        response
            .headers_mut()
            .insert(http::header::CONTENT_TYPE, ct.clone());
    }
    response
}

#[cfg(test)]
mod tests {
    #[test]
    fn body_empty_constructs() {
        let body = axum::body::Body::empty();
        let _ = body;
    }
}
