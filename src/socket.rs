use axum_tws::WebSocket;
use futures::StreamExt;
use std::sync::Arc;
use tracing::{Span, debug, error, info, instrument};
use uuid::Uuid;

use crate::{message::SocketHandler, state::AppState};

#[instrument(skip(socket, state), fields(connection_id = %Uuid::new_v4()))]
pub async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let connection_id = Span::current().field("connection_id").unwrap();
    info!("New WebSocket connection established");

    let (mut sink, stream) = socket.split();
    let handler = SocketHandler::new(state, connection_id.to_string());

    // Send stored messages first
    match handler.send_current_generation(&mut sink).await {
        Ok(_) => {
            debug!("Successfully sent stored messages to new connection");
            // Run the main handler
        }
        Err(e) => {
            error!("Failed to send stored messages to new connection: {}", e);
            return;
        }
    }
    handler.run(stream, sink).await;

    info!("WebSocket connection terminated");
}
