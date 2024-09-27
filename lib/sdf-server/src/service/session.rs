use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use dal::{
    KeyPairError, StandardModelError, TransactionsError, UserError, UserPk, WorkspaceError,
    WorkspacePk,
};
use serde::{Deserialize, Serialize};
use telemetry::prelude::*;
use thiserror::Error;

use crate::AppState;

use super::ApiError;

pub mod auth_connect;
pub mod load_workspaces;
mod refresh_workspace_members;
pub mod restore_authentication;

#[remain::sorted]
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("auth api error: {0}")]
    AuthApiError(String),
    #[error(transparent)]
    ContextTransactions(#[from] TransactionsError),
    #[error("Invalid user: {0}")]
    InvalidUser(UserPk),
    #[error("Invalid workspace: {0}")]
    InvalidWorkspace(WorkspacePk),
    #[error("json serialize failed")]
    JSONSerialize(#[from] serde_json::Error),
    #[error(transparent)]
    KeyPair(#[from] KeyPairError),
    #[error("login failed")]
    LoginFailed,
    #[error(transparent)]
    Nats(#[from] si_data_nats::NatsError),
    #[error(transparent)]
    Pg(#[from] si_data_pg::PgError),
    #[error("http error: {0}")]
    Request(#[from] reqwest::Error),
    #[error(transparent)]
    StandardModel(#[from] StandardModelError),
    #[error("user error: {0}")]
    User(#[from] UserError),
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
    #[error("workspace {0} not yet migrated to new snapshot graph version. Migration required")]
    WorkspaceNotYetMigrated(WorkspacePk),
    #[error("you do not have permission to create a workspace on this instance")]
    WorkspacePermissions,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthApiErrBody {
    pub kind: String,
    pub message: String,
}

pub type SessionResult<T> = std::result::Result<T, SessionError>;

impl IntoResponse for SessionError {
    fn into_response(self) -> Response {
        let (status_code, error_message) = match self {
            SessionError::LoginFailed => (StatusCode::CONFLICT, self.to_string()),
            SessionError::InvalidWorkspace(_) => (StatusCode::CONFLICT, self.to_string()),
            SessionError::WorkspacePermissions => (StatusCode::UNAUTHORIZED, self.to_string()),
            SessionError::AuthApiError(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        ApiError::new(status_code, error_message).into_response()
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/connect", post(auth_connect::auth_connect))
        .route("/reconnect", get(auth_connect::auth_reconnect))
        .route(
            "/restore_authentication",
            get(restore_authentication::restore_authentication),
        )
        .route("/load_workspaces", get(load_workspaces::load_workspaces))
        .route(
            "/refresh_workspace_members",
            post(refresh_workspace_members::refresh_workspace_members),
        )
}
