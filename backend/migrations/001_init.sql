CREATE TABLE IF NOT EXISTS documents (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_documents_updated_at
ON documents(updated_at DESC);

CREATE TABLE IF NOT EXISTS document_snapshots (
  document_id TEXT PRIMARY KEY,
  yjs_snapshot BLOB NOT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_document_snapshots_created_at
ON document_snapshots(created_at DESC);