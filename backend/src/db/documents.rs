use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;
use yrs::{Doc, ReadTxn, StateVector, Transact};

use crate::{
    db::Database,
    models::{
        api::DocumentPayload,
        db::{DocumentListItem, DocumentRow, RoomSeed, SnapshotRow},
    },
};

use super::folders::DEFAULT_FOLDER_ID;

const COLLAB_FIELD: &str = "content";

impl Database {
    pub async fn list_documents(
        &self,
        folder_id: Option<&str>,
    ) -> Result<Vec<DocumentListItem>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, COALESCE(folder_id, ?) AS folder_id, title, created_at, updated_at
            FROM documents
            WHERE (? IS NULL OR COALESCE(folder_id, ?) = ?)
            ORDER BY updated_at DESC
            "#,
        )
        .bind(DEFAULT_FOLDER_ID)
        .bind(folder_id)
        .bind(DEFAULT_FOLDER_ID)
        .bind(folder_id)
        .fetch_all(self.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| DocumentListItem {
                id: row.get("id"),
                folder_id: row.get("folder_id"),
                title: row.get("title"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect())
    }

    pub async fn create_document(
        &self,
        title: &str,
        folder_id: Option<&str>,
    ) -> Result<DocumentRow, sqlx::Error> {
        let now = iso_now();
        let document_id = Uuid::now_v7().to_string();
        let folder_id = folder_id.unwrap_or(DEFAULT_FOLDER_ID).to_string();
        let title = sanitize_title(title);
        let initial_snapshot = empty_snapshot();
        let mut tx = self.pool().begin().await?;

        sqlx::query(
            r#"
            INSERT INTO documents (id, folder_id, title, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&document_id)
        .bind(&folder_id)
        .bind(&title)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO document_snapshots (document_id, yjs_snapshot, created_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&document_id)
        .bind(initial_snapshot)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(DocumentRow {
            id: document_id,
            folder_id,
            title,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub async fn duplicate_document(
        &self,
        source_document_id: &str,
        snapshot: &[u8],
    ) -> Result<Option<DocumentRow>, sqlx::Error> {
        let source = sqlx::query(
            r#"
            SELECT COALESCE(folder_id, ?) AS folder_id, title
            FROM documents
            WHERE id = ?
            "#,
        )
        .bind(DEFAULT_FOLDER_ID)
        .bind(source_document_id)
        .fetch_optional(self.pool())
        .await?;

        let Some(source) = source else {
            return Ok(None);
        };

        let now = iso_now();
        let document_id = Uuid::now_v7().to_string();
        let folder_id: String = source.get("folder_id");
        let title = duplicate_title(source.get::<String, _>("title").as_str());
        let mut tx = self.pool().begin().await?;

        sqlx::query(
            r#"
            INSERT INTO documents (id, folder_id, title, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&document_id)
        .bind(&folder_id)
        .bind(&title)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO document_snapshots (document_id, yjs_snapshot, created_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&document_id)
        .bind(snapshot)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(DocumentRow {
            id: document_id,
            folder_id,
            title,
            created_at: now.clone(),
            updated_at: now,
        }))
    }

    pub async fn get_document(
        &self,
        document_id: &str,
    ) -> Result<Option<DocumentRow>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT id, COALESCE(folder_id, ?) AS folder_id, title, created_at, updated_at
            FROM documents
            WHERE id = ?
            "#,
        )
        .bind(DEFAULT_FOLDER_ID)
        .bind(document_id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(|row| DocumentRow {
            id: row.get("id"),
            folder_id: row.get("folder_id"),
            title: row.get("title"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    pub async fn rename_document(
        &self,
        document_id: &str,
        title: &str,
    ) -> Result<Option<DocumentPayload>, sqlx::Error> {
        let now = iso_now();
        let title = sanitize_title(title);
        let result = sqlx::query(
            r#"
            UPDATE documents
            SET title = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&title)
        .bind(&now)
        .bind(document_id)
        .execute(self.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        let folder_id = row_folder_id(self, document_id).await?;

        Ok(Some(DocumentPayload {
            id: document_id.to_string(),
            folder_id,
            title,
            created_at: None,
            updated_at: now,
        }))
    }

    pub async fn move_document_to_folder(
        &self,
        document_id: &str,
        folder_id: &str,
    ) -> Result<Option<DocumentPayload>, sqlx::Error> {
        let now = iso_now();
        let result = sqlx::query(
            r#"
            UPDATE documents
            SET folder_id = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(folder_id)
        .bind(&now)
        .bind(document_id)
        .execute(self.pool())
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        let row = sqlx::query(
            r#"
            SELECT id, COALESCE(folder_id, ?) AS folder_id, title, created_at, updated_at
            FROM documents
            WHERE id = ?
            "#,
        )
        .bind(DEFAULT_FOLDER_ID)
        .bind(document_id)
        .fetch_one(self.pool())
        .await?;

        Ok(Some(DocumentPayload {
            id: row.get("id"),
            folder_id: row.get("folder_id"),
            title: row.get("title"),
            created_at: Some(row.get("created_at")),
            updated_at: row.get("updated_at"),
        }))
    }

    pub async fn delete_document(&self, document_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM documents
            WHERE id = ?
            "#,
        )
        .bind(document_id)
        .execute(self.pool())
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn load_room_seed(&self, document_id: &str) -> Result<Option<RoomSeed>, sqlx::Error> {
        let fallback_snapshot = empty_snapshot();
        let row = sqlx::query(
            r#"
            SELECT
                d.id,
                COALESCE(d.folder_id, ?) AS folder_id,
                d.title,
                d.created_at AS document_created_at,
                d.updated_at,
                COALESCE(s.yjs_snapshot, ?) AS yjs_snapshot
            FROM documents d
            LEFT JOIN document_snapshots s ON s.document_id = d.id
            WHERE d.id = ?
            "#,
        )
        .bind(DEFAULT_FOLDER_ID)
        .bind(fallback_snapshot)
        .bind(document_id)
        .fetch_optional(self.pool())
        .await?;

        Ok(row.map(|row| RoomSeed {
            document: DocumentRow {
                id: row.get("id"),
                folder_id: row.get("folder_id"),
                title: row.get("title"),
                created_at: row.get("document_created_at"),
                updated_at: row.get("updated_at"),
            },
            snapshot: SnapshotRow {
                yjs_snapshot: row.get("yjs_snapshot"),
            },
        }))
    }

    pub async fn persist_room_state(
        &self,
        document_id: &str,
        snapshot: &[u8],
    ) -> Result<bool, sqlx::Error> {
        let now = iso_now();
        let mut tx = self.pool().begin().await?;

        let result = sqlx::query(
            r#"
            UPDATE documents
            SET updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&now)
        .bind(document_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query(
            r#"
            INSERT INTO document_snapshots (document_id, yjs_snapshot, created_at)
            VALUES (?, ?, ?)
            ON CONFLICT(document_id) DO UPDATE SET
                yjs_snapshot = excluded.yjs_snapshot,
                created_at = excluded.created_at
            "#,
        )
        .bind(document_id)
        .bind(snapshot)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }
}

async fn row_folder_id(db: &Database, document_id: &str) -> Result<String, sqlx::Error> {
    let row = sqlx::query("SELECT COALESCE(folder_id, ?) AS folder_id FROM documents WHERE id = ?")
        .bind(DEFAULT_FOLDER_ID)
        .bind(document_id)
        .fetch_one(db.pool())
        .await?;

    Ok(row.get("folder_id"))
}

fn sanitize_title(title: &str) -> String {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        "Untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

fn duplicate_title(title: &str) -> String {
    format!("{} Copy", sanitize_title(title))
}

fn iso_now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn empty_snapshot() -> Vec<u8> {
    let doc = Doc::new();
    doc.get_or_insert_xml_fragment(COLLAB_FIELD);
    let txn = doc.transact();
    txn.encode_state_as_update_v1(&StateVector::default())
}

#[cfg(test)]
mod tests {
    use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};
    use yrs::{
        Doc, GetString, ReadTxn, StateVector, Transact, Update, XmlFragment, XmlTextPrelim,
        updates::decoder::Decode,
    };

    use crate::db::{Database, documents::COLLAB_FIELD, migrations::run_migrations};

    async fn test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        run_migrations(&pool).await.expect("run migrations");
        pool
    }

    fn snapshot_with_text(text: &str) -> Vec<u8> {
        let doc = Doc::new();
        let fragment = doc.get_or_insert_xml_fragment(COLLAB_FIELD);
        let mut txn = doc.transact_mut();
        fragment.push_back(&mut txn, XmlTextPrelim::new(text));
        txn.encode_state_as_update_v1(&StateVector::default())
    }

    fn snapshot_text(snapshot: &[u8]) -> String {
        let doc = Doc::new();
        let update = Update::decode_v1(snapshot).expect("decode snapshot");
        let mut txn = doc.transact_mut();
        txn.apply_update(update).expect("apply snapshot");
        drop(txn);

        let txn = doc.transact();
        let fragment = txn
            .get_xml_fragment(COLLAB_FIELD)
            .expect("content fragment exists");

        fragment.get_string(&txn).to_string()
    }

    #[tokio::test]
    async fn create_document_creates_initial_snapshot() {
        let pool = test_db().await;
        let db = Database::new(pool.clone());

        let document = db
            .create_document("New doc", None)
            .await
            .expect("create document");
        let seed = db
            .load_room_seed(&document.id)
            .await
            .expect("load room seed")
            .expect("room seed exists");

        assert_eq!(seed.document.id, document.id);
        assert_eq!(seed.document.folder_id, super::DEFAULT_FOLDER_ID);

        let restored = Doc::new();
        let update = Update::decode_v1(&seed.snapshot.yjs_snapshot).expect("decode snapshot");
        let mut txn = restored.transact_mut();
        txn.apply_update(update).expect("apply snapshot");
        drop(txn);

        assert!(!seed.snapshot.yjs_snapshot.is_empty());

        let snapshot_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM document_snapshots")
            .fetch_one(&pool)
            .await
            .expect("count snapshots");
        assert_eq!(snapshot_count, 1);
    }

    #[tokio::test]
    async fn persist_room_state_keeps_single_latest_snapshot() {
        let pool = test_db().await;
        let db = Database::new(pool.clone());
        let document = db
            .create_document("Snapshot doc", None)
            .await
            .expect("create document");

        let first = snapshot_with_text("first");
        let second = snapshot_with_text("second");

        assert!(
            db.persist_room_state(&document.id, &first)
                .await
                .expect("persist first snapshot")
        );
        assert!(
            db.persist_room_state(&document.id, &second)
                .await
                .expect("persist second snapshot")
        );

        let row = sqlx::query("SELECT yjs_snapshot FROM document_snapshots WHERE document_id = ?")
            .bind(&document.id)
            .fetch_one(&pool)
            .await
            .expect("load snapshot row");

        let stored: Vec<u8> = row.get("yjs_snapshot");
        assert_eq!(stored, second);

        let snapshot_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM document_snapshots WHERE document_id = ?")
                .bind(&document.id)
                .fetch_one(&pool)
                .await
                .expect("count per document");
        assert_eq!(snapshot_count, 1);
    }

    #[tokio::test]
    async fn load_room_seed_falls_back_when_snapshot_row_is_missing() {
        let pool = test_db().await;
        let db = Database::new(pool.clone());
        let now = "2026-03-18T00:00:00Z";

        sqlx::query(
            "INSERT INTO documents (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)",
        )
        .bind("legacy-doc")
        .bind("Legacy")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("insert legacy document");

        let seed = db
            .load_room_seed("legacy-doc")
            .await
            .expect("load room seed")
            .expect("room seed exists");

        assert_eq!(seed.document.id, "legacy-doc");
        assert_eq!(seed.document.folder_id, super::DEFAULT_FOLDER_ID);
        assert!(!seed.snapshot.yjs_snapshot.is_empty());
    }

    #[tokio::test]
    async fn move_document_updates_folder() {
        let pool = test_db().await;
        let db = Database::new(pool);

        let parent = db
            .create_folder("Projects", None)
            .await
            .expect("create folder");
        let child = db
            .create_folder("Q2", Some(&parent.id))
            .await
            .expect("create nested folder");
        let document = db
            .create_document("Planning", None)
            .await
            .expect("create document");

        let moved = db
            .move_document_to_folder(&document.id, &child.id)
            .await
            .expect("move document")
            .expect("document exists");

        assert_eq!(moved.folder_id, child.id);
    }

    #[tokio::test]
    async fn duplicate_document_copies_snapshot_and_folder() {
        let pool = test_db().await;
        let db = Database::new(pool.clone());

        let parent = db
            .create_folder("Projects", None)
            .await
            .expect("create folder");
        let source = db
            .create_document("Planning", Some(&parent.id))
            .await
            .expect("create source document");
        let snapshot = snapshot_with_text("Copied content");

        db.persist_room_state(&source.id, &snapshot)
            .await
            .expect("persist snapshot");

        let duplicate = db
            .duplicate_document(&source.id, &snapshot)
            .await
            .expect("duplicate document")
            .expect("duplicate exists");

        assert_ne!(duplicate.id, source.id);
        assert_eq!(duplicate.folder_id, source.folder_id);
        assert_eq!(duplicate.title, "Planning Copy");

        let copied_seed = db
            .load_room_seed(&duplicate.id)
            .await
            .expect("load duplicate room seed")
            .expect("duplicate room seed exists");

        assert_eq!(
            snapshot_text(&copied_seed.snapshot.yjs_snapshot),
            "Copied content"
        );

        let document_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM documents")
            .fetch_one(&pool)
            .await
            .expect("count documents");
        assert_eq!(document_count, 2);
    }
}
