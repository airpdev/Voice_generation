use axum::{body::Body, response::Response};

pub struct NotFoundError {
    pub message: String,
}

impl NotFoundError {
    pub fn new(error_message: &str) -> Self {
        Self {
            message: error_message.to_string(),
        }
    }

    pub fn into_response(self) -> Response<Body> {
        Response::builder()
            .status(404)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&self.message).unwrap()))
            .unwrap()
    }
}