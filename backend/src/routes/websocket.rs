use std::sync::Arc;

use axum::{
    Router,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use futures::{
    sink::SinkExt,
    stream::{SplitSink, StreamExt},
};
use tokio::sync::mpsc;

use crate::{
    app_state::AppState,
    models::{
        api::AppError,
        ws::{ClientMessage, ErrorPayload, ServerMessage},
    },
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

fn serialize_server_message(message: &ServerMessage) -> Message {
    Message::Text(serde_json::to_string(message).unwrap_or_default().into())
}

async fn send_handshake_error(sender: &mut SplitSink<WebSocket, Message>, message: &'static str) {
    let _ = sender
        .send(serialize_server_message(&ServerMessage::Error(
            ErrorPayload {
                code: "invalid_message",
                message,
            },
        )))
        .await;
}

async fn handle_socket(socket: WebSocket, room: Arc<crate::realtime::rooms::Room>) {
    let (mut sender, mut receiver) = socket.split();

    let Some(Ok(Message::Text(message))) = receiver.next().await else {
        return;
    };

    let join = match serde_json::from_str::<ClientMessage>(&message) {
        Ok(ClientMessage::Join(join)) => join,
        Ok(_) => {
            send_handshake_error(&mut sender, "Join must be the first websocket message").await;
            return;
        }
        Err(_) => {
            send_handshake_error(&mut sender, "Unsupported websocket message").await;
            return;
        }
    };

    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<Message>();

    let outgoing_task = tokio::spawn(async move {
        while let Some(message) = outgoing_rx.recv().await {
            if sender.send(message).await.is_err() {
                break;
            }
        }
    });

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
                let _ = outgoing_tx.send(serialize_server_message(&message));
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

#[cfg(test)]
mod tests {
    use super::router;
    use std::sync::Arc;

    use axum::Router;
    use futures::{SinkExt, StreamExt};
    use sqlx::sqlite::SqlitePoolOptions;
    use tokio::net::TcpListener;
    use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};

    use crate::{
        app_state::AppState,
        db::{Database, migrations::run_migrations},
        realtime::rooms::RoomManager,
    };

    async fn test_app() -> (Router, String) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect in-memory sqlite");
        run_migrations(&pool).await.expect("run migrations");

        let db = Database::new(pool);
        let document = db
            .create_document("Handshake test")
            .await
            .expect("create document");
        let rooms = RoomManager::new(db.clone());
        let state = Arc::new(AppState::new(db, rooms));

        (router().with_state(state), document.id)
    }

    #[tokio::test]
    async fn invalid_first_message_returns_protocol_error() {
        let (app, document_id) = test_app().await;
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test listener");
        let address = listener.local_addr().expect("read test listener address");

        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("serve websocket test app");
        });

        let url = format!("ws://{address}/documents/{document_id}/live");
        let (mut socket, _) = connect_async(url).await.expect("connect websocket client");

        socket
            .send(TungsteniteMessage::Text(
                r#"{"type":"heartbeat","payload":{"clientId":"client-1"}}"#
                    .to_string()
                    .into(),
            ))
            .await
            .expect("send invalid first message");

        let response = socket
            .next()
            .await
            .expect("receive server response")
            .expect("receive websocket frame");
        let text = response.into_text().expect("receive text error frame");
        let payload: serde_json::Value =
            serde_json::from_str(text.as_ref()).expect("parse error payload");

        assert_eq!(payload["type"], "error");
        assert_eq!(payload["payload"]["code"], "invalid_message");
        assert_eq!(
            payload["payload"]["message"],
            "Join must be the first websocket message"
        );

        server.abort();
    }
}
