use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
        routing::{get, patch},
    Json, Router,
};

use crate::{
    app_state::AppState,
    models::api::{
        AppError, CreateDocumentRequest, DocumentListPayload, DocumentListResponse,
        DocumentPayload, DocumentResponse, RenameDocumentRequest,
    },
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/documents", get(list_documents).post(create_document))
        .route("/documents/{id}", get(get_document))
        .route("/documents/{id}/title", patch(rename_document))
}

async fn list_documents(
    State(state): State<Arc<AppState>>,
) -> Result<Json<DocumentListResponse>, AppError> {
    let documents = state
        .db
        .list_documents()
        .await?
        .into_iter()
        .map(|document| DocumentListPayload {
            id: document.id,
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
    let document = state.db.create_document(&payload.title).await?;

    Ok((
        StatusCode::CREATED,
        Json(DocumentResponse {
            document: DocumentPayload {
                id: document.id,
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
            title: document.title,
            created_at: Some(document.created_at),
            updated_at: document.updated_at,
        },
    }))
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