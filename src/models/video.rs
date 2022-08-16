use serde::Deserialize;
use serde::Serialize;
use sqlx::types::chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateVideoInstance {
    pub folder_id: Uuid,            // folder id
    pub name: String,               // instance name
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateVideoinstance {
    pub id: Uuid,
    pub name: Option<String>,
    pub video_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub audio_batch_id: Option<Uuid>,
    #[serde(default)]
    pub image_column_id: Option<i64>,
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoInstance {
    pub id: Uuid,
    pub name: String,
    pub user_id: String,    
    pub folder_id: Uuid,
    pub video_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub audio_batch_id: Option<Uuid>,
    pub image_column_id: Option<i64>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Video {
    pub id: Uuid,
    pub user_id: String,
    pub name: String,
    pub url: String,
    pub length: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedVideo{
    pub id: Uuid,
    pub batch_id: Uuid,
    pub audio_lables: Vec<String>,
    pub name: String,
    pub user_id: String,
    pub video_instance_id: Uuid,
    pub video_url: Option<String>,
    pub vimeo_url: Option<String>,
    pub thumbnail: Option<String>,
    pub status: String,
    pub vimeo_status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}