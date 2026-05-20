use axum::{
    Extension, Json,
    extract::{Path, Query, State, rejection::QueryRejection},
};
use response_derive::IntoResponseEnum;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::services::{
    authorization::persistence::access_session_storage_unit::AccessSessionStorageUnit,
    shared::MatrixErrorResponse,
    state::ApplicationState,
    synchronization::{
        entities::{SyncBatch, SyncFilterSelection, SyncRequest, SyncSetPresence},
        errors::SyncronizationApplicationError,
    },
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

#[derive(Clone, Debug, Deserialize, Default)]
pub struct SyncQueryInfo {
    pub filter: Option<String>,
    pub since: Option<String>,
    pub full_state: Option<bool>,
    pub set_presence: Option<SyncSetPresence>,
    pub timeout: Option<u64>,
    pub use_state_after: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SyncResponseView {
    pub next_batch: String,
    pub rooms: SyncRoomsResponseView,
    pub presence: SyncEventsResponseView,
    pub account_data: SyncEventsResponseView,
    pub to_device: SyncEventsResponseView,
    pub device_lists: SyncDeviceListsResponseView,
    pub device_one_time_keys_count: serde_json::Map<String, serde_json::Value>,
}

impl From<SyncBatch> for SyncResponseView {
    fn from(value: SyncBatch) -> Self {
        Self {
            next_batch: value.next_batch,
            rooms: SyncRoomsResponseView {
                join: value.rooms.join,
                invite: value.rooms.invite,
                leave: value.rooms.leave,
                knock: value.rooms.knock,
            },
            presence: SyncEventsResponseView {
                events: value.presence.events,
            },
            account_data: SyncEventsResponseView {
                events: value.account_data.events,
            },
            to_device: SyncEventsResponseView {
                events: value.to_device.events,
            },
            device_lists: SyncDeviceListsResponseView {
                changed: value.device_lists.changed,
                left: value.device_lists.left,
            },
            device_one_time_keys_count: value.device_one_time_keys_count,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SyncEventsResponseView {
    pub events: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SyncRoomsResponseView {
    pub join: serde_json::Map<String, serde_json::Value>,
    pub invite: serde_json::Map<String, serde_json::Value>,
    pub leave: serde_json::Map<String, serde_json::Value>,
    pub knock: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SyncDeviceListsResponseView {
    pub changed: Vec<String>,
    pub left: Vec<String>,
}

#[derive(IntoResponseEnum)]
#[allow(
    clippy::large_enum_variant,
    reason = "HTTP response payload carries the full sync body in success case"
)]
pub enum SyncResponse {
    #[matrix(status = 200)]
    Ok(Json<SyncResponseView>),
    #[matrix(status = 400, error = [(matrix_error = "M_INVALID_PARAM", from = SyncronizationApplicationError::InvalidParameter)])]
    BadRequest(Json<MatrixErrorResponse>),
    #[matrix(status = 401, error = [(matrix_error = "M_UNKNOWN_TOKEN", from = SyncronizationApplicationError::Unauthorized)])]
    Unauthorized(Json<MatrixErrorResponse>),
    #[matrix(status = 403, error = [(matrix_error = "M_FORBIDDEN", from = SyncronizationApplicationError::Forbidden)])]
    Forbidden(Json<MatrixErrorResponse>),
    #[matrix(status = 500, error = [(matrix_error = "M_UNKNOWN", from = SyncronizationApplicationError::Internal)])]
    Internal(Json<MatrixErrorResponse>),
}

pub async fn define_filter(
    Path(user_id): Path<String>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
    Json(request): Json<DefineFilterInfo>,
) -> DefineFilterResponse {
    match application_state.syncronization_service.define_filter(
        &access_session,
        &user_id,
        request.payload,
    ) {
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

pub async fn sync(
    sync_query: Result<Query<SyncQueryInfo>, QueryRejection>,
    State(application_state): State<ApplicationState>,
    Extension(access_session): Extension<AccessSessionStorageUnit>,
) -> SyncResponse {
    let Ok(Query(query)) = sync_query else {
        return SyncResponse::from_mapped_error(SyncronizationApplicationError::InvalidParameter);
    };

    let request = match build_sync_request(query) {
        Ok(request) => request,
        Err(error_kind) => return SyncResponse::from_mapped_error(error_kind),
    };

    match application_state
        .syncronization_service
        .sync(&access_session, request)
        .await
    {
        Ok(view) => SyncResponse::Ok(Json(SyncResponseView::from(view))),
        Err(error_kind) => {
            if matches!(error_kind, SyncronizationApplicationError::Internal) {
                error!("sync failed with internal error");
            } else {
                info!(error = %error_kind, "sync rejected");
            }
            SyncResponse::from_mapped_error(error_kind)
        }
    }
}

fn build_sync_request(query: SyncQueryInfo) -> Result<SyncRequest, SyncronizationApplicationError> {
    let filter = query.filter.as_deref().map(parse_sync_filter).transpose()?;
    let timeline_limit = parse_timeline_limit_from_filter(filter.as_ref())?.unwrap_or(10);

    Ok(SyncRequest {
        filter,
        since: query.since,
        full_state: query.full_state.unwrap_or(false),
        set_presence: query.set_presence,
        timeout_milliseconds: query.timeout.unwrap_or(0),
        use_state_after: query.use_state_after.unwrap_or(false),
        timeline_limit,
    })
}

fn parse_sync_filter(
    raw_filter: &str,
) -> Result<SyncFilterSelection, SyncronizationApplicationError> {
    let trimmed_filter = raw_filter.trim();
    if trimmed_filter.starts_with('{') {
        let inline_filter: serde_json::Value = serde_json::from_str(trimmed_filter)
            .map_err(|_| SyncronizationApplicationError::InvalidParameter)?;
        if !inline_filter.is_object() {
            return Err(SyncronizationApplicationError::InvalidParameter);
        }
        return Ok(SyncFilterSelection::InlineFilterDefinition(inline_filter));
    }

    if trimmed_filter.is_empty() {
        return Err(SyncronizationApplicationError::InvalidParameter);
    }

    Ok(SyncFilterSelection::StoredFilterIdentifier(
        trimmed_filter.to_owned(),
    ))
}

fn parse_timeline_limit_from_filter(
    filter: Option<&SyncFilterSelection>,
) -> Result<Option<usize>, SyncronizationApplicationError> {
    let Some(SyncFilterSelection::InlineFilterDefinition(inline_filter)) = filter else {
        return Ok(None);
    };

    let timeline_limit = inline_filter
        .get("room")
        .and_then(|room| room.get("timeline"))
        .and_then(|timeline| timeline.get("limit"))
        .and_then(serde_json::Value::as_u64);

    timeline_limit.map_or(Ok(None), |value| {
        usize::try_from(value)
            .map(Some)
            .map_err(|_| SyncronizationApplicationError::InvalidParameter)
    })
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
