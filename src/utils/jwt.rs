use anyhow::Error;
use axum::{
    extract::TypedHeader,
    headers::{authorization::Bearer, Authorization},
};
use jsonwebtoken::{decode, errors::ErrorKind, DecodingKey, Validation};
use serde::Deserialize;
use serde::Serialize;

use crate::utils::grpc::check_token;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub company: String,
    pub exp: usize,
}

pub async fn jwt_auth(
    TypedHeader(cookies): TypedHeader<Authorization<Bearer>>,
) -> Result<String, Error> {
    let token = cookies.0.token();
    let validation = Validation::default();
    let token_data = decode::<Claims>(&token, &DecodingKey::from_secret(b"secret"), &validation)
        .map_err(|e| match *e.kind() {
            ErrorKind::InvalidToken => anyhow::anyhow!("Token is invalid"),
            ErrorKind::InvalidIssuer => anyhow::anyhow!("Issuer is invalid"),
            _ => anyhow::anyhow!("Some other errors"),
        })?;

    let user_id: &str = &token_data.claims.sub[..];
    if user_id.is_empty() {
        Err(Error::msg("User id is empty".to_string()))
    } else {
        let res = check_token(&user_id.to_string(), &token.to_string()).await;
        match res {
            Ok(_) => Ok(user_id.to_string()),
            Err(e) => Err(e),
        }
    }
}
