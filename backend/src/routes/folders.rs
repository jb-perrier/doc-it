use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
};

use crate::{
    app_state::AppState,
    db::folders::DeleteFolderResult,
    models::api::{
        AppError, CreateFolderRequest, FolderListResponse, FolderPayload, FolderResponse,
        RenameFolderRequest,
    },
    models::db::FolderRow,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/folders", get(list_folders).post(create_folder))
    .route("/folders/{id}", patch(rename_folder).delete(delete_folder))
}

async fn list_folders(
    State(state): State<Arc<AppState>>,
) -> Result<Json<FolderListResponse>, AppError> {
    let folders = state
        .db
        .list_folders()
        .await?
        .into_iter()
        .map(map_folder_payload)
        .collect();

    Ok(Json(FolderListResponse { folders }))
}

async fn create_folder(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateFolderRequest>,
) -> Result<(StatusCode, Json<FolderResponse>), AppError> {
    let folder = state
        .db
        .create_folder(&payload.name, payload.parent_folder_id.as_deref())
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(FolderResponse {
            folder: map_folder_payload(folder),
        }),
    ))
}

async fn rename_folder(
    State(state): State<Arc<AppState>>,
    Path(folder_id): Path<String>,
    Json(payload): Json<RenameFolderRequest>,
) -> Result<Json<FolderResponse>, AppError> {
    let folder = state
        .db
        .rename_folder(&folder_id, &payload.name)
        .await?
        .ok_or(AppError::FolderNotFound)?;

    Ok(Json(FolderResponse {
        folder: map_folder_payload(folder),
    }))
}

async fn delete_folder(
    State(state): State<Arc<AppState>>,
    Path(folder_id): Path<String>,
) -> Result<StatusCode, AppError> {
    match state.db.delete_folder(&folder_id).await? {
        DeleteFolderResult::Deleted => Ok(StatusCode::NO_CONTENT),
        DeleteFolderResult::NotFound => Err(AppError::FolderNotFound),
        DeleteFolderResult::Protected => Err(AppError::BadRequest(
            "Workspace cannot be deleted".to_string(),
        )),
        DeleteFolderResult::ContainsDocuments => Err(AppError::BadRequest(
            "Move or delete documents inside this folder before removing it"
                .to_string(),
        )),
    }
}

fn map_folder_payload(folder: FolderRow) -> FolderPayload {
    FolderPayload {
        id: folder.id,
        parent_folder_id: folder.parent_folder_id,
        name: folder.name,
        created_at: folder.created_at,
        updated_at: folder.updated_at,
    }
}
