use axum_tws::Message;
use rand::Rng;

use crate::protocol::{PROTOCOL_VERSION, WsMessage, encode_ws_message};

pub struct WsPayload {
    pub parsed: WsMessage,
}

pub fn get_dummy_payload() -> Message {
    let response = WsMessage {
        version: PROTOCOL_VERSION,
        msg_type: 1,
        flags: 0,
        payload: b"hello".to_vec(),
    };

    let encoded = encode_ws_message(&response);
    return encoded;
}

impl WsPayload {
    pub fn handle_payload(&self) -> Message {
        println!("Received: {:?}", self.parsed);
        if self.parsed.msg_type == 42 {
            let encoded = create_binary_payload();
            return encoded;
        } else {
            let response = WsMessage {
                version: PROTOCOL_VERSION,
                msg_type: self.parsed.msg_type,
                flags: 0,
                payload: self.parsed.payload.clone(),
            };

            let encoded = encode_ws_message(&response);
            return encoded;
        }
    }
}

pub fn create_binary_payload() -> Message {
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
    encoded
}
