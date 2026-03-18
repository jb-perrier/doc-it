#[derive(Debug, Clone)]
pub struct DocumentRow {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct DocumentListItem {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SnapshotRow {
    pub yjs_snapshot: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RoomSeed {
    pub document: DocumentRow,
    pub snapshot: SnapshotRow,
}