mod protocol;

use axum::{Router, routing::get};
use axum_tws::{Message, WebSocket, WebSocketUpgrade};
use futures::StreamExt;
use protocol::{PROTOCOL_VERSION, WsMessage, decode_ws_message, encode_ws_message};
use rand::Rng;
use std::net::SocketAddr;

async fn ws_handler(ws: WebSocketUpgrade) -> axum::response::Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.next().await {
        if msg.is_binary() {
            let data = msg.into_payload();
            match decode_ws_message(data) {
                Ok(parsed) => {
                    println!("Received: {:?}", parsed);

                    if parsed.msg_type == 42 {
                        let x: u16 = rand::rng().random_range(0..40);
                        let y: u16 = rand::rng().random_range(0..40);
                        let r: u8 = rand::rng().random_range(0..=255);
                        let g: u8 = rand::rng().random_range(0..=255);
                        let b: u8 = rand::rng().random_range(0..=255);

                        let mut payload = Vec::with_capacity(7);
                        payload.extend_from_slice(&x.to_be_bytes());
                        payload.extend_from_slice(&y.to_be_bytes());
                        payload.push(r);
                        payload.push(g);
                        payload.push(b);

                        let msg = WsMessage {
                            version: PROTOCOL_VERSION,
                            msg_type: 100, // draw pixel
                            flags: 0,
                            payload,
                        };

                        let encoded = encode_ws_message(&msg);
                        if socket.send(encoded).await.is_err() {
                            break;
                        }
                    } else {
                        let response = WsMessage {
                            version: PROTOCOL_VERSION,
                            msg_type: parsed.msg_type,
                            flags: 0,
                            payload: parsed.payload,
                        };

                        let encoded = encode_ws_message(&response);
                        if socket.send(encoded).await.is_err() {
                            break;
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Failed to decode: {err}");
                }
            }
        } else {
            if socket
                .send(Message::text("Only binary message is supported"))
                .await
                .is_err()
            {
                break;
            }
            eprintln!("Only binary message is suuported");
            break;
        }
    }
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(axum_static::static_router("static"));

    println!("server running at {addr}");
    axum::serve(listener, app).await.unwrap();
}
