use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use axum::extract::ws::Message;
use base64::{Engine, engine::general_purpose::STANDARD as Base64};
use chrono::Utc;
use tokio::sync::{Mutex, RwLock, mpsc::UnboundedSender};
use yrs::{Doc, ReadTxn, StateVector, Transact, Update, updates::decoder::Decode};

use crate::{
    db::Database,
    models::{
        api::AppError,
        db::RoomSeed,
        ws::{
            JoinedPayload, PeerPresence, PresenceSnapshotPayload, PresenceUpdatePayload,
            ServerMessage, SyncInitPayload, SyncUpdatePayload,
        },
    },
};

#[derive(Clone)]
pub struct RoomManager {
    db: Database,
    rooms: Arc<RwLock<HashMap<String, Arc<Room>>>>,
}

impl RoomManager {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn spawn_presence_reaper(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                manager.expire_stale_peers().await;
            }
        });
    }

    pub async fn get_or_create(&self, document_id: &str) -> Result<Arc<Room>, AppError> {
        if let Some(room) = self.rooms.read().await.get(document_id).cloned() {
            return Ok(room);
        }

        let seed = self
            .db
            .load_room_seed(document_id)
            .await?
            .ok_or(AppError::NotFound)?;
        let room = Arc::new(Room::from_seed(self.db.clone(), seed)?);

        let mut rooms = self.rooms.write().await;
        let entry = rooms
            .entry(document_id.to_string())
            .or_insert_with(|| room.clone());
        Ok(entry.clone())
    }

    pub async fn remove_peer(&self, document_id: &str, client_id: &str) {
        let room = self.rooms.read().await.get(document_id).cloned();

        if let Some(room) = room {
            room.remove_peer(client_id).await;
            self.evict_if_inactive(document_id, &room).await;
        }
    }

    pub async fn current_snapshot(&self, document_id: &str) -> Option<Vec<u8>> {
        let room = self.rooms.read().await.get(document_id).cloned();

        match room {
            Some(room) => Some(room.full_update_bytes().await),
            None => None,
        }
    }

    pub async fn delete_room(&self, document_id: &str) {
        let room = self.rooms.write().await.remove(document_id);

        if let Some(room) = room {
            room.shutdown().await;
        }
    }

    async fn expire_stale_peers(&self) {
        let rooms = self.rooms.read().await;
        let list = rooms
            .iter()
            .map(|(document_id, room)| (document_id.clone(), room.clone()))
            .collect::<Vec<_>>();
        drop(rooms);

        for (document_id, room) in list {
            room.expire_stale_peers().await;
            self.evict_if_inactive(&document_id, &room).await;
        }
    }

    async fn evict_if_inactive(&self, document_id: &str, room: &Arc<Room>) {
        if !room.is_empty().await {
            return;
        }

        let mut rooms = self.rooms.write().await;
        let Some(current) = rooms.get(document_id) else {
            return;
        };

        if Arc::ptr_eq(current, room) && room.is_empty().await {
            rooms.remove(document_id);
        }
    }
}

pub struct Room {
    document_id: String,
    db: Database,
    doc: Mutex<Doc>,
    peers: Mutex<HashMap<String, PeerState>>,
    save_ticket: AtomicU64,
}

struct PeerState {
    client_id: String,
    name: String,
    color: String,
    anchor: Option<u32>,
    head: Option<u32>,
    last_seen: Instant,
    sender: UnboundedSender<Message>,
}

pub struct JoinContext {
    pub joined: JoinedPayload,
    pub sync_init: SyncInitPayload,
    pub presence: PresenceSnapshotPayload,
}

impl Room {
    fn from_seed(db: Database, seed: RoomSeed) -> Result<Self, AppError> {
        let doc = Doc::new();
        let update =
            Update::decode_v1(&seed.snapshot.yjs_snapshot).map_err(|_| AppError::Internal)?;
        {
            let mut txn = doc.transact_mut();
            txn.apply_update(update).map_err(|_| AppError::Internal)?;
        }

        Ok(Self {
            document_id: seed.document.id,
            db,
            doc: Mutex::new(doc),
            peers: Mutex::new(HashMap::new()),
            save_ticket: AtomicU64::new(0),
        })
    }

    pub async fn join(
        self: &Arc<Self>,
        client_id: String,
        name: String,
        color: String,
        sender: UnboundedSender<Message>,
    ) -> Result<JoinContext, AppError> {
        let mut peers = self.peers.lock().await;
        let peer = PeerState {
            client_id: client_id.clone(),
            name: name.clone(),
            color: color.clone(),
            anchor: None,
            head: None,
            last_seen: Instant::now(),
            sender,
        };
        peers.insert(client_id.clone(), peer);

        let room_peers = peers.values().map(to_presence).collect::<Vec<_>>();
        drop(peers);

        let joined = JoinedPayload {
            document_id: self.document_id.clone(),
            server_time: iso_now(),
            peers: room_peers.clone(),
        };

        let sync_init = SyncInitPayload {
            update: self.full_update_base64().await,
        };

        let presence = PresenceSnapshotPayload { peers: room_peers };
        self.broadcast_presence_snapshot().await;

        Ok(JoinContext {
            joined,
            sync_init,
            presence,
        })
    }

