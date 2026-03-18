use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::Response,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::mpsc;

use crate::{
    app_state::AppState,
    models::{api::AppError, ws::{ClientMessage, ServerMessage}},
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/documents/{id}/live", get(websocket_handler))
}

async fn websocket_handler(
    State(state): State<Arc<AppState>>,
    Path(document_id): Path<String>,
    ws: WebSocketUpgrade,
) -> Result<Response, AppError> {
    let room = state.rooms.get_or_create(&document_id).await?;
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, room)))
}

async fn handle_socket(socket: WebSocket, room: Arc<crate::realtime::rooms::Room>) {
    let (mut sender, mut receiver) = socket.split();
    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<Message>();

    let outgoing_task = tokio::spawn(async move {
        while let Some(message) = outgoing_rx.recv().await {
            if sender.send(message).await.is_err() {
                break;
            }
        }
    });

    let Some(Ok(Message::Text(message))) = receiver.next().await else {
        outgoing_task.abort();
        return;
    };

    let ClientMessage::Join(join) = (match serde_json::from_str::<ClientMessage>(&message) {
        Ok(message) => message,
        Err(_) => {
            let _ = outgoing_tx.send(Message::Text(
                serde_json::to_string(&ServerMessage::Error(crate::models::ws::ErrorPayload {
                    code: "invalid_message",
                    message: "Unsupported websocket message",
                }))
                .unwrap_or_default()
                .into(),
            ));
            outgoing_task.abort();
            return;
        }
    }) else {
        let _ = outgoing_tx.send(Message::Text(
            serde_json::to_string(&ServerMessage::Error(crate::models::ws::ErrorPayload {
                code: "invalid_message",
                message: "Join must be the first websocket message",
            }))
            .unwrap_or_default()
            .into(),
        ));
        outgoing_task.abort();
        return;
    };

    let client_id = join.client_id.clone();
    match room
        .join(
            join.client_id.clone(),
            join.name.clone(),
            join.color.clone(),
            outgoing_tx.clone(),
        )
        .await
    {
        Ok(context) => {
            let initial_messages = [
                ServerMessage::Joined(context.joined),
                ServerMessage::SyncInit(context.sync_init),
                ServerMessage::PresenceSnapshot(context.presence),
            ];

            for message in initial_messages {
                let _ = outgoing_tx.send(Message::Text(
                    serde_json::to_string(&message).unwrap_or_default().into(),
                ));
            }
        }
        Err(error) => {
            tracing::warn!(?error, "failed to join websocket room");
            outgoing_task.abort();
            return;
        }
    }

    while let Some(next) = receiver.next().await {
        match next {
            Ok(Message::Text(text)) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::Join(_)) => {}
                Ok(ClientMessage::SyncUpdate(payload)) => {
                    if let Err(error) = room.apply_update(payload).await {
                        tracing::warn!(?error, client_id = %client_id, "failed to apply sync update");
                    }
                }
                Ok(ClientMessage::PresenceUpdate(payload)) => room.update_presence(payload).await,
                Ok(ClientMessage::Heartbeat(payload)) => room.heartbeat(&payload.client_id).await,
                Err(error) => {
                    tracing::warn!(?error, "invalid websocket message");
                }
            },
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(bytes)) => {
                let _ = outgoing_tx.send(Message::Pong(bytes));
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(?error, client_id = %client_id, "websocket receive failed");
                break;
            }
        }
    }

    room.remove_peer(&client_id).await;
    outgoing_task.abort();
}