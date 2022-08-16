use axum::response::IntoResponse;

pub fn into_reponse(code: i64, body: serde_json::Value) -> impl IntoResponse {
    let value = serde_json::json!({
        "code": code,
        "result": body,
    });
    axum::Json(value)
}