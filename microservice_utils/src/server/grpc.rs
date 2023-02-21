use anyhow::*;
use uuid::Uuid;
use tonic::transport::Endpoint;

pub mod auth_service {
    tonic::include_proto!("auth_service");
}

pub mod user_service {
    tonic::include_proto!("user_service");
}

pub mod workspace_service {
    tonic::include_proto!("workspace_service");
}

pub mod ai_studio_service {
    tonic::include_proto!("ai_studio_service");
}

pub mod file_manager_service {
    tonic::include_proto!("file_manager_service");
}

use user_service::{user_service_client::UserServiceClient, 
    CreateUserRequest, CheckUserRequest, AddWorkspaceRequest, RemoveWorkspaceRequest, 
    UserQuotaRequest, UserQuotaResponse, UserQuotaNotBinded, UserQuotaUpdate,
    StripeUsageRecordReuqest,
};
use workspace_service::{
    workspace_service_client::WorkspaceServiceClient, 
    WorkspaceCreateRequest, WorkspaceCreateResponse,
    WorkspaceInfo, UpdateWorkspaceQuotaByUser, UpdateWorkspaceQuota,
    WorkspaceQuotaRequest, WorkspaceQuotaResponse,
};
use auth_service::{
    auth_service_client::AuthServiceClient, 
    CheckTokenRequest, TokenRefreshRequest, CheckShopifyToken, CheckHubspotToken,
    AuthProviderRequest, AuthProviderResponse, KlaviyoAuthRequest
};
use ai_studio_service::{
    ai_studio_service_client::AiStudioServiceClient,
    FolderCreateRequest, FolderCreateResponse,
    VideoInstanceCreateRequest, VideoInstanceCreateResponse,
    CreateActorRequest, CreateActorResponse,
    GeneratedUsageRequest, GeneratedUsageResponse,
};
use file_manager_service::{
    file_manager_service_client::FileManagerServiceClient,
    GetVideoRequest, GetVideoResponse,
};

use crate::USER_SERVICE_URL;
use crate::AUTH_SERVICE_URL;
use crate::WORKSPACE_SERVICE_URL;
use crate::AISTUDIO_URL;
use crate::FILE_SERVICE_URL;

pub async fn pre_create_user(
    user_id: &String, email: &String
) -> Result<()> {
    lazy_static::initialize(&USER_SERVICE_URL);
    let endpoint: Endpoint = USER_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = UserServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with user service")?;
    let res = grpc
        .pre_create_user(CreateUserRequest {
            user_id: user_id.to_string(),
            email: email.to_string(),
        })
        .await
        .context("Unable to send pre_create_user request")?;

    tracing::info!("{:?}", res);

    Ok(())
}

pub async fn add_workspace_id(
    user_id: &String, workspace_id: &Uuid
) -> Result<()> {
    lazy_static::initialize(&USER_SERVICE_URL);
    let endpoint: Endpoint = USER_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = UserServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with user service")?;
    let res = grpc
        .add_workspace_id(AddWorkspaceRequest {
            user_id: user_id.to_string(),
            workspace_id: workspace_id.to_string(),
        })
        .await
        .context("Unable to send add_workspace_id request")?;

    tracing::info!("{:?}", res);

    Ok(())
}

pub async fn remove_workspace_id(
    user_id: &String, workspace_id: &Uuid
) -> Result<()> {
    lazy_static::initialize(&USER_SERVICE_URL);
    let endpoint: Endpoint = USER_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = UserServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with user service")?;
    let res = grpc
        .remove_workspace_id(RemoveWorkspaceRequest {
            user_id: user_id.to_string(),
            workspace_id: workspace_id.to_string(),
        })
        .await
        .context("Unable to send remove_workspace_id request")?;

    tracing::info!("{:?}", res);

    Ok(())
}

