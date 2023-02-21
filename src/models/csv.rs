use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CsvRequiredId {
    pub video_instance_id: Uuid,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioBatchId {
    pub audio_batch_id: Uuid,
}