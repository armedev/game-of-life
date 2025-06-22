use axum_tws::{Message, WebSocket};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};

use crate::{
    payload::{WsPayload, message_types},
    protocol::decode_ws_message,
    state::AppState,
};

async fn handle_receive_from_channel(
    mut channel_receiver: broadcast::Receiver<Message>,
    mut socket_sender: SplitSink<WebSocket, Message>,
) {
    while let Ok(msg) = channel_receiver.recv().await {
        if socket_sender.send(msg).await.is_err() {
            break;
        }
    }
}

async fn handle_send_to_channel(
    mut socket_receiver: SplitStream<WebSocket>,
    channel_sender: broadcast::Sender<Message>,
    state: Arc<AppState>,
) {
    while let Some(Ok(msg)) = socket_receiver.next().await {
        if msg.is_binary() {
            let data = msg.into_payload();
            match decode_ws_message(data) {
                Ok(parsed) => {
                    let message_type = parsed.msg_type.clone();
                    let payload = WsPayload { parsed };
                    let encoded = payload.handle_payload();
                    if message_type == message_types::SEND_PIXEL {
                        state.messages.lock().await.push(encoded.clone());
                    }
                    if channel_sender.send(encoded).is_err() {
                        break;
                    }
                }
                Err(err) => {
                    eprintln!("Failed to decode: {err}");
                }
            }
        } else if msg.is_text() {
            if channel_sender
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
}

async fn handle_previous_messages_from_state(
    stored_messages: Arc<Mutex<Vec<Message>>>,
    sink: &mut SplitSink<WebSocket, Message>,
) {
    let messages = {
        let locked_messages = stored_messages.lock().await;
        locked_messages.clone()
    };

    for message in messages {
        if sink.send(message).await.is_err() {
            return;
        };
    }
}

pub async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sink, stream) = socket.split();
    let channel = state.channel.clone();

    handle_previous_messages_from_state(state.messages.clone(), &mut sink).await;

    let mut send_task = tokio::spawn(handle_receive_from_channel(channel.subscribe(), sink));

    let mut recv_task = tokio::spawn(handle_send_to_channel(stream, channel, state));

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    };
}
