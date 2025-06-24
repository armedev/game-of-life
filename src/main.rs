mod message;
mod payload;
mod protocol;
mod socket;
mod state;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use axum_tws::WebSocketUpgrade;
use chrono::{Duration, Utc};
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::payload::create_binary_payload;
use crate::socket::handle_socket;
use crate::state::AppState;

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    info!("New WebSocket connection attempt");
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or("info,websocket_server=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting WebSocket server");

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
        error!("Failed to bind to address {}: {}", addr, e);
        e
    })?;

    let app_state = Arc::new(AppState::new(100, 1600));
    info!("Application state initialized");

    let channel = app_state.channel.clone();

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state)
        .fallback_service(axum_static::static_router("static"));

    // Spawn background task for periodic message generation
    let broadcast_handle = thread::spawn(move || {
        info!("Starting periodic message broadcaster");
        let mut target_dt = Utc::now();
        let mut consecutive_errors = 0;
        const MAX_CONSECUTIVE_ERRORS: u32 = 10;

        loop {
            target_dt = target_dt
                .checked_add_signed(Duration::milliseconds(100))
                .unwrap();

            let diff = match (target_dt - Utc::now()).to_std() {
                Ok(duration) => duration,
                Err(e) => {
                    warn!("Time calculation error: {}, using 100ms default", e);
                    std::time::Duration::from_millis(100)
                }
            };

            thread::sleep(diff);

            if channel.receiver_count() > 0 {
                match channel.send(create_binary_payload()) {
                    Ok(_) => {
                        consecutive_errors = 0;
                        debug!(
                            "Broadcasted message to {} receivers",
                            channel.receiver_count()
                        );
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        error!(
                            "Failed to broadcast message (attempt {}): {}",
                            consecutive_errors, e
                        );

                        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                            error!(
                                "Too many consecutive broadcast errors, shutting down broadcaster"
                            );
                            break;
                        }
                    }
                }
            } else {
                trace!("No active receivers, skipping broadcast");
            }
        }

        warn!("Periodic message broadcaster shutting down");
    });

    info!("Server running at {}", addr);

    let server_result = axum::serve(listener, app).await;

    // Cleanup
    warn!("Server shutting down");
    if broadcast_handle.join().is_err() {
        error!("Failed to cleanly shutdown broadcast thread");
    }

    server_result.map_err(|e| {
        error!("Server error: {}", e);
        e.into()
    })
}