    pub async fn apply_update(
        self: &Arc<Self>,
        payload: SyncUpdatePayload,
    ) -> Result<(), AppError> {
        let raw_update = Base64
            .decode(payload.update.as_bytes())
            .map_err(|_| AppError::BadRequest("Invalid Yjs update payload".to_string()))?;

        {
            let doc = self.doc.lock().await;
            let update = Update::decode_v1(&raw_update)
                .map_err(|_| AppError::BadRequest("Invalid Yjs update payload".to_string()))?;
            let mut txn = doc.transact_mut();
            txn.apply_update(update)
                .map_err(|_| AppError::BadRequest("Failed to apply Yjs update".to_string()))?;
        }

        let sender_id = payload.client_id.clone();
        self.broadcast_except(&sender_id, &ServerMessage::SyncUpdate(payload))
            .await;
        self.schedule_persist();
        Ok(())
    }

    pub async fn update_presence(&self, payload: PresenceUpdatePayload) {
        let mut peers = self.peers.lock().await;
        if let Some(peer) = peers.get_mut(&payload.client_id) {
            peer.anchor = payload.anchor;
            peer.head = payload.head;
            peer.last_seen = Instant::now();
        }
        drop(peers);
        self.broadcast_presence_snapshot().await;
    }

    pub async fn heartbeat(&self, client_id: &str) {
        let mut peers = self.peers.lock().await;
        if let Some(peer) = peers.get_mut(client_id) {
            peer.last_seen = Instant::now();
        }
    }

    pub async fn remove_peer(self: &Arc<Self>, client_id: &str) {
        self.remove_peer_internal(client_id, false).await;
    }

    pub async fn shutdown(&self) {
        self.save_ticket.fetch_add(1, Ordering::SeqCst);

        let peers = {
            let mut peers = self.peers.lock().await;
            peers.drain().map(|(_, peer)| peer).collect::<Vec<_>>()
        };

        for peer in peers {
            let _ = peer.sender.send(Message::Close(None));
        }
    }

    async fn is_empty(&self) -> bool {
        self.peers.lock().await.is_empty()
    }

    async fn expire_stale_peers(self: &Arc<Self>) {
        let stale_clients = {
            let peers = self.peers.lock().await;
            peers
                .iter()
                .filter(|(_, peer)| peer.last_seen.elapsed() > Duration::from_secs(30))
                .map(|(client_id, _)| client_id.clone())
                .collect::<Vec<_>>()
        };

        for client_id in stale_clients {
            self.remove_peer_internal(&client_id, true).await;
        }
    }

    async fn remove_peer_internal(self: &Arc<Self>, client_id: &str, close_socket: bool) {
        let mut peers = self.peers.lock().await;
        let removed_peer = peers.remove(client_id);
        let is_empty = peers.is_empty();
        drop(peers);

        if let Some(peer) = removed_peer {
            if close_socket {
                let _ = peer.sender.send(Message::Close(None));
            }
            self.broadcast_presence_snapshot().await;
        }

        if is_empty {
            self.save_ticket.fetch_add(1, Ordering::SeqCst);
            self.flush().await;
        }
    }

    fn schedule_persist(self: &Arc<Self>) {
        let ticket = self.save_ticket.fetch_add(1, Ordering::SeqCst) + 1;
        let room = self.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if room.save_ticket.load(Ordering::SeqCst) == ticket {
                room.flush().await;
            }
        });
    }

    async fn flush(&self) {
        let snapshot_bytes = self.full_update_bytes().await;

        match self
            .db
            .persist_room_state(&self.document_id, &snapshot_bytes)
            .await
        {
            Ok(true) => {}
            Ok(false) => {}
            Err(error) => {
                tracing::error!(?error, document_id = %self.document_id, "failed to persist room state")
            }
        }
    }

    async fn full_update_base64(&self) -> String {
        Base64.encode(self.full_update_bytes().await)
    }

    async fn full_update_bytes(&self) -> Vec<u8> {
        let doc = self.doc.lock().await;
        let txn = doc.transact();
        txn.encode_state_as_update_v1(&StateVector::default())
    }

    async fn broadcast_presence_snapshot(&self) {
        let peers = self.peers.lock().await;
        let payload = PresenceSnapshotPayload {
            peers: peers.values().map(to_presence).collect(),
        };
        drop(peers);
        self.broadcast(&ServerMessage::PresenceSnapshot(payload))
            .await;
    }

    async fn broadcast(&self, message: &ServerMessage) {
        let peers = self.peers.lock().await;
        let text = serialize_message(message);
        for peer in peers.values() {
            let _ = peer.sender.send(Message::Text(text.clone().into()));
        }
    }

    async fn broadcast_except(&self, client_id: &str, message: &ServerMessage) {
        let peers = self.peers.lock().await;
        let text = serialize_message(message);
        for peer in peers.values() {
            if peer.client_id != client_id {
                let _ = peer.sender.send(Message::Text(text.clone().into()));
            }
        }
    }
}

