use anyhow::Result;
use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::Response,
};
use bytes::Bytes;
use futures::{stream, TryStreamExt};

use crate::db::models::App;

/// Build the upstream URL for a request.
///
/// - `app.upstream_base_path` is prepended to the forwarded path
/// - For http_path apps, `slug_prefix` is the matched prefix (e.g. "/myapp");
///   it is stripped from `req_path` when `app.strip_prefix` is true.
pub fn build_upstream_url(app: &App, req_path: &str, query: Option<&str>, _slug_prefix: &str) -> Result<String> {
    let path = req_path;

    let base = app.base_url.trim_end_matches('/');
    let upstream_base = app.upstream_base_path.trim_end_matches('/');
    let path = path.trim_start_matches('/');

    let full_path = if upstream_base.is_empty() || upstream_base == "/" {
        format!("{}/{}", base, path)
    } else {
        format!("{}{}/{}", base, upstream_base, path)
    };

    match query {
        Some(q) if !q.is_empty() => Ok(format!("{}?{}", full_path, q)),
        _ => Ok(full_path),
    }
}

/// Forward a request to the upstream and return the streamed response.
pub async fn forward(
    client: &reqwest::Client,
    method: Method,
    upstream_url: String,
    headers: HeaderMap,
    body: Bytes,
    iam_token: &str,
) -> Result<Response<Body>> {
    // Build reqwest request
    let mut req = client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes())?,
        &upstream_url,
    );

    // Forward all incoming headers, then inject IAM token (overrides any existing)
    for (name, value) in &headers {
        // skip hop-by-hop headers
        if is_hop_by_hop(name.as_str()) {
            continue;
        }
        req = req.header(name.as_str(), value.as_bytes());
    }
    req = req.header("X-Auth-Token", iam_token);

    if !body.is_empty() {
        req = req.body(body);
    }

    let upstream_resp = req.send().await?;

    // Map status
    let status = StatusCode::from_u16(upstream_resp.status().as_u16())?;

    // Map response headers
    let mut resp_headers = HeaderMap::new();
    let is_sse = upstream_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/event-stream"))
        .unwrap_or(false);
    for (name, value) in upstream_resp.headers() {
        if is_hop_by_hop(name.as_str()) {
            continue;
        }
        if let (Ok(n), Ok(v)) = (
            HeaderName::from_bytes(name.as_str().as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            resp_headers.insert(n, v);
        }
    }
    if is_sse {
        resp_headers.insert("cache-control", HeaderValue::from_static("no-cache"));
        resp_headers.insert("x-accel-buffering", HeaderValue::from_static("no"));
        resp_headers.remove("content-length");
    }

    // Stream body back.
    // For SSE (and any streaming response), use chunk() to yield each chunk
    // as soon as it arrives rather than waiting for reqwest's read buffer to fill.
    let body = if is_sse {
        let body = stream::unfold(upstream_resp, |mut resp| async move {
            match resp.chunk().await {
                Ok(Some(chunk)) => Some((Ok::<Bytes, std::io::Error>(chunk), resp)),
                _ => None,
            }
        });
        Body::from_stream(body)
    } else {
        let stream = upstream_resp
            .bytes_stream()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
        Body::from_stream(stream)
    };

    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = resp_headers;

    Ok(response)
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
