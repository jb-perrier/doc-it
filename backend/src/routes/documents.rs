use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use base64::{Engine, engine::general_purpose::STANDARD as Base64};
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
            snapshot: None,
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

    Ok(Json(DocumentResponse {
        document: DocumentPayload {
            id: document.id,
            folder_id: document.folder_id,
            title: document.title,
            created_at: Some(document.created_at),
            updated_at: document.updated_at,
        },
        snapshot: Some(Base64.encode(snapshot)),
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
            snapshot: None,
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

    Ok(Json(DocumentResponse {
        document,
        snapshot: None,
    }))
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

    Ok(Json(DocumentResponse {
        document,
        snapshot: None,
    }))
}

async fn delete_document(
    State(state): State<Arc<AppState>>,
    Path(document_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let deleted = state.db.delete_document(&document_id).await?;

    if !deleted {
        return Err(AppError::NotFound);
    }

    state.rooms.delete_room(&document_id).await;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::delete_document;
    use std::sync::Arc;

    use axum::{extract::{ws::Message, Path, State}, http::StatusCode};
    use sqlx::sqlite::SqlitePoolOptions;
    use tokio::sync::mpsc;

    use crate::{
        app_state::AppState,
        db::{Database, migrations::run_migrations},
        realtime::rooms::RoomManager,
    };

    async fn test_state() -> Arc<AppState> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        run_migrations(&pool).await.expect("run migrations");

        let db = Database::new(pool);
        let rooms = RoomManager::new(db.clone());

        Arc::new(AppState::new(db, rooms))
    }

    async fn expect_close(receiver: &mut mpsc::UnboundedReceiver<Message>) {
        loop {
            match receiver.recv().await {
                Some(Message::Close(None)) => return,
                Some(_) => continue,
                None => panic!("expected close message"),
            }
        }
    }

    #[tokio::test]
    async fn delete_document_closes_active_room_and_removes_snapshot_source() {
        let state = test_state().await;
        let document = state
            .db
            .create_document("Delete route", None)
            .await
            .expect("create document");

        let room = state
            .rooms
            .get_or_create(&document.id)
            .await
            .expect("create room");
        let (sender, mut receiver) = mpsc::unbounded_channel();
        room.join(
            "client-1".to_string(),
            "Alice".to_string(),
            "#111111".to_string(),
            sender,
        )
        .await
        .expect("join room");

        let status = delete_document(
            State(state.clone()),
            Path(document.id.clone()),
        )
        .await
        .expect("delete document");

        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(state.rooms.current_snapshot(&document.id).await.is_none());
        assert!(state.db.get_document(&document.id).await.expect("query document").is_none());
        expect_close(&mut receiver).await;
    }
}
