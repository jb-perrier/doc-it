use crate::{db::Database, realtime::rooms::RoomManager};

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub rooms: RoomManager,
}

impl AppState {
    pub fn new(db: Database, rooms: RoomManager) -> Self {
        Self { db, rooms }
    }
}