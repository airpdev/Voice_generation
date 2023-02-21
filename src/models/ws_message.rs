use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WsMessage {
    pub user_id: String,
    pub message_type: String,
    pub message: String,
}

impl From<String> for WsMessage {
    fn from(json: String) -> Self {
        serde_json::from_str(&json).unwrap()
    }
}
