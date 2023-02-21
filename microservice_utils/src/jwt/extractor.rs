use std::sync::Arc;

use sqlx::{PgPool, Row};

use axum::{
    async_trait,
    extract::{Query, RequestParts},
    Extension, TypedHeader,
};
use headers::{authorization::Bearer, Authorization};
use okapi::openapi3::{SecurityRequirement, SecurityScheme, SecuritySchemeData};
use openapi_rs::{
    gen::OpenApiGenerator,
    request::{OpenApiFromRequest, RequestHeaderInput},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::auth::jwt_auth;

#[derive(Default, Deserialize, Serialize, Clone)]
pub struct ZapierAuth {
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Serialize, Default, Deserialize, JsonSchema)]
pub struct AuthToken(pub String, pub String);

#[async_trait]
impl<T> axum::extract::FromRequest<T> for AuthToken
where
    T: Send,
{
    type Rejection = String;

    async fn from_request(req: &mut RequestParts<T>) -> Result<Self, Self::Rejection> {
        let query: Query<ZapierAuth> =
            Query::<ZapierAuth>::from_request(req).await.map_err(|e| {
                let ret = serde_json::json!({
                    "code": 404,
                    "body": format!("{:?}", e),
                });
                ret.to_string()
            })?;

        match (&query.client_id, &query.client_secret) {
            (Some(client_id), Some(client_secret)) => {
                let pool: Extension<Arc<PgPool>> = Extension::from_request(req)
                    .await.map_err(|e| {
                        let ret = serde_json::json!({
                            "code": 500,
                            "body": format!("BUG: ApiContext was not added as an extension {:?}", e),
                        });
                        ret.to_string()
                    })?;

                let user = sqlx::query(&format!(
                    "SELECT * FROM generated_keys WHERE client_id = '{}' AND client_secret = '{}'",
                    client_id, client_secret
                ))
                .fetch_one(&**pool)
                .await
                .map_err(|e| {
                    let ret = serde_json::json!({
                        "code": 404,
                        "body": format!("{:?}", e),
                    });
                    ret.to_string()
                })?;

                let user_id = user.try_get::<String, &str>("user_id").map_err(|e| {
                    let ret = serde_json::json!({
                        "code": 404,
                        "body": format!("{:?}", e),
                    });
                    ret.to_string()
                })?;
                let role = user.try_get::<String, &str>("role").map_err(|e| {
                    let ret = serde_json::json!({
                        "code": 404,
                        "body": format!("{:?}", e),
                    });
                    ret.to_string()
                })?;

                Ok(AuthToken(user_id, role))
            }
            _ => {
                let cookies = TypedHeader::<Authorization<Bearer>>::from_request(req)
                    .await
                    .map_err(|e| {
                        let ret = serde_json::json!({
                            "code": 404,
                            "body": format!("{:?}", e),
                        });
                        ret.to_string()
                    })?;
                jwt_auth(cookies)
                    .await
                    .map_err(|e| {
                        let ret = serde_json::json!({
                            "code": 404,
                            "body": format!("{:?}", e),
                        });
                        ret.to_string()
                    })
                    .map(|(user_id, role)| AuthToken(user_id, role))
            }
        }
    }
}

impl<T> OpenApiFromRequest<T> for AuthToken
where
    T: Send,
{
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> anyhow::Result<RequestHeaderInput> {
        // Setup global requirement for Security scheme
        let security_scheme = SecurityScheme {
            description: Some("Requires an Access Token".to_owned()),
            // Setup data requirements.
            data: SecuritySchemeData::Http {
                // Other flows are very similar.
                // For more info see: https://swagger.io/docs/specification/authentication/oauth2/
                scheme: "bearer".into(),
                bearer_format: Some("JWT".into()), // bearer_format:Some("JWT".into()),
            },
            // Add example data for RapiDoc
            extensions: okapi::map! {},
        };
        // Add the requirement for this route/endpoint
        // This can change between routes.
        let mut security_req = SecurityRequirement::new();

        security_req.insert("Bearer".to_owned(), Vec::new());

        // Each security requirement needs to be met before access is allowed.
        // These vvvvvvv-----^^^^^^^^^^ values need to match exactly!
        Ok(RequestHeaderInput::Security(
            "Bearer".to_owned(),
            security_scheme,
            security_req,
        ))
    }
}
