CREATE TABLE IF NOT EXISTS folders (
  id TEXT PRIMARY KEY,
  parent_folder_id TEXT,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (parent_folder_id) REFERENCES folders(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_folders_parent_folder_id
ON folders(parent_folder_id);

INSERT OR IGNORE INTO folders (id, parent_folder_id, name, created_at, updated_at)
VALUES (
  'workspace-root',
  NULL,
  'Workspace',
  STRFTIME('%Y-%m-%dT%H:%M:%SZ', 'now'),
  STRFTIME('%Y-%m-%dT%H:%M:%SZ', 'now')
);