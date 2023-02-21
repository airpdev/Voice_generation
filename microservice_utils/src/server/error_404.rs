use axum::{body::Body, http::Response};
use axum_macros::debug_handler;


use crate::server::not_found::NotFoundError;

#[debug_handler]
// Handler 404 - Not Found
pub async fn error_404() -> Response<Body> {
    NotFoundError::new("Not Found").into_response()
}