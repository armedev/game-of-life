use crate::protocol::{PROTOCOL_VERSION, WsMessage, encode_ws_message};
use axum_tws::Message;
use rand::Rng;
use tracing::{debug, warn};

pub struct WsPayload {
    pub parsed: WsMessage,
}

const CANVAS_WIDTH: u16 = 40;
const CANVAS_HEIGHT: u16 = 40;
const PIXEL_PAYLOAD_SIZE: usize = 7;
const HELLO_PAYLOAD: &[u8] = b"hello";

pub mod message_types {
    pub const HELLO: u8 = 1;
    pub const SEND_PIXEL: u8 = 42;
    pub const DRAW_PIXEL: u8 = 100;
}

#[allow(dead_code)]
pub fn get_dummy_payload() -> Message {
    let response = WsMessage {
        version: PROTOCOL_VERSION,
        msg_type: message_types::HELLO,
        flags: 0,
        payload: HELLO_PAYLOAD.to_vec(),
    };
    encode_ws_message(&response)
}

impl WsPayload {
    pub fn handle_payload(&self) -> Message {
        debug!(
            "Processing payload - Type: {}, Size: {} bytes",
            self.parsed.msg_type,
            self.parsed.payload.len()
        );

        match self.parsed.msg_type {
            message_types::SEND_PIXEL => {
                debug!("Generating pixel response for SEND_PIXEL message");
                create_binary_payload()
            }
            message_types::HELLO => {
                debug!("Processing HELLO message");
                self.create_echo_response()
            }
            unknown_type => {
                warn!("Unknown message type: {}, echoing back", unknown_type);
                self.create_echo_response()
            }
        }
    }

    fn create_echo_response(&self) -> Message {
        let response = WsMessage {
            version: PROTOCOL_VERSION,
            msg_type: self.parsed.msg_type,
            flags: 0,
            payload: self.parsed.payload.clone(),
        };
        encode_ws_message(&response)
    }
}

pub fn create_binary_payload() -> Message {
    create_random_pixel_message()
}

pub fn create_random_pixel_message() -> Message {
    let mut rng = rand::rng();

    let x: u16 = rng.random_range(0..CANVAS_WIDTH);
    let y: u16 = rng.random_range(0..CANVAS_HEIGHT);
    let r: u8 = rng.random();
    let g: u8 = rng.random();
    let b: u8 = rng.random();

    debug!(
        "Generated random pixel: ({}, {}) RGB({}, {}, {})",
        x, y, r, g, b
    );
    create_pixel_message(x, y, r, g, b)
}

pub fn create_pixel_message(x: u16, y: u16, r: u8, g: u8, b: u8) -> Message {
    if x >= CANVAS_WIDTH || y >= CANVAS_HEIGHT {
        panic!(
            "Pixel coordinates out of bounds: ({}, {}) max: ({}, {})",
            x,
            y,
            CANVAS_WIDTH - 1,
            CANVAS_HEIGHT - 1
        );
    }

    let mut payload = Vec::with_capacity(PIXEL_PAYLOAD_SIZE);
    payload.extend_from_slice(&x.to_be_bytes());
    payload.extend_from_slice(&y.to_be_bytes());
    payload.push(r);
    payload.push(g);
    payload.push(b);

    let msg = WsMessage {
        version: PROTOCOL_VERSION,
        msg_type: message_types::DRAW_PIXEL,
        flags: 0,
        payload,
    };

    encode_ws_message(&msg)
}
