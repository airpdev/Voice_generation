use tokio::sync::mpsc;
use axum::extract::ws::Message;
use std::result::Result;

#[derive(Debug, Clone)]
pub struct Client {
    pub user_id: String,
    pub sender: Option<mpsc::UnboundedSender<Result<Message, axum::Error>>>,
}