fn serialize_message(message: &ServerMessage) -> String {
    serde_json::to_string(message).unwrap_or_else(|_| {
        serde_json::to_string(&ServerMessage::Error(crate::models::ws::ErrorPayload {
            code: "internal_error",
            message: "Failed to serialize websocket payload",
        }))
        .unwrap_or_else(|_| "{\"type\":\"error\",\"payload\":{\"code\":\"internal_error\",\"message\":\"Serialization failed\"}}".to_string())
    })
}

fn to_presence(peer: &PeerState) -> PeerPresence {
    PeerPresence {
        client_id: peer.client_id.clone(),
        name: peer.name.clone(),
        color: peer.color.clone(),
        anchor: peer.anchor,
        head: peer.head,
    }
}

fn iso_now() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    use sqlx::sqlite::SqlitePoolOptions;
    use tokio::sync::mpsc;
    use yrs::{XmlFragment, XmlTextPrelim, types::GetString};

    use crate::{db::migrations::run_migrations, models::ws::SyncUpdatePayload};

    async fn test_db() -> Database {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        run_migrations(&pool).await.expect("run migrations");
        Database::new(pool)
    }

    fn sync_update_with_text(client_id: &str, text: &str) -> SyncUpdatePayload {
        let doc = Doc::new();
        let fragment = doc.get_or_insert_xml_fragment("content");
        let mut txn = doc.transact_mut();
        fragment.push_back(&mut txn, XmlTextPrelim::new(text));
        let update = txn.encode_state_as_update_v1(&StateVector::default());

        SyncUpdatePayload {
            client_id: client_id.to_string(),
            update: Base64.encode(update),
        }
    }

    fn snapshot_text(snapshot: &[u8]) -> String {
        let doc = Doc::new();
        let fragment = doc.get_or_insert_xml_fragment("content");
        let update = Update::decode_v1(snapshot).expect("decode snapshot");
        let mut txn = doc.transact_mut();
        txn.apply_update(update).expect("apply snapshot");
        drop(txn);

        let txn = doc.transact();
        fragment.get_string(&txn)
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
    async fn removes_empty_rooms_and_keeps_reopened_state_isolated() {
        let db = test_db().await;
        let manager = RoomManager::new(db.clone());
        let document = db
            .create_document("Room eviction", None)
            .await
            .expect("create document");

        let room = manager
            .get_or_create(&document.id)
            .await
            .expect("create first room");
        let (sender, _receiver) = mpsc::unbounded_channel();
        room.join(
            "client-1".to_string(),
            "Alice".to_string(),
            "#111111".to_string(),
            sender,
        )
        .await
        .expect("join first room");
        room.apply_update(sync_update_with_text("client-1", "first"))
            .await
            .expect("apply first update");

        manager.remove_peer(&document.id, "client-1").await;

        assert!(manager.rooms.read().await.is_empty());

        let reopened = manager
            .get_or_create(&document.id)
            .await
            .expect("recreate room after eviction");
        assert!(!Arc::ptr_eq(&room, &reopened));

        let (sender, _receiver) = mpsc::unbounded_channel();
        reopened
            .join(
                "client-2".to_string(),
                "Bob".to_string(),
                "#222222".to_string(),
                sender,
            )
            .await
            .expect("join reopened room");
        reopened
            .apply_update(sync_update_with_text("client-2", "second"))
            .await
            .expect("apply second update");

        manager.remove_peer(&document.id, "client-2").await;

        assert!(manager.rooms.read().await.is_empty());

        let expected = db
            .load_room_seed(&document.id)
            .await
            .expect("load room seed before delayed flush")
            .expect("room seed exists before delayed flush");
        let expected_text = snapshot_text(&expected.snapshot.yjs_snapshot);

        tokio::time::sleep(Duration::from_secs(3)).await;

        let seed = db
            .load_room_seed(&document.id)
            .await
            .expect("load room seed")
            .expect("room seed exists");

        assert_eq!(snapshot_text(&seed.snapshot.yjs_snapshot), expected_text);
    }

    #[tokio::test]
    async fn delete_room_closes_peers_and_removes_room_without_persisting() {
        let db = test_db().await;
        let manager = RoomManager::new(db.clone());
        let document = db
            .create_document("Delete room", None)
            .await
            .expect("create document");

        let room = manager
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

        manager.delete_room(&document.id).await;

        assert!(manager.current_snapshot(&document.id).await.is_none());
        expect_close(&mut receiver).await;
    }
}
