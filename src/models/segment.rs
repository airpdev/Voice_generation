use serde::Deserialize;
use serde::Serialize;
use sqlx::types::chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateSegment {
    pub video_instance_id: Uuid,
    pub prefix_time_marker_start: String,
    pub prefix_time_marker_end: String,
    pub suffix_time_marker_start: String,
    pub suffix_time_marker_end: String,
    pub audio_variable_column_id: i64,
    pub audio_variable_name: String,
    pub variable_time_marker_start: String,
    pub variable_time_marker_end: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SegmentOptionalId {
    pub id: Option<Uuid>,
    pub video_instance_id: Option<Uuid>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateSegment {
    pub id: Uuid,
    pub audio_variable_name: String,    
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub id: Uuid,
    pub user_id: String,
    pub video_instance_id: Uuid,
    pub prefix_time_marker_start: String,
    pub prefix_time_marker_end: String,
    pub suffix_time_marker_start: String,
    pub suffix_time_marker_end: String,
    pub audio_variable_column_id: i64,
    pub audio_variable_name: String,
    pub variable_time_marker_start: String,
    pub variable_time_marker_end: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
