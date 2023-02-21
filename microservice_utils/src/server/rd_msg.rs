use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::fmt;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RdMessage {
    pub user_id: String,
    pub msg_type: RdType,
    pub sub_type: RdSubType,
    pub message: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RdType {
    #[serde(rename = "notify")]
    Notify,
    #[serde(rename = "workspace")]
    Workspace,
    #[serde(rename = "post")]
    Post,
    #[serde(rename = "like")]
    Like,
    #[serde(rename = "comment")]
    Comment,
    #[serde(rename = "credit")]
    Credit,
    #[serde(rename = "video_gen")]
    VideoGen,
    #[serde(rename = "voice_gen")]
    VoiceGen,
    #[serde(rename = "plan")]
    Plan,
    #[serde(rename = "quota")]
    Quota,
    DefaultType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RdSubType {
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "get")]
    Get,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "add")]
    Add,
    #[serde(rename = "remove")]
    Remove,
    #[serde(rename = "list")]
    List,
    #[serde(rename = "error")]
    Error,
    DefaultSubType,
}

impl Default for RdType {
    fn default() -> Self {
        Self::DefaultType
    }
}

impl fmt::Display for RdType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for RdSubType {
    fn default() -> Self {
        Self::DefaultSubType
    }
}

impl fmt::Display for RdSubType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}