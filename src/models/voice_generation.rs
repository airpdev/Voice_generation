use sqlx::types::chrono::NaiveDateTime;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub template_region: String,
    pub template_bucket: String,
    pub template_key: String,
    pub output_region: String,
    pub output_bucket: String,
    pub transcripts: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceCodeParam {
    pub audio_key: String,
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioTrashInfo {
    pub id: Uuid,
    pub file_path: String,
    pub voice_code: String,
    pub similarity: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioProcessInfo {
    pub template_region: String,
    pub template_bucket : String,
    pub template_key: String,
    pub audio_region: String,
    pub audio_bucket : String,
    pub audio_assets: Vec<String>,
    pub denoise: bool,
    pub amplitude_equalize: bool,
    pub silence_removal: bool,
    pub output_region : String,
    pub output_bucket : String
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioPauseInfo {
    pub audio_region: String,
    pub audio_bucket: String,
    pub audio_key: String
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HugginefaceInfo {
    pub target_path: String,
    pub reference_path_list: HashMap<String, String>,
    pub output_list: HashMap<String, String>
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProsodyInfo {
    pub target_path: String,
    pub target_transcript: String,
    pub reference_path: String,
    pub reference_transcript: String,
    pub output_path: String,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioSimilarityInfo {
    pub file_path: String,
    pub similarity: String
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MturkIdInfo {
    pub mturk_id: String
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MturkAudioInfo {
    pub id: Uuid,
    pub mturk_id: String,
    pub transcript: String,
    pub transcript_id: String,
    pub file_path: String,
    pub duration: String,
    pub status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}   

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MturkProcessInfo {
    pub mturk_id: String,
    pub transcript: String,
    pub transcript_id: String,
    pub file_path: String,
    pub duration: String,
    pub s3_bucket: String,
    pub s3_key: String
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MturkUploadInfo {
    pub mturk_id: String,
    pub transcript: String,
    pub transcript_id: String
}
#[derive(Serialize, Deserialize)]
pub struct Alignment {
    pub i: Vec<i64>,
    pub t: Vec<i64>,
    pub s: f64,
}

#[derive(Serialize, Deserialize)]
pub struct SimilarityInfo {
    pub transcript: String,
    pub whisper_transcript: String,
    pub alignments: Vec<Alignment>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProsodyUploadParam {
    pub actor_id: String,
    pub template_region: String,
    pub template_bucket: String,
    pub template_key: String,
    pub template_transcript: String,
    pub reference_region: String,
    pub reference_bucket: String,
    pub reference_key: String,
    pub reference_transcript: String,
    pub output_bucket: String
}

#[derive(Serialize, Deserialize)]
pub struct ExtractInfo {
    pub region: String,
    pub bucket: String,
    pub key: String
}

#[derive(Serialize, Deserialize)]
pub struct TranscriptCsvInfo {
    pub s3_links: String,
    pub names: String,
    pub created: String,
    pub done: String,
    pub modified: String
}
#[derive(Serialize, Deserialize)]
pub struct TranscriptInfo {
    pub transcript: String,
    pub start_time: i64,
    pub end_time: i64
}
#[derive(Serialize, Deserialize)]
pub struct MturkLoginInfo {
    pub mturk_id: String,
    pub password: String
}
#[derive(Serialize, Deserialize)]
pub struct MturkSignupInfo {
    pub mturk_id: String,
    pub password: String,
    pub paypal: String
}
#[derive(Serialize, Deserialize)]
pub struct MturkPaypalInfo {
    pub mturk_id: String,
    pub paypal: String
}

#[derive(Serialize, Deserialize)]
pub struct MturkPaymentInfo {
    pub mturk_id: String,
    pub payment_amount: i64
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MturkUserInfo {
    pub id: Uuid,
    pub mturk_id: String,
    pub password: String,
    pub paypal: Option<String>,
    pub total_payment: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MturkFullUserInfo {
    pub mturk_id: String,
    pub password: String,
    pub total_records: i64,
    pub paypal: Option<String>,
    pub total_payment: i64
}

#[derive(sqlx::FromRow)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioDetectPauseInfo {
    pub id: Uuid,
    pub s3_path: String,
    pub pauses: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LipsyncInfo {
    pub template_region: String,
    pub template_bucket : String,
    pub template_key: String,
    pub audio_region: String,
    pub audio_bucket : String,
    pub audio_assets: Vec<String>,
    pub output_region : String,
    pub output_bucket : String
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LipsyncInputInfo {
    pub model: String,
    pub video : String,
    pub audio: String,
    pub output: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PodcastTranscriptInfo {
    pub transcript: String,
    pub start_time : f64,
    pub end_time: f64
}