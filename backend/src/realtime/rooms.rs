use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use axum::extract::ws::Message;
use base64::{engine::general_purpose::STANDARD as Base64, Engine};
use chrono::Utc;
use tokio::sync::{mpsc::UnboundedSender, Mutex, RwLock};
use yrs::{updates::decoder::Decode, Doc, ReadTxn, StateVector, Transact, Update};

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

    async fn expire_stale_peers(&self) {
        let rooms = self.rooms.read().await;
        let list = rooms.values().cloned().collect::<Vec<_>>();
        drop(rooms);

        for room in list {
            room.expire_stale_peers().await;
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
        let update = Update::decode_v1(&seed.snapshot.yjs_snapshot)
            .map_err(|_| AppError::Internal)?;
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
        self.schedule_persist(false);
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
            self.flush(true).await;
        }
    }

    fn schedule_persist(self: &Arc<Self>, force_snapshot: bool) {
        let ticket = self.save_ticket.fetch_add(1, Ordering::SeqCst) + 1;
        let room = self.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if room.save_ticket.load(Ordering::SeqCst) == ticket {
                room.flush(force_snapshot).await;
            }
        });
    }

    async fn flush(&self, force_snapshot: bool) {
        let _ = force_snapshot;
        let snapshot_bytes = self.full_update_bytes().await;

        match self
            .db
            .persist_room_state(&self.document_id, &snapshot_bytes)
            .await
        {
            Ok(true) => {}
            Ok(false) => {}
            Err(error) => tracing::error!(?error, document_id = %self.document_id, "failed to persist room state"),
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
        self.broadcast(&ServerMessage::PresenceSnapshot(payload)).await;
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