pub async fn check_user(
    user_id: &String
) -> Result<bool> {
    lazy_static::initialize(&USER_SERVICE_URL);
    let endpoint: Endpoint = USER_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = UserServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with user service")?;
    let res = grpc
        .check_user(CheckUserRequest {
            user_id: user_id.to_string(),
        })
        .await
        .context("Unable to send check_user request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(message.user_exists)
}

pub async fn get_user_quota(
    user_id: &String
) -> Result<UserQuotaResponse> {
    lazy_static::initialize(&USER_SERVICE_URL);
    let endpoint: Endpoint = USER_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = UserServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with user service")?;
    let res = grpc
        .get_user_quota(UserQuotaRequest {
            user_id: user_id.to_string(),
        })
        .await
        .context("Unable to send get_user_quota request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(message)
}

pub async fn set_not_binded_videos(
    user_id: &String, not_binded_videos: i64
) -> Result<()> {
    lazy_static::initialize(&USER_SERVICE_URL);
    let endpoint: Endpoint = USER_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = UserServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with user service")?;
    let res = grpc
        .set_not_binded_videos(UserQuotaNotBinded {
            user_id: user_id.to_string(),
            not_binded_videos: not_binded_videos,
        })
        .await
        .context("Unable to send set_not_binded_videos request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(())
}

pub async fn user_increase_videos_used(
    user_id: &String, generated_videos_used: i64
) -> Result<()> {
    lazy_static::initialize(&USER_SERVICE_URL);
    let endpoint: Endpoint = USER_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = UserServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with user service")?;
    let res = grpc
        .increase_videos_used(UserQuotaUpdate {
            user_id: user_id.to_string(),
            generated_videos_used: generated_videos_used,
        })
        .await
        .context("Unable to send user_increase_videos_used request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(())
}

pub async fn user_decrease_videos_used(
    user_id: &String, generated_videos_used: i64
) -> Result<()> {
    lazy_static::initialize(&USER_SERVICE_URL);
    let endpoint: Endpoint = USER_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = UserServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with user service")?;
    let res = grpc
        .decrease_videos_used(UserQuotaUpdate {
            user_id: user_id.to_string(),
            generated_videos_used: generated_videos_used,
        })
        .await
        .context("Unable to send user_decrease_videos_used request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(())
}

pub async fn stripe_usage_records(
    user_id: &String, number_of_succ_vid: i64
) -> Result<()> {
    lazy_static::initialize(&USER_SERVICE_URL);
    let endpoint: Endpoint = USER_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = UserServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with user service")?;
    let res = grpc
        .stripe_usage_records(StripeUsageRecordReuqest {
            user_id: user_id.to_string(),
            number_of_succ_vid: number_of_succ_vid,
        })
        .await
        .context("Unable to send stripe_usage_records request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(())
}

pub async fn get_auth_provider(
    user_id: &String
) -> Result<AuthProviderResponse, Error> {
    lazy_static::initialize(&AUTH_SERVICE_URL);
    let endpoint: Endpoint = AUTH_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = AuthServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with auth service")?;
    let res = grpc
        .get_auth_provider(AuthProviderRequest {
            user_id: user_id.to_string(),
        })
        .await
        .context("Unable to send get_auth_provider request")?;

    let message = res.into_inner();
    
    tracing::info!("{:?}", message);

    Ok(message)
}

pub async fn check_token(
    user_id: &String, access_token: &String
) -> Result<String, Error> {
    lazy_static::initialize(&AUTH_SERVICE_URL);
    let endpoint: Endpoint = AUTH_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = AuthServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with auth service")?;
    let res = grpc
        .check_token(CheckTokenRequest {
            user_id: user_id.to_string(),
            access_token: access_token.to_string(),
        })
        .await
        .context("Unable to send check_token request")?;

    let message = res.into_inner();
    if message.status == "success" {
        tracing::info!("{:?}", message);
        Ok(message.role)
    } else {
        Err(Error::msg("Authentication failed"))
    }
}

pub async fn refresh_token(
    user_id: &String, refresh_token: &String
) -> Result<(), Error> {
    lazy_static::initialize(&AUTH_SERVICE_URL);
    let endpoint: Endpoint = AUTH_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = AuthServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with auth service")?;
    let res = grpc
        .refresh_token(TokenRefreshRequest {
            user_id: user_id.to_string(),
            refresh_token: refresh_token.to_string(),
        })
        .await
        .context("Unable to send refresh_token request")?;
   
    let message = res.into_inner();
    if message.status == "success" {
        tracing::info!("{:?}", message);
        Ok(())
    } else {
        Err(Error::msg("Authentication failed"))
    }
}

pub async fn get_shopify_token(
    user_id: &String
) -> Result<String, Error> {
    lazy_static::initialize(&AUTH_SERVICE_URL);
    let endpoint: Endpoint = AUTH_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = AuthServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with auth service")?;
    let res = grpc
        .get_shopify_token(CheckShopifyToken {
            user_id: user_id.to_string(),
        })
        .await
        .context("Unable to send get_shopify_token request")?;

    let message = res.into_inner();
    if message.status == "success" {
        tracing::info!("{:?}", message);
        Ok(message.token)
    } else {
        Err(Error::msg("Authentication failed"))
    }
}

pub async fn get_hubspot_token(
    user_id: &String
) -> Result<String, Error> {
    lazy_static::initialize(&AUTH_SERVICE_URL);
    let endpoint: Endpoint = AUTH_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = AuthServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with auth service")?;
    let res = grpc
        .get_hubspot_token(CheckHubspotToken {
            user_id: user_id.to_string(),
        })
        .await
        .context("Unable to send get_hubspot_token request")?;

    let message = res.into_inner();
    if message.status == "success" {
        tracing::info!("{:?}", message);
        Ok(message.access_token)
    } else {
        Err(Error::msg("Authentication failed"))
    }
}

pub async fn get_klaviyo_key(
    user_id: &String
) -> Result<String, Error> {
    lazy_static::initialize(&AUTH_SERVICE_URL);
    let endpoint: Endpoint = AUTH_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = AuthServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with auth service")?;
    let res = grpc
        .get_klaviyo_auth(KlaviyoAuthRequest {
            user_id: user_id.to_string(),
        })
        .await
        .context("Unable to send get_klaviyo_key request")?;

    let message = res.into_inner();
    if !message.private_key.is_empty() {
        tracing::info!("{:?}", message);
        Ok(message.private_key)
    } else {
        Err(Error::msg("No private key found"))
    }
}

pub async fn create_workspace(
    user_id: &String, name: &String, parent_videos_quota: i64, generated_videos_quota: i64
) -> Result<WorkspaceCreateResponse> {
    lazy_static::initialize(&WORKSPACE_SERVICE_URL);
    let endpoint: Endpoint = WORKSPACE_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = WorkspaceServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with workspace service")?;
    let res = grpc
        .create_workspace(WorkspaceCreateRequest {
            user_id: user_id.to_string(),
            name: name.to_string(),
            role: "owner".to_string(),
            description: "My Workspace".to_string(),
            parent_videos_quota: parent_videos_quota,
            generated_videos_quota: generated_videos_quota,
        })
        .await
        .context("Unable to send create_workspace request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(message)
}

pub async fn check_workspace(
    user_id: &String, workspace_id: &Uuid
) -> Result<(), Error> {
    lazy_static::initialize(&WORKSPACE_SERVICE_URL);
    let endpoint: Endpoint = WORKSPACE_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = WorkspaceServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with workspace service")?;
    let res = grpc
        .check_workspace(WorkspaceInfo {
            user_id: user_id.to_string(),
            workspace_id: workspace_id.to_string(),
        })
        .await
        .context("Unable to send check_workspace request")?;

    let message = res.into_inner();
    if message.status == "success" {
        tracing::info!("{:?}", message);
        Ok(())
    } else {
        Err(Error::msg("Workspace does not exist"))
    }
}

pub async fn update_workspace_quota_by_user(
    user_id: &String, generated_videos_quota: i64, generated_videos_used: i64
) -> Result<(), Error> {
    lazy_static::initialize(&WORKSPACE_SERVICE_URL);
    let endpoint: Endpoint = WORKSPACE_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = WorkspaceServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with workspace service")?;
    let res = grpc
        .update_workspace_quota_by_user(UpdateWorkspaceQuotaByUser {
            user_id: user_id.to_string(),
            generated_videos_quota: generated_videos_quota,
            generated_videos_used: generated_videos_used,
        })
        .await
        .context("Unable to send update_workspace_quota_by_user request")?;

    let message = res.into_inner();
    if message.status == "success" {
        tracing::info!("{:?}", message);
        Ok(())
    } else {
        Err(Error::msg("Failed update workspace quota"))
    }
}

pub async fn increase_workspace_quota_by_user(
    user_id: &String, generated_videos_quota: i64,
) -> Result<(), Error> {
    lazy_static::initialize(&WORKSPACE_SERVICE_URL);
    let endpoint: Endpoint = WORKSPACE_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = WorkspaceServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with workspace service")?;
    let res = grpc
        .increase_workspace_quota_by_user(UpdateWorkspaceQuotaByUser {
            user_id: user_id.to_string(),
            generated_videos_quota: generated_videos_quota,
            generated_videos_used: 0,
        })
        .await
        .context("Unable to send increase_workspace_quota_by_user request")?;

    let message = res.into_inner();
    if message.status == "success" {
        tracing::info!("{:?}", message);
        Ok(())
    } else {
        Err(Error::msg("Failed update workspace quota"))
    }
}

pub async fn ws_increase_videos_used(
    workspace_id: &Uuid, generated_videos_used: i64
) -> Result<()> {
    lazy_static::initialize(&WORKSPACE_SERVICE_URL);
    let endpoint: Endpoint = WORKSPACE_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = WorkspaceServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with workspace service")?;
    let res = grpc
        .increase_videos_used(UpdateWorkspaceQuota {
            workspace_id: workspace_id.to_string(),
            generated_videos_used: generated_videos_used,
        })
        .await
        .context("Unable to send ws_increase_videos_used request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(())
}

pub async fn ws_decrease_videos_used(
    workspace_id: &Uuid, generated_videos_used: i64
) -> Result<()> {
    lazy_static::initialize(&WORKSPACE_SERVICE_URL);
    let endpoint: Endpoint = WORKSPACE_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = WorkspaceServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with workspace service")?;
    let res = grpc
        .decrease_videos_used(UpdateWorkspaceQuota {
            workspace_id: workspace_id.to_string(),
            generated_videos_used: generated_videos_used,
        })
        .await
        .context("Unable to send ws_decrease_videos_used request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(())
}

pub async fn get_workspace_quota(
    workspace_id: &Uuid
) -> Result<WorkspaceQuotaResponse> {
    lazy_static::initialize(&WORKSPACE_SERVICE_URL);
    let endpoint: Endpoint = WORKSPACE_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = WorkspaceServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with workspace service")?;
    let res = grpc
        .get_workspace_quota(WorkspaceQuotaRequest {
            workspace_id: workspace_id.to_string(),
        })
        .await
        .context("Unable to send get_workspace_quota request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(message)
}

pub async fn create_folder(
    user_id: &String, workspace_id: &Uuid, name: &String
) -> Result<FolderCreateResponse> {
    lazy_static::initialize(&AISTUDIO_URL);
    let endpoint: Endpoint = AISTUDIO_URL.parse().context("Invalid endpoint")?;
    let mut grpc = AiStudioServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with ai-studio service")?;
    let res = grpc
        .create_folder(FolderCreateRequest {
            user_id: user_id.to_string(),
            workspace_id: workspace_id.to_string(),
            name: name.to_string(),
        })
        .await
        .context("Unable to send create_folder request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(message)
}

pub async fn create_v_instance(
    user_id: &String, folder_id: &Uuid, actor_id: &Uuid, name: &String
) -> Result<VideoInstanceCreateResponse> {
    lazy_static::initialize(&AISTUDIO_URL);
    let endpoint: Endpoint = AISTUDIO_URL.parse().context("Invalid endpoint")?;
    let mut grpc = AiStudioServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with ai-studio service")?;
    let res = grpc
        .create_v_instance(VideoInstanceCreateRequest {
            user_id: user_id.to_string(),
            folder_id: folder_id.to_string(),
            actor_id: actor_id.to_string(),
            name: name.to_string(),
        })
        .await
        .context("Unable to send create_v_instance request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(message)
}

pub async fn create_actor(
    user_id: &String, name: &String
) -> Result<CreateActorResponse> {
    lazy_static::initialize(&AISTUDIO_URL);
    let endpoint: Endpoint = AISTUDIO_URL.parse().context("Invalid endpoint")?;
    let mut grpc = AiStudioServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with ai-studio service")?;
    let res = grpc
        .create_actor(CreateActorRequest {
            user_id: user_id.to_string(),
            name: name.to_string(),
        })
        .await
        .context("Unable to send create_actor request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(message)
}

pub async fn generated_usage(
    from: &String,
    to: &String,
) -> Result<GeneratedUsageResponse, Error> {
    lazy_static::initialize(&AISTUDIO_URL);
    let endpoint: Endpoint = AISTUDIO_URL.parse().context("Invalid endpoint")?;
    let mut grpc = AiStudioServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with ai-studio service")?;
    let res = grpc
        .generated_usage(GeneratedUsageRequest {
            from: from.to_string(),
            to: to.to_string(),
        })
        .await
        .context("Unable to send generated_usage request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(message)
}

pub async fn get_video(
    user_id: &String,
    video_id: &Uuid,
) -> Result<GetVideoResponse, Error> {
    lazy_static::initialize(&FILE_SERVICE_URL);
    let endpoint: Endpoint = FILE_SERVICE_URL.parse().context("Invalid endpoint")?;
    let mut grpc = FileManagerServiceClient::connect(endpoint)
        .await
        .context("Unable to establish connection with filemanager service")?;
    let res = grpc
        .get_video(GetVideoRequest {
            user_id: user_id.to_string(),
            video_id: video_id.to_string(),
        })
        .await
        .context("Unable to send get_video request")?;

    let message = res.into_inner();

    tracing::info!("{:?}", message);

    Ok(message)
}
