use axum_tws::Message;
use tracing::debug;

use crate::{
    constants::{CANVAS_HEIGHT, CANVAS_WIDTH, PIXEL_PAYLOAD_SIZE, message_types},
    protocol::{PROTOCOL_VERSION, WsMessage, encode_ws_message},
};

/// creates a random rgb value
pub fn create_random_rgb() -> [u8; 3] {
    let r = rand::random_range(0..255);
    let g = rand::random_range(0..255);
    let b = rand::random_range(0..255);

    return [r, g, b];
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

pub fn create_frame_message(frame_data: Vec<u8>) -> Message {
    let expected_size = (CANVAS_WIDTH as usize) * (CANVAS_HEIGHT as usize) * 3;
    if frame_data.len() != expected_size {
        panic!(
            "Frame data size mismatch: got {} bytes, expected {} bytes for {}x{} RGB canvas",
            frame_data.len(),
            expected_size,
            CANVAS_WIDTH,
            CANVAS_HEIGHT
        );
    }

    // Frame payload format:
    // - 2 bytes: canvas width (big-endian)
    // - 2 bytes: canvas height (big-endian)
    // - N bytes: RGB pixel data (width * height * 3 bytes)
    let mut payload = Vec::with_capacity(4 + frame_data.len());
    payload.extend_from_slice(&CANVAS_WIDTH.to_be_bytes());
    payload.extend_from_slice(&CANVAS_HEIGHT.to_be_bytes());
    payload.extend_from_slice(&frame_data);

    debug!(
        "Created frame message: {}x{} canvas, {} total bytes",
        CANVAS_WIDTH,
        CANVAS_HEIGHT,
        payload.len()
    );

    let msg = WsMessage {
        version: PROTOCOL_VERSION,
        msg_type: message_types::DRAW_FRAME,
        flags: 0,
        payload,
    };
    encode_ws_message(&msg)
}
