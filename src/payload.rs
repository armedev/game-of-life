use crate::protocol::{PROTOCOL_VERSION, WsMessage, encode_ws_message};
use axum_tws::Message;
use rand::Rng;

pub struct WsPayload {
    pub parsed: WsMessage,
}

// Constants for better maintainability
const CANVAS_WIDTH: u16 = 40;
const CANVAS_HEIGHT: u16 = 40;
const PIXEL_PAYLOAD_SIZE: usize = 7; // 2 + 2 + 1 + 1 + 1 bytes
const HELLO_PAYLOAD: &[u8] = b"hello";

// Message types as constants for clarity
pub mod message_types {
    pub const HELLO: u8 = 1;
    pub const SEND_PIXEL: u8 = 42;
    pub const DRAW_PIXEL: u8 = 100;
}

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
        println!(
            "Received message - Type: {}, Payload size: {} bytes",
            self.parsed.msg_type,
            self.parsed.payload.len()
        );

        match self.parsed.msg_type {
            message_types::SEND_PIXEL => {
                // Generate a random pixel when receiving type 42
                create_binary_payload()
            }
            _ => {
                // Echo back the received message with same type
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

    create_pixel_message(x, y, r, g, b)
}

pub fn create_pixel_message(x: u16, y: u16, r: u8, g: u8, b: u8) -> Message {
    // Validate coordinates
    if x >= CANVAS_WIDTH || y >= CANVAS_HEIGHT {
        panic!(
            "Pixel coordinates out of bounds: ({}, {}) max: ({}, {})",
            x,
            y,
            CANVAS_WIDTH - 1,
            CANVAS_HEIGHT - 1
        );
    }

    // Pre-allocate exact size needed
    let mut payload = Vec::with_capacity(PIXEL_PAYLOAD_SIZE);

    // Pack pixel data in big-endian format
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
//
// // Utility functions for pixel manipulation
// pub fn create_colored_pixel(x: u16, y: u16, color: (u8, u8, u8)) -> Message {
//     create_pixel_message(x, y, color.0, color.1, color.2)
// }
//
// pub fn create_grayscale_pixel(x: u16, y: u16, intensity: u8) -> Message {
//     create_pixel_message(x, y, intensity, intensity, intensity)
// }
//
// // Batch pixel operations for efficiency
// pub fn create_pixel_batch(pixels: &[(u16, u16, u8, u8, u8)]) -> Vec<Message> {
//     pixels
//         .iter()
//         .map(|&(x, y, r, g, b)| create_pixel_message(x, y, r, g, b))
//         .collect()
// }
//
// // Common color constants
// pub mod colors {
//     pub const RED: (u8, u8, u8) = (255, 0, 0);
//     pub const GREEN: (u8, u8, u8) = (0, 255, 0);
//     pub const BLUE: (u8, u8, u8) = (0, 0, 255);
//     pub const WHITE: (u8, u8, u8) = (255, 255, 255);
//     pub const BLACK: (u8, u8, u8) = (0, 0, 0);
//     pub const YELLOW: (u8, u8, u8) = (255, 255, 0);
//     pub const MAGENTA: (u8, u8, u8) = (255, 0, 255);
//     pub const CYAN: (u8, u8, u8) = (0, 255, 255);
// }
//
// // Pattern generators for testing/demo
// pub fn create_rainbow_pixel(x: u16, y: u16) -> Message {
//     // Create rainbow effect based on position
//     let hue = ((x + y) as f32 / (CANVAS_WIDTH + CANVAS_HEIGHT) as f32) * 360.0;
//     let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
//     create_pixel_message(x, y, r, g, b)
// }
//
// pub fn create_gradient_pixel(x: u16, y: u16) -> Message {
//     // Create gradient based on position
//     let r = ((x as f32 / CANVAS_WIDTH as f32) * 255.0) as u8;
//     let g = ((y as f32 / CANVAS_HEIGHT as f32) * 255.0) as u8;
//     let b = 128; // Constant blue component
//     create_pixel_message(x, y, r, g, b)
// }
//
// // Helper function to convert HSV to RGB
// fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
//     let c = v * s;
//     let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
//     let m = v - c;
//
//     let (r_prime, g_prime, b_prime) = match h as u32 {
//         0..=59 => (c, x, 0.0),
//         60..=119 => (x, c, 0.0),
//         120..=179 => (0.0, c, x),
//         180..=239 => (0.0, x, c),
//         240..=299 => (x, 0.0, c),
//         300..=359 => (c, 0.0, x),
//         _ => (0.0, 0.0, 0.0),
//     };
//
//     (
//         ((r_prime + m) * 255.0) as u8,
//         ((g_prime + m) * 255.0) as u8,
//         ((b_prime + m) * 255.0) as u8,
//     )
// }
//
// // Performance-optimized random pixel generation
// pub struct PixelGenerator {
//     rng: rand::rngs::ThreadRng,
// }
//
// impl PixelGenerator {
//     pub fn new() -> Self {
//         Self { rng: rand::rng() }
//     }
//
//     pub fn generate_pixel(&mut self) -> Message {
//         let x: u16 = self.rng.random_range(0..CANVAS_WIDTH);
//         let y: u16 = self.rng.random_range(0..CANVAS_HEIGHT);
//         let r: u8 = self.rng.random();
//         let g: u8 = self.rng.random();
//         let b: u8 = self.rng.random();
//
//         create_pixel_message(x, y, r, g, b)
//     }
//
//     pub fn generate_pixels(&mut self, count: usize) -> Vec<Message> {
//         (0..count).map(|_| self.generate_pixel()).collect()
//     }
// }
//
// impl Default for PixelGenerator {
//     fn default() -> Self {
//         Self::new()
//     }
// }
