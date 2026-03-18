mod app_state;
mod db;
mod models;
mod realtime;
mod routes;

use std::{env, net::SocketAddr, str::FromStr, sync::Arc};

use app_state::AppState;
use axum::{routing::get, Router};
use db::{migrations::run_migrations, Database};
use routes::{documents, websocket};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "docit=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = env::var("DOCIT_DB").unwrap_or_else(|_| "sqlite://docit.db".to_string());
    let port = env::var("DOCIT_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3001);

    let connect_options = SqliteConnectOptions::from_str(&database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(connect_options)
        .await?;

    run_migrations(&pool).await?;

    let db = Database::new(pool);
    let rooms = realtime::rooms::RoomManager::new(db.clone());
    rooms.spawn_presence_reaper();

    let app_state = Arc::new(AppState::new(db, rooms));

    let api = Router::new()
        .merge(documents::router())
        .merge(websocket::router());

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .nest("/api", api)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(address).await?;

    info!("backend listening on http://{}", address);
    axum::serve(listener, app).await?;
    Ok(())
}
