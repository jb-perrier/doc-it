use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ClientMessage {
    Join(JoinPayload),
    SyncUpdate(SyncUpdatePayload),
    PresenceUpdate(PresenceUpdatePayload),
    Heartbeat(HeartbeatPayload),
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ServerMessage {
    Joined(JoinedPayload),
    SyncInit(SyncInitPayload),
    SyncUpdate(SyncUpdatePayload),
    PresenceSnapshot(PresenceSnapshotPayload),
    Error(ErrorPayload),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinPayload {
    pub client_id: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncUpdatePayload {
    pub client_id: String,
    pub update: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceUpdatePayload {
    pub client_id: String,
    pub anchor: Option<u32>,
    pub head: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatPayload {
    pub client_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinedPayload {
    pub document_id: String,
    pub server_time: String,
    pub peers: Vec<PeerPresence>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncInitPayload {
    pub update: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PresenceSnapshotPayload {
    pub peers: Vec<PeerPresence>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorPayload {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerPresence {
    pub client_id: String,
    pub name: String,
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<u32>,
}