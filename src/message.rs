use anyhow::{Context, Result};
use axum_tws::{Message, WebSocket};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tracing::{debug, error, info, instrument, warn};

use crate::{
    constants::message_types, patterns::gol::current_generation, payload::WsPayload,
    protocol::decode_ws_message, state::AppState,
};

/// Custom error types for better error handling
#[derive(Debug, thiserror::Error)]
pub enum SocketError {
    #[error("WebSocket send error: {0}")]
    SendError(String),
    #[error("WebSocket receive error: {0}")]
    ReceiveError(String),
    #[error("Protocol decode error: {0}")]
    DecodeError(#[from] anyhow::Error),
    #[error("Broadcast channel error: {0}")]
    BroadcastError(#[from] broadcast::error::SendError<Message>),
    #[error("Connection timeout after {duration:?}")]
    Timeout { duration: Duration },
    #[error("Connection closed by client")]
    ConnectionClosed,
}

#[derive(Debug)]
pub struct SocketHandler {
    state: Arc<AppState>,
    connection_id: String,
}

impl SocketHandler {
    pub fn new(state: Arc<AppState>, connection_id: String) -> Self {
        Self {
            state,
            connection_id,
        }
    }

    #[instrument(skip(self, sink), fields(connection_id = %self.connection_id, start_time))]
    pub async fn send_current_generation(
        &self,
        sink: &mut SplitSink<WebSocket, Message>,
    ) -> Result<(), SocketError> {
        sink.send(current_generation()).await.map_err(|e| {
            SocketError::SendError(format!(
                "Failed to send current generation: connection_id: {},  {}",
                self.connection_id, e
            ))
        })?;

        debug!(
            "Successfully sent current generation to client: connection_id: {}",
            self.connection_id
        );

        Ok(())
    }

    #[instrument(skip(self, stream, sink), fields(connection_id = %self.connection_id))]
    pub async fn run(self, stream: SplitStream<WebSocket>, sink: SplitSink<WebSocket, Message>) {
        let channel = self.state.channel.clone();
        let channel_rx = channel.subscribe();

        info!("Starting WebSocket message handlers");

        // Spawn receiver task (from channel to socket)
        let recv_handler = ChannelReceiver::new(self.connection_id.clone());
        let mut recv_task = tokio::spawn(async move {
            if let Err(e) = recv_handler.run(channel_rx, sink).await {
                error!("Channel receiver error: {}", e);
            }
        });

        // Spawn sender task (from socket to channel)
        let send_handler = ChannelSender::new(self.connection_id.clone());
        let mut send_task = tokio::spawn(async move {
            if let Err(e) = send_handler.run(stream, channel).await {
                error!("Socket sender error: {}", e);
            }
        });

        // Wait for either task to complete and cleanup
        tokio::select! {
            result = &mut recv_task => {
                match result {
                    Ok(_) => debug!("Channel receiver task completed normally"),
                    Err(e) => error!("Channel receiver task panicked: {}", e),
                }
                send_task.abort();
            }
            result = &mut send_task => {
                match result {
                    Ok(_) => debug!("Socket sender task completed normally"),
                    Err(e) => error!("Socket sender task panicked: {}", e),
                }
                recv_task.abort();
            }
        }

        info!("WebSocket handler tasks terminated");
    }
}

/// Handles receiving messages from the broadcast channel and sending to socket
struct ChannelReceiver {
    connection_id: String,
    message_count: u64,
}

impl ChannelReceiver {
    fn new(connection_id: String) -> Self {
        Self {
            connection_id,
            message_count: 0,
        }
    }

    #[instrument(skip(self, channel_receiver, socket_sender), fields(connection_id = %self.connection_id))]
    async fn run(
        mut self,
        mut channel_receiver: broadcast::Receiver<Message>,
        mut socket_sender: SplitSink<WebSocket, Message>,
    ) -> Result<(), SocketError> {
        debug!("Channel receiver started");
        let mut consecutive_errors = 0;
        const MAX_CONSECUTIVE_ERRORS: u32 = 5;

        loop {
            match channel_receiver.recv().await {
                Ok(msg) => {
                    consecutive_errors = 0;
                    self.message_count += 1;

                    match socket_sender.send(msg).await {
                        Ok(_) => {
                            debug!("Sent message #{} to client", self.message_count);
                        }
                        Err(e) => {
                            warn!("Failed to send message to client: {}", e);
                            return Err(SocketError::SendError(e.to_string()));
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    consecutive_errors += 1;
                    warn!("Channel receiver lagging, skipped {} messages", skipped);

                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        return Err(SocketError::ReceiveError(format!(
                            "Too many consecutive lag events: {}",
                            consecutive_errors
                        )));
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    info!("Broadcast channel closed, terminating receiver");
                    return Err(SocketError::ConnectionClosed);
                }
            }
        }
    }
}

/// Handles receiving messages from socket and sending to broadcast channel
struct ChannelSender {
    connection_id: String,
    message_count: u64,
    last_activity: Instant,
}

impl ChannelSender {
    fn new(connection_id: String) -> Self {
        Self {
            connection_id,
            message_count: 0,
            last_activity: Instant::now(),
        }
    }

    #[instrument(skip(self, socket_receiver, channel_sender), fields(connection_id = %self.connection_id))]
    async fn run(
        mut self,
        mut socket_receiver: SplitStream<WebSocket>,
        channel_sender: broadcast::Sender<Message>,
    ) -> Result<(), SocketError> {
        debug!("Socket sender started");
        const ACTIVITY_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

        loop {
            // Check for timeout
            if self.last_activity.elapsed() > ACTIVITY_TIMEOUT {
                warn!("Connection inactive for {:?}, timing out", ACTIVITY_TIMEOUT);
                return Err(SocketError::Timeout {
                    duration: ACTIVITY_TIMEOUT,
                });
            }

            match socket_receiver.next().await {
                Some(Ok(msg)) => {
                    self.last_activity = Instant::now();
                    self.message_count += 1;

                    debug!("Received message #{} from client", self.message_count);

                    if msg.is_binary() {
                        self.handle_binary_message(msg, &channel_sender).await?;
                    } else if msg.is_text() {
                        self.handle_text_message(msg, &channel_sender).await?;
                    } else {
                        debug!("Received non-text/binary message (ping/pong/close)");
                    }
                }
                Some(Err(e)) => {
                    error!("WebSocket receive error: {}", e);
                    return Err(SocketError::ReceiveError(e.to_string()));
                }
                None => {
                    info!("WebSocket stream ended (client disconnected)");
                    return Err(SocketError::ConnectionClosed);
                }
            }
        }
    }

    #[instrument(skip(self, msg, channel_sender), fields(connection_id = %self.connection_id))]
    async fn handle_binary_message(
        &self,
        msg: Message,
        channel_sender: &broadcast::Sender<Message>,
    ) -> Result<(), SocketError> {
        let data = msg.into_payload();
        let data_len = data.len();

        match decode_ws_message(data) {
            Ok(parsed) => {
                let message_type = parsed.msg_type;
                debug!(
                    "Decoded binary message: type={}, payload_len={}",
                    message_type,
                    parsed.payload.len()
                );

                let payload = WsPayload { parsed };
                let encoded = payload.handle_payload();

                // Broadcast to all connected clients
                channel_sender
                    .send(encoded)
                    .context("Failed to broadcast message")?;

                let msg_type_name = match message_type {
                    t if t == message_types::CREATE_NEW_GOL_GENERATION => {
                        "CREATE_NEW_GOL_GENERATION"
                    }
                    t if t == message_types::AWAKEN_RANDOM_GOL_CELL => "AWAKEN_RANDOM_GOL_CELL",
                    t if t == message_types::KILL_RANDOM_GOL_CELL => "KILL_RANDOM_GOL_CELL",
                    t if t == message_types::ADVANCE_GOL_GENERATION => "AWAKEN_RANDOM_GOL_CELL",

                    t if t == message_types::CREATE_NEW_MLP_PAINTING => "CREATE_NEW_MLP_PAINTING",
                    t if t == message_types::ADVANCE_MLP_PAINTING => "ADVANCE_MLP_PAINTING",

                    _ => "OTHER",
                };
                debug!(
                    "Successfully processed and broadcasted {} message",
                    msg_type_name
                );
            }
            Err(err) => {
                error!(
                    "Failed to decode binary message (len={}): {}",
                    data_len, err
                );
                return Err(SocketError::DecodeError(err));
            }
        }
        Ok(())
    }

    #[instrument(skip(self, msg, channel_sender), fields(connection_id = %self.connection_id))]
    async fn handle_text_message(
        &self,
        msg: Message,
        channel_sender: &broadcast::Sender<Message>,
    ) -> Result<(), SocketError> {
        let payload = msg.into_payload();
        warn!(
            "Received unsupported text message: {:?}",
            String::from_utf8_lossy(&payload)
                .chars()
                .take(100)
                .collect::<String>()
        );

        let error_msg = Message::text("Only binary messages are supported");
        channel_sender
            .send(error_msg)
            .context("Failed to send error message")?;

        Ok(())
    }
}
