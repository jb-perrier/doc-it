use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

use crate::{db::Database, models::db::FolderRow};

pub const ROOT_FOLDER_ID: &str = "workspace-root";

pub enum DeleteFolderResult {
    Deleted,
    NotFound,
    Protected,
    ContainsDocuments,
}

impl Database {
    pub async fn list_folders(&self) -> Result<Vec<FolderRow>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, parent_folder_id, name, created_at, updated_at
            FROM folders
            ORDER BY created_at ASC, name COLLATE NOCASE ASC
            "#,
        )
        .fetch_all(self.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| FolderRow {
                id: row.get("id"),
                parent_folder_id: row.get("parent_folder_id"),
                name: row.get("name"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }

    pub async fn create_folder(
        &self,
        name: &str,
        parent_folder_id: Option<&str>,
    ) -> Result<FolderRow, sqlx::Error> {
        let now = iso_now();
        let folder_id = Uuid::now_v7().to_string();
        let parent_folder_id = parent_folder_id.unwrap_or(ROOT_FOLDER_ID);
        let name = sanitize_folder_name(name);

        sqlx::query(
            r#"
            INSERT INTO folders (id, parent_folder_id, name, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&folder_id)
        .bind(parent_folder_id)
        .bind(&name)
        .bind(&now)
        .bind(&now)
        .execute(self.pool())
        .await?;

        Ok(FolderRow {
            id: folder_id,
            parent_folder_id: Some(parent_folder_id.to_string()),
            name,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn rename_folder(
        &self,
        folder_id: &str,
        name: &str,
    ) -> Result<Option<FolderRow>, sqlx::Error> {
        let now = iso_now();
        let name = sanitize_folder_name(name);
        let result = sqlx::query(
            r#"
            UPDATE folders
            SET name = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&name)
        .bind(&now)
        .bind(folder_id)
        .execute(self.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        let row = sqlx::query(
            r#"
            SELECT id, parent_folder_id, name, created_at, updated_at
            FROM folders
            WHERE id = ?
            "#,
        )
        .bind(folder_id)
        .fetch_one(self.pool())
        .await?;

        Ok(Some(FolderRow {
            id: row.get("id"),
            parent_folder_id: row.get("parent_folder_id"),
            name: row.get("name"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    pub async fn delete_folder(
        &self,
        folder_id: &str,
    ) -> Result<DeleteFolderResult, sqlx::Error> {
        if folder_id == ROOT_FOLDER_ID {
            return Ok(DeleteFolderResult::Protected);
        }

        let exists = sqlx::query("SELECT 1 FROM folders WHERE id = ?")
            .bind(folder_id)
            .fetch_optional(self.pool())
            .await?
            .is_some();

        if !exists {
            return Ok(DeleteFolderResult::NotFound);
        }

        let row = sqlx::query(
            r#"
            WITH RECURSIVE folder_tree(id) AS (
                SELECT id
                FROM folders
                WHERE id = ?

                UNION ALL

                SELECT folders.id
                FROM folders
                INNER JOIN folder_tree ON folders.parent_folder_id = folder_tree.id
            )
            SELECT EXISTS(
                SELECT 1
                FROM documents
                WHERE COALESCE(folder_id, ?) IN (SELECT id FROM folder_tree)
            ) AS has_documents
            "#,
        )
        .bind(folder_id)
        .bind(ROOT_FOLDER_ID)
        .fetch_one(self.pool())
        .await?;

        let has_documents: bool = row.get("has_documents");
        if has_documents {
            return Ok(DeleteFolderResult::ContainsDocuments);
        }

        sqlx::query("DELETE FROM folders WHERE id = ?")
            .bind(folder_id)
            .execute(self.pool())
            .await?;

        Ok(DeleteFolderResult::Deleted)
    }
}

fn sanitize_folder_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "Untitled folder".to_string()
    } else {
        trimmed.to_string()
    }
}

fn iso_now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use crate::db::{Database, migrations::run_migrations};

    #[tokio::test]
    async fn migrations_create_workspace_root() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        run_migrations(&pool).await.expect("run migrations");
        let db = Database::new(pool);

        let folders = db.list_folders().await.expect("list folders");

        assert!(
            folders
                .iter()
                .any(|folder| folder.id == super::ROOT_FOLDER_ID)
        );
    }

    #[tokio::test]
    async fn delete_folder_rejects_workspace_root() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        run_migrations(&pool).await.expect("run migrations");
        let db = Database::new(pool);

        let result = db
            .delete_folder(super::ROOT_FOLDER_ID)
            .await
            .expect("delete protected folder");

        assert!(matches!(result, super::DeleteFolderResult::Protected));
    }

    #[tokio::test]
    async fn delete_folder_allows_empty_folder() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        run_migrations(&pool).await.expect("run migrations");
        let db = Database::new(pool.clone());

        let folder = db
            .create_folder("Archive", Some(super::ROOT_FOLDER_ID))
            .await
            .expect("create folder");

        let result = db
            .delete_folder(&folder.id)
            .await
            .expect("delete empty folder");

        assert!(matches!(result, super::DeleteFolderResult::Deleted));

        let folders = db.list_folders().await.expect("list folders");
        assert!(folders.iter().all(|listed| listed.id != folder.id));
    }

    #[tokio::test]
    async fn delete_folder_rejects_subtrees_with_documents() {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        run_migrations(&pool).await.expect("run migrations");
        let db = Database::new(pool);

        let folder = db
            .create_folder("Projects", Some(super::ROOT_FOLDER_ID))
            .await
            .expect("create folder");
        db.create_document("Spec", Some(&folder.id))
            .await
            .expect("create document in folder");

        let result = db
            .delete_folder(&folder.id)
            .await
            .expect("delete folder with documents");

        assert!(matches!(result, super::DeleteFolderResult::ContainsDocuments));
    }
}
