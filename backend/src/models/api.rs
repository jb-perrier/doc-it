use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPayload {
    pub id: String,
    pub folder_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentListPayload {
    pub id: String,
    pub folder_id: String,
    pub title: String,
    pub updated_at: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDocumentRequest {
    pub title: String,
    pub folder_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameDocumentRequest {
    pub title: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveDocumentRequest {
    pub folder_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderPayload {
    pub id: String,
    pub parent_folder_id: Option<String>,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderRequest {
    pub name: String,
    pub parent_folder_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameFolderRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct DocumentListResponse {
    pub documents: Vec<DocumentListPayload>,
}

#[derive(Debug, Serialize)]
pub struct DocumentResponse {
    pub document: DocumentPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FolderListResponse {
    pub folders: Vec<FolderPayload>,
}

#[derive(Debug, Serialize)]
pub struct FolderResponse {
    pub folder: FolderPayload,
}

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub error: ErrorPayload,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Document not found")]
    NotFound,
    #[error("Folder not found")]
    FolderNotFound,
    #[error("{0}")]
    BadRequest(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("Internal server error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "document_not_found",
                self.to_string(),
            ),
            Self::FolderNotFound => (
                StatusCode::NOT_FOUND,
                "folder_not_found",
                self.to_string(),
            ),
            Self::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request", message),
            Self::Sqlx(error) => {
                if let Some(message) = client_database_message(&error) {
                    tracing::warn!(?error, "database constraint rejected client request");
                    (StatusCode::BAD_REQUEST, "bad_request", message)
                } else {
                    tracing::error!(?error, "database error");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "internal_error",
                        "Unexpected database failure".to_string(),
                    )
                }
            }
            Self::Serde(error) => {
                tracing::warn!(?error, "invalid json payload");
                (
                    StatusCode::BAD_REQUEST,
                    "invalid_json",
                    "Malformed JSON payload".to_string(),
                )
            }
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "Unexpected server failure".to_string(),
            ),
        };

        (
            status,
            Json(ErrorEnvelope {
                error: ErrorPayload { code, message },
            }),
        )
            .into_response()
    }
}

fn client_database_message(error: &sqlx::Error) -> Option<String> {
    let sqlx::Error::Database(database_error) = error else {
        return None;
    };

    let message = database_error.message();
    if message.contains("FOREIGN KEY constraint failed") {
        return Some("Referenced folder does not exist".to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::AppError;

    use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
    use serde_json::Value;
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::db::{Database, migrations::run_migrations};

    #[tokio::test]
    async fn invalid_folder_reference_maps_to_bad_request() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        run_migrations(&pool).await.expect("run migrations");
        let db = Database::new(pool);

        let error = db
            .create_document("Bad folder", Some("missing-folder"))
            .await
            .expect_err("invalid folder should fail");

        let response = AppError::from(error).into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response body");
        let payload: Value = serde_json::from_slice(&body).expect("parse response body");

        assert_eq!(payload["error"]["code"], "bad_request");
        assert_eq!(payload["error"]["message"], "Referenced folder does not exist");
    }
}
