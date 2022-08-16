use anyhow::*;
use uuid::Uuid;
use tonic::transport::Endpoint;

pub mod auth_service {
    tonic::include_proto!("auth_service");
}

use auth_service::{auth_service_client::AuthServiceClient, CheckTokenRequest, TokenRefreshRequest};

pub async fn check_token(user_id: &String, access_token: &String) -> Result<(), Error> {
    let endpoint: Endpoint = "https://test.bhuman.ai/grpc/auth".parse().context("Invalid endpoint")?;
    let mut grpc = AuthServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection")?;
    let res = grpc
        .check_token(CheckTokenRequest {
            user_id: user_id.to_string(),
            access_token: access_token.to_string(),
        })
        .await
        .context("Unable to send echo request")?;

    let message = res.into_inner();
    if message.status == "success" {
        println!("{:?}", message);
        Ok(())
    } else {
        Err(Error::msg("Authentication failed"))
    }
}

pub async fn refresh_token(user_id: &String, refresh_token: &Uuid) -> Result<(), Error> {
    let endpoint: Endpoint = "https://test.bhuman.ai/grpc/auth".parse().context("Invalid endpoint")?;
    let mut grpc = AuthServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection")?;
    let res = grpc
        .refresh_token(TokenRefreshRequest {
            user_id: user_id.to_string(),
            refresh_token: refresh_token.to_string(),
        })
        .await
        .context("Unable to send echo request")?;
   
    let message = res.into_inner();
    if message.status == "success" {
        println!("{:?}", message);
        Ok(())
    } else {
        Err(Error::msg("Authentication failed"))
    }
}
