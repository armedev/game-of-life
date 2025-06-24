use std::sync::Arc;

use axum_tws::Message;
use tokio::sync::{Mutex, broadcast};
use tracing::info;

#[derive(Debug)]
pub struct AppState {
    pub channel: broadcast::Sender<Message>,
    pub messages: Arc<Mutex<Vec<Message>>>,
}

impl AppState {
    pub fn new(channel_cap: usize, messages_cap: usize) -> AppState {
        let channel = broadcast::Sender::<Message>::new(channel_cap);
        let messages = Arc::new(Mutex::new(Vec::with_capacity(messages_cap)));

        info!(
            "Created AppState with channel capacity: {}, message buffer capacity: {}",
            channel_cap, messages_cap
        );

        AppState { channel, messages }
    }

    #[allow(dead_code)]
    pub async fn get_stats(&self) -> (usize, usize) {
        let messages = self.messages.lock().await;
        let stored_count = messages.len();
        let receiver_count = self.channel.receiver_count();
        (stored_count, receiver_count)
    }
}
