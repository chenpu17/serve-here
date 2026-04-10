use std::path::Path;
use std::time::SystemTime;

use axum::body::Body;
use axum::extract::State;
use axum::extract::Request;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use tokio_util::io::ReaderStream;

use crate::error::ServeError;
use crate::listing;
use crate::server::AppState;
use crate::stats;

const INDEX_FILES: &[&str] = &["index.html", "index.htm"];

pub async fn handle_stats_page(State(state): State<AppState>) -> Response {
    let html = stats::render_stats_page(
        state.metrics.root_dir(),
        &state.host,
        state.port,
        state.metrics.started_at_ms(),
        &state.dashboard_data_path,
    );

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8".to_string()),
            (header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        html,
    )
        .into_response()
}

pub async fn handle_stats_data(State(state): State<AppState>) -> Response {
    (
        [(header::CACHE_CONTROL, "no-store")],
        Json(state.metrics.snapshot()),
    )
        .into_response()
}

pub async fn handle_request(State(state): State<AppState>, req: Request) -> Response {
    let is_head = req.method() == Method::HEAD;
    handle_request_inner(
        req,
        state.root_dir.as_path(),
        state.dashboard_path.as_ref(),
        is_head,
    )
    .await
}

async fn handle_request_inner(
    req: Request,
    root_dir: &Path,
    dashboard_path: &str,
    is_head: bool,
) -> Response {
    let uri = req.uri().clone();

    let (pathname, search) = {
        let path = uri.path();
        let query = uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
        (path.to_string(), query)
    };

    // Only allow GET and HEAD
    let method = req.method().clone();
    if method != Method::GET && method != Method::HEAD {
        return ServeError::MethodNotAllowed.into_response();
    }

    // Decode the URL path
    let decoded_path = match urlencoding::decode(&pathname) {
        Ok(p) => p.into_owned(),
        Err(_) => return ServeError::BadRequest("Bad Request".to_string()).into_response(),
    };

    // Build candidate path
    let candidate_segments: Vec<&str> = decoded_path
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();

    let resolved_path = if candidate_segments.is_empty() {
        root_dir.to_path_buf()
    } else {
        let mut base = root_dir.to_path_buf();
        for seg in &candidate_segments {
            base.push(seg);
        }
        match base.canonicalize() {
            Ok(p) => p,
            Err(_) => return ServeError::NotFound.into_response(),
        }
    };

    // Path traversal protection
    if !resolved_path.starts_with(root_dir) {
        return ServeError::Forbidden.into_response();
    }

    // Stat the resolved path
    let metadata = match tokio::fs::metadata(&resolved_path).await {
        Ok(m) => m,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return ServeError::NotFound.into_response();
            }
            return ServeError::InternalError(format!("Error reading path: {}", e)).into_response();
        }
    };

    if metadata.is_dir() {
        // Directory redirect if no trailing slash
        if !pathname.ends_with('/') {
            let location = format!("{}{}{}", pathname, "/", search);
            return (
                StatusCode::MOVED_PERMANENTLY,
                [(header::LOCATION, location)],
                Body::empty(),
            )
                .into_response();
        }

        // Try index files
        for index_file in INDEX_FILES {
            let candidate = resolved_path.join(index_file);
            if let Ok(index_meta) = tokio::fs::metadata(&candidate).await {
                if index_meta.is_file() {
                    return serve_file(&candidate, &index_meta, is_head).await;
                }
            }
        }

        // Directory listing
        match listing::generate_listing_html(&decoded_path, &resolved_path, dashboard_path).await {
            Ok(html) => {
                let content_length = html.len().to_string();
                let body = if is_head { Body::empty() } else { Body::from(html) };
                return (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, "text/html; charset=utf-8".to_string()),
                        (header::CACHE_CONTROL, "no-cache".to_string()),
                        (header::CONTENT_LENGTH, content_length),
                    ],
                    body,
                )
                    .into_response();
            }
            Err(e) => {
                return ServeError::InternalError(format!("Error generating directory listing: {}", e))
                    .into_response();
            }
        }
    }

    if metadata.is_file() {
        return serve_file(&resolved_path, &metadata, is_head).await;
    }

    ServeError::Forbidden.into_response()
}

async fn serve_file(path: &Path, metadata: &std::fs::Metadata, is_head: bool) -> Response {
    let mime_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    let mtime: chrono::DateTime<chrono::Utc> = metadata
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .into();

    let headers = [
        (header::CONTENT_TYPE, HeaderValue::from_str(&mime_type).unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"))),
        (header::CONTENT_LENGTH, HeaderValue::from(metadata.len())),
        (header::LAST_MODIFIED, HeaderValue::from_str(&mtime.format("%a, %d %b %Y %H:%M:%S GMT").to_string()).unwrap_or_else(|_| HeaderValue::from_static(""))),
        (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
    ];

    if is_head {
        return (StatusCode::OK, headers, Body::empty()).into_response();
    }

    match tokio::fs::File::open(path).await {
        Ok(file) => {
            let stream = ReaderStream::new(file);
            let body = Body::from_stream(stream);
            (StatusCode::OK, headers, body).into_response()
        }
        Err(e) => {
            ServeError::InternalError(format!("Error opening file: {}", e)).into_response()
        }
    }
}
