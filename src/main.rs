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

use crate::payload::create_binary_payload;
use crate::socket::handle_socket;
use crate::state::AppState;

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    let app_state = Arc::new(AppState::new(100, 1600));

    let channel = app_state.channel.clone();

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state)
        .fallback_service(axum_static::static_router("static"));

    thread::spawn(move || {
        let mut target_dt = Utc::now();
        loop {
            target_dt = target_dt
                .checked_add_signed(Duration::milliseconds(100))
                .unwrap();
            let diff = (target_dt - Utc::now()).to_std().unwrap();
            thread::sleep(diff);
            if channel.receiver_count() > 0 {
                if let Err(e) = channel.send(create_binary_payload()) {
                    dbg!(e);
                    break;
                }
            }
        }
    });

    println!("server running at {addr}");
    axum::serve(listener, app).await.unwrap();
}

// fn get_prev_10_min_dt(current_dt: DateTime<Utc>) -> DateTime<Utc> {
//     let target_dt = current_dt.checked_add_signed(Duration::seconds(1)).unwrap();
//     // Round to 10 min multiple
//     let target_minute = (target_dt.minute() / 10) * 10;
//
//     let target_dt = target_dt.with_minute(target_minute).unwrap();
//     target_dt.with_second(0).unwrap()
// }
