pub const CANVAS_WIDTH: u16 = 100;
pub const CANVAS_HEIGHT: u16 = 100;
pub const PIXEL_PAYLOAD_SIZE: usize = 7;
pub const HELLO_PAYLOAD: &[u8] = b"hello";
#[allow(dead_code)]
pub const LIVE_CELL_R_G_B: [u8; 3] = [0, 0, 0];
pub const DEAD_CELL_R_G_B: [u8; 3] = [255, 255, 255];

pub mod message_types {
    pub const HELLO: u8 = 1;

    pub const CREATE_NEW_GOL_GENERATION: u8 = 40;
    pub const AWAKEN_RANDOM_GOL_CELL: u8 = 41;
    pub const KILL_RANDOM_GOL_CELL: u8 = 42;
    pub const ADVANCE_GOL_GENERATION: u8 = 43;
    pub const KILL_ALL_GOL_CELLS: u8 = 45;

    pub const CREATE_NEW_MLP_PAINTING: u8 = 20;
    pub const ADVANCE_MLP_PAINTING: u8 = 21;

    pub const REQUEST_RANDOM_COLORED_PIXEL: u8 = 200;

    pub const DRAW_PIXEL: u8 = 100;
    pub const DRAW_FRAME: u8 = 101;
}
