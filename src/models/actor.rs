use serde::Deserialize;
use serde::Serialize;
use sqlx::types::chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateActor {
    pub name: String,               // actor name
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateActor {
    pub id: Uuid,                   // actor id
    pub name: String,               // actor name
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Actor {
    pub id: Uuid,                   // folder id
    pub user_id: String,            // user id
    pub name: String,               // actor name
    pub created_at: NaiveDateTime,  // create date
    pub updated_at: NaiveDateTime,  // update date
}
