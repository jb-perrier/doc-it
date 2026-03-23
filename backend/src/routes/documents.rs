use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use serde::Deserialize;

use crate::{
    app_state::AppState,
    models::api::{
        AppError, CreateDocumentRequest, DocumentListPayload, DocumentListResponse,
        DocumentPayload, DocumentResponse, MoveDocumentRequest, RenameDocumentRequest,
    },
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListDocumentsQuery {
    folder_id: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/documents", get(list_documents).post(create_document))
        .route("/documents/{id}", get(get_document).delete(delete_document))
    .route("/documents/{id}/duplicate", post(duplicate_document))
        .route("/documents/{id}/title", patch(rename_document))
        .route("/documents/{id}/folder", patch(move_document_to_folder))
}

async fn list_documents(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListDocumentsQuery>,
) -> Result<Json<DocumentListResponse>, AppError> {
    let documents = state
        .db
        .list_documents(query.folder_id.as_deref())
        .await?
        .into_iter()
        .map(|document| DocumentListPayload {
            id: document.id,
            folder_id: document.folder_id,
            title: document.title,
            created_at: document.created_at,
            updated_at: document.updated_at,
        })
        .collect();

    Ok(Json(DocumentListResponse { documents }))
}

async fn create_document(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<DocumentResponse>), AppError> {
    let document = state
        .db
        .create_document(&payload.title, payload.folder_id.as_deref())
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(DocumentResponse {
            document: DocumentPayload {
                id: document.id,
                folder_id: document.folder_id,
                title: document.title,
                created_at: Some(document.created_at),
                updated_at: document.updated_at,
            },
        }),
    ))
}

async fn get_document(
    State(state): State<Arc<AppState>>,
    Path(document_id): Path<String>,
) -> Result<Json<DocumentResponse>, AppError> {
    let document = state
        .db
        .get_document(&document_id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(DocumentResponse {
        document: DocumentPayload {
            id: document.id,
            folder_id: document.folder_id,
            title: document.title,
            created_at: Some(document.created_at),
            updated_at: document.updated_at,
        },
    }))
}

async fn duplicate_document(
    State(state): State<Arc<AppState>>,
    Path(document_id): Path<String>,
) -> Result<(StatusCode, Json<DocumentResponse>), AppError> {
    let snapshot = match state.rooms.current_snapshot(&document_id).await {
        Some(snapshot) => snapshot,
        None => state
            .db
            .load_room_seed(&document_id)
            .await?
            .ok_or(AppError::NotFound)?
            .snapshot
            .yjs_snapshot,
    };

    let document = state
        .db
        .duplicate_document(&document_id, &snapshot)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok((
        StatusCode::CREATED,
        Json(DocumentResponse {
            document: DocumentPayload {
                id: document.id,
                folder_id: document.folder_id,
                title: document.title,
                created_at: Some(document.created_at),
                updated_at: document.updated_at,
            },
        }),
    ))
}

async fn rename_document(
    State(state): State<Arc<AppState>>,
    Path(document_id): Path<String>,
    Json(payload): Json<RenameDocumentRequest>,
) -> Result<Json<DocumentResponse>, AppError> {
    let document = state
        .db
        .rename_document(&document_id, &payload.title)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(DocumentResponse { document }))
}

async fn move_document_to_folder(
    State(state): State<Arc<AppState>>,
    Path(document_id): Path<String>,
    Json(payload): Json<MoveDocumentRequest>,
) -> Result<Json<DocumentResponse>, AppError> {
    let document = state
        .db
        .move_document_to_folder(&document_id, &payload.folder_id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(DocumentResponse { document }))
}

async fn delete_document(
    State(state): State<Arc<AppState>>,
    Path(document_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let deleted = state.db.delete_document(&document_id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}
