use serde::Deserialize;
use serde::Serialize;
use sqlx::types::chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateFolder {
    pub workspace_id: Uuid,         // workspace id
    pub name: String,               // folder name
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateFolder {
    pub id: Uuid,                   // folder id
    pub name: String,               // folder name
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FolderOptionalId {
    pub id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub id: Uuid,                   // folder id
    pub user_id: String,            // user id
    pub workspace_id: Uuid,         // workspace id
    pub name: String,               // folder name
    pub parent_videos: i64,         // parent videos
    pub generated_videos: i64,      // generated videos
    pub created_at: NaiveDateTime,  // create date
    pub updated_at: NaiveDateTime,  // update date
}
