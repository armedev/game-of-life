use std::sync::Arc;

use axum_tws::Message;
use tokio::sync::{Mutex, broadcast};

pub struct AppState {
    pub channel: broadcast::Sender<Message>,
    pub messages: Arc<Mutex<Vec<Message>>>,
}

impl AppState {
    pub fn new(channel_cap: usize, messages_cap: usize) -> AppState {
        let channel = broadcast::Sender::<Message>::new(channel_cap);
        let messages = Arc::new(Mutex::new(Vec::with_capacity(messages_cap)));

        AppState { channel, messages }
    }
}
