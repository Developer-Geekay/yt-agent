use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

// Define our custom error type
pub enum AppError {
    Internal(anyhow::Error),
    YtDlp(String),
    BadRequest(String),
    NotFound(String),
}

// This implementation allows us to convert our AppError into a valid HTTP response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Internal(e) => {
                // Log the full error for debugging
                tracing::error!("Internal server error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "An internal server error occurred".to_string(),
                )
            }
            AppError::YtDlp(e) => (StatusCode::BAD_REQUEST, format!("yt-dlp error: {}", e)),
            AppError::BadRequest(e) => (StatusCode::BAD_REQUEST, e),
            AppError::NotFound(e) => (StatusCode::NOT_FOUND, e),
        };

        let body = Json(json!({ "error": error_message }));
        (status, body).into_response()
    }
}

// This allows us to use the `?` operator to automatically convert
// any error that implements `std::error::Error` into our `AppError::Internal`.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Internal(err.into())
    }
}
