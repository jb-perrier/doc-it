UPDATE documents
SET folder_id = 'workspace-inbox'
WHERE folder_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_documents_folder_id
ON documents(folder_id);