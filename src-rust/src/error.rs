use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

pub enum ServeError {
    NotFound,
    Forbidden,
    BadRequest(String),
    MethodNotAllowed,
    InternalError(String),
}

impl IntoResponse for ServeError {
    fn into_response(self) -> Response {
        if let ServeError::MethodNotAllowed = self {
            let body = "405 Method Not Allowed";
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                [
                    (header::ALLOW, "GET, HEAD"),
                    (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
                    (header::CACHE_CONTROL, "no-store"),
                ],
                body,
            )
                .into_response();
        }

        let (status, message): (StatusCode, &str) = match self {
            ServeError::NotFound => (StatusCode::NOT_FOUND, "Not Found"),
            ServeError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            ServeError::BadRequest(ref msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            ServeError::InternalError(ref msg) => {
                tracing::error!("{}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
            }
            ServeError::MethodNotAllowed => unreachable!(),
        };

        let body = format!("{} {}", status.as_u16(), message);
        (
            status,
            [
                ("content-type", "text/plain; charset=utf-8".to_string()),
                ("cache-control", "no-store".to_string()),
            ],
            body,
        )
            .into_response()
    }
}
