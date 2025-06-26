use axum_tws::Message;
use tokio::sync::broadcast;
use tracing::info;

#[derive(Debug)]
pub struct AppState {
    pub channel: broadcast::Sender<Message>,
}

impl AppState {
    pub fn new(channel_cap: usize) -> AppState {
        let channel = broadcast::Sender::<Message>::new(channel_cap);

        info!("Created AppState with channel capacity: {}", channel_cap);

        AppState { channel }
    }
}
