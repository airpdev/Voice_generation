use sqlx::types::chrono::NaiveDateTime;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioInfo {
    pub id: Uuid,
    pub file_path: String,
    pub voice_code: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageError{
    pub error: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UploadParam {
    pub user_name: String,
}

#[derive(Copy, Clone)]
pub struct Silence {
    pub start_index: usize,
    pub start_time: f64,
    pub end_index: usize,
    pub end_time: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchUploadParam {
    pub template_bucket: String,
    pub template_key: String,
    pub transcripts: Vec<String>,
}
