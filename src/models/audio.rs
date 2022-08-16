use serde::Deserialize;
use serde::Serialize;
use sqlx::types::chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Audio {
    pub id: Uuid,
    pub user_id: String,
    pub actor_id: Uuid,
    pub name: String,
    pub url: String,
    pub audio_length: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioBatch {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}