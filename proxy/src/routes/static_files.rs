use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    response::Response,
};
use std::path::{Path, PathBuf};

/// Serve static files from the `./static` directory.
/// Falls back to `index.html` for any path that doesn't match a file
/// (supports Vue client-side routing).
pub async fn handler(req: Request<Body>) -> Response {
    let uri_path = req.uri().path();
    let static_dir = Path::new("./static");

    // Strip leading slash and sanitize
    let rel = uri_path.trim_start_matches('/');
    let file_path = static_dir.join(rel);

    if let Some(resp) = serve_file(&file_path).await {
        return resp;
    }

    // Fallback: serve index.html (SPA routing)
    if let Some(resp) = serve_file(&static_dir.join("index.html")).await {
        return resp;
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not found"))
        .unwrap()
}

async fn serve_file(path: &PathBuf) -> Option<Response> {
    let content = tokio::fs::read(path).await.ok()?;
    let mime = mime_type(path);
    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .body(Body::from(content))
            .unwrap(),
    )
}

fn mime_type(path: &PathBuf) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js")   => "application/javascript",
        Some("css")  => "text/css",
        Some("svg")  => "image/svg+xml",
        Some("png")  => "image/png",
        Some("ico")  => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    }
}
