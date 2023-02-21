use serde::{Deserialize, Serialize};

type AmpResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AmpInfo {
    UserInfo {
        fullname: String,
        email: String,
    },
    QuotaInfo {
        used_quota: i64,
    },
    PlanInfo {
        plan_id: String,
        plan_name: String,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AmpEvent {
    pub user_id: String,
    pub event_type: String,
    pub user_properties: AmpInfo,
}

pub async fn amp_event(event: &AmpEvent) -> AmpResult<()> {
    let params = serde_json::json!({
        "api_key": "f9505daa9542929df0bdb6c99472aecd",
        "events": serde_json::json!(&event),
    });
    let client = reqwest::Client::new();
    client
        .post("https://api.amplitude.com/2/httpapi")
        .json(&params)
        .send()
        .await?;
    Ok(())
}