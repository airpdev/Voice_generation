use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequiredId {
    pub id: Uuid,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptionalId {
    pub id: Option<Uuid>,
}