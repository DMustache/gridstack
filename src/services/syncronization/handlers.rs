use axum::{
    Extension, Json,
    extract::{Path, State},
};
use response_derive::IntoResponseEnum;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::services::{
    authorization::persistence::access_session_storage_unit::AccessSessionStorageUnit,
    shared::MatrixErrorResponse,
    state::ApplicationState,
    syncronization::errors::SyncronizationApplicationError,
};

#[derive(Clone, Debug, Deserialize)]
pub struct DefineFilterInfo {
    #[serde(flatten)]
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct DefineFilterView {
    pub filter_id: String,
}

#[derive(IntoResponseEnum)]
pub enum DefineFilterResponse {
    #[matrix(status = 200)]
    Ok(Json<DefineFilterView>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = SyncronizationApplicationError::InvalidParameter)])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 401, error = [(matrix_error = "M_UNKNOWN_TOKEN", from = SyncronizationApplicationError::Unauthorized)])]
    Unauthorized(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = SyncronizationApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = SyncronizationApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

#[derive(IntoResponseEnum)]
pub enum GetFilterResponse {
    #[matrix(status = 200)]
    Ok(Json<serde_json::Value>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = SyncronizationApplicationError::InvalidParameter)])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 401, error = [(matrix_error = "M_UNKNOWN_TOKEN", from = SyncronizationApplicationError::Unauthorized)])]
    Unauthorized(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = SyncronizationApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 404, error = [(matrix_error = "M_NOT_FOUND", from = SyncronizationApplicationError::NotFound)])]
    NotFound(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = SyncronizationApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

pub async fn define_filter(
    Path(user_id): Path<String>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(request): Json<DefineFilterInfo>,
) -> DefineFilterResponse {
    match application_state
        .syncronization_service
        .define_filter(&access_session, &user_id, request.payload)
    {
        Ok(view) => {
            info!(user_id, filter_id = %view.filter_id, "filter created");
            DefineFilterResponse::Ok(Json(view))
        }
        Err(error_kind) => {
            if matches!(error_kind, SyncronizationApplicationError::Internal) {
                error!("define filter failed with internal error");
            } else {
                info!(error = %error_kind, "define filter rejected");
            }
            DefineFilterResponse::from_mapped_error(error_kind)
        }
    }
}

pub async fn get_filter(
    Path((user_id, filter_id)): Path<(String, String)>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> GetFilterResponse {
    match application_state
        .syncronization_service
        .get_filter(&access_session, &user_id, &filter_id)
    {
        Ok(view) => GetFilterResponse::Ok(Json(view)),
        Err(error_kind) => {
            if matches!(error_kind, SyncronizationApplicationError::Internal) {
                error!("get filter failed with internal error");
            } else {
                info!(error = %error_kind, "get filter rejected");
            }
            GetFilterResponse::from_mapped_error(error_kind)
        }
    }
}
