use crate::{
    constants::{CANVAS_WIDTH, HELLO_PAYLOAD, message_types},
    patterns::{gol, mlp},
    protocol::{PROTOCOL_VERSION, WsMessage, encode_ws_message},
};
use axum_tws::Message;
use rand::Rng;
use tracing::{debug, warn};

pub struct WsPayload {
    pub parsed: WsMessage,
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
            message_types::CREATE_NEW_GOL_GENERATION => {
                debug!("GOL: Creating a new generation");
                gol::create_new_generation()
            }
            message_types::AWAKEN_RANDOM_GOL_CELL => {
                debug!("GOL: Adding a random live cell to current generation");
                gol::awaken_random_cell()
            }
            message_types::KILL_RANDOM_GOL_CELL => {
                debug!("GOL: Killing a random cell of current generation");
                gol::kill_random_cell()
            }
            message_types::ADVANCE_GOL_GENERATION => {
                debug!("GOL: Advancing to next generation");
                gol::advance_generation()
            }
            message_types::KILL_ALL_GOL_CELLS => {
                debug!("GOL: Killing all the cells");
                gol::kill_all_cells()
            }
            message_types::CREATE_NEW_MLP_PAINTING => {
                debug!("MLP: Creating new painting canvas");
                mlp::start_new_painting()
            }
            message_types::ADVANCE_MLP_PAINTING => {
                let mut rng = rand::rng();
                debug!("MLP: Advancing to next stroke");
                mlp::apply_brush_strokes_batch(rng.random_range(0..CANVAS_WIDTH as usize))
            }
            message_types::REQUEST_RANDOM_COLORED_PIXEL => {
                let x = self.parsed.payload[0];
                let y = self.parsed.payload[1];
                debug!("GOL: Adding a live cell to current generation");
                gol::awaken_cell(x as u16, y as u16)
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
