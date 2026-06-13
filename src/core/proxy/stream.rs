//! 流式响应透传（spec §4.4）
//!
//! **关键约束**：拿到上游 `Response` 后**直接透传 body 流**，禁止 `.bytes().await?` 一次性缓冲。

use axum::body::Body;
use bytes::Bytes;
use futures_util::StreamExt;
use http::Response;

/// 把 `reqwest::Response` 转为 axum `Response<Body>`，body 透传不缓冲
pub fn into_axum_response(resp: reqwest::Response) -> Response<Body> {
    let status = resp.status();
    let headers = resp.headers().clone();
    let stream = resp.bytes_stream();

    // reqwest::Error -> io::Error（BoxError 兼容）
    let body_stream =
        stream.map(|result: Result<Bytes, reqwest::Error>| result.map_err(std::io::Error::other));

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
