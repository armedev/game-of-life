mod payload;
mod protocol;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use axum_tws::{Message, WebSocket, WebSocketUpgrade};
use chrono::{Duration, Utc};
use futures::SinkExt;
use futures::StreamExt;
use protocol::decode_ws_message;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use tokio::sync::{RwLock, broadcast};

use crate::payload::{WsPayload, create_binary_payload};

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sink, mut stream) = socket.split();

    let mut rx = state.tx.subscribe();

    let messages = state.messages.clone();

    for message in messages.read().await.iter() {
        if sink.send(message.clone()).await.is_err() {
            return;
        };
    }

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    let tx = state.tx.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            if msg.is_binary() {
                let data = msg.into_payload();
                match decode_ws_message(data) {
                    Ok(parsed) => {
                        let message_type = parsed.msg_type.clone();
                        let payload = WsPayload { parsed };
                        let encoded = payload.handle_payload();
                        if message_type == 42 {
                            messages.write().await.push(encoded.clone());
                        }
                        if tx.send(encoded).is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        eprintln!("Failed to decode: {err}");
                    }
                }
            } else if msg.is_text() {
                if tx
                    .send(Message::text("Only binary message is supported"))
                    .is_err()
                {
                    break;
                }

                eprintln!("x: {:?}", msg.into_payload());
                eprintln!("Only binary message is suuported");
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    };
}

struct AppState {
    tx: broadcast::Sender<Message>,
    messages: Arc<RwLock<Vec<Message>>>,
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    let tx = broadcast::Sender::<Message>::new(100);
    let messages = Arc::new(RwLock::new(Vec::with_capacity(1600)));

    let app_state = Arc::new(AppState {
        tx: tx.clone(),
        messages,
    });

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
            if tx.receiver_count() > 0 {
                if let Err(e) = tx.send(create_binary_payload()) {
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
