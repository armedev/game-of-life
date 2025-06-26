use crate::{
    constants::{CANVAS_HEIGHT, CANVAS_WIDTH, DEAD_CELL_R_G_B},
    patterns::gol_threads::GameOfLifeVecs,
    utils::{create_frame_message, create_pixel_message, create_random_rgb},
};
use axum_tws::Message;
use once_cell::sync::Lazy;
use std::sync::RwLock;
use tracing::debug;

// Global Game of Life state
static GAME_STATE: Lazy<RwLock<GameOfLifeVecs>> =
    Lazy::new(|| RwLock::new(GameOfLifeVecs::new(CANVAS_WIDTH, CANVAS_HEIGHT)));

pub fn current_generation() -> Message {
    let game_state = GAME_STATE.read().unwrap();
    let frame_data = game_state.to_rgb_data();

    create_frame_message(frame_data)
}

pub fn awaken_random_cell() -> Message {
    let (x, y) = { GAME_STATE.write().unwrap().awaken_random_cell() };

    debug!(
        "Added a random live cell to current generation, x:{}, y:{}, generation_count:{}",
        x,
        y,
        GAME_STATE.read().unwrap().generation_count
    );

    let [r, g, b] = create_random_rgb();

    create_pixel_message(x, y, r, g, b)
}

pub fn awaken_cell(x: u16, y: u16) -> Message {
    {
        GAME_STATE.write().unwrap().awaken_cell_in(x, y)
    };

    debug!(
        "Added a live cell to current generation, x:{}, y:{}, generation_count:{}",
        x,
        y,
        GAME_STATE.read().unwrap().generation_count
    );

    let [r, g, b] = create_random_rgb();

    create_pixel_message(x, y, r, g, b)
}

pub fn kill_random_cell() -> Message {
    let (x, y) = { GAME_STATE.write().unwrap().kill_random_cell() };

    debug!(
        "Killed a random live cell of current generation, x:{}, y:{}, generation_count:{}",
        x,
        y,
        GAME_STATE.read().unwrap().generation_count
    );

    create_pixel_message(
        x,
        y,
        DEAD_CELL_R_G_B[0],
        DEAD_CELL_R_G_B[1],
        DEAD_CELL_R_G_B[2],
    )
}

pub fn kill_all_cells() -> Message {
    {
        GAME_STATE.write().unwrap().kill_all_cells()
    };

    // Convert current state to RGB data
    let game_state = GAME_STATE.read().unwrap();
    let frame_data = game_state.to_rgb_data();

    debug!(
        "Killed all cells: current generation {}, {}x{} pixels ({} bytes)",
        game_state.generation_count,
        CANVAS_WIDTH,
        CANVAS_HEIGHT,
        frame_data.len()
    );

    create_frame_message(frame_data)
}

pub fn create_new_generation() -> Message {
    reset_game_of_life_random();
    let game_state = GAME_STATE.read().unwrap();
    let frame_data = game_state.to_rgb_data();

    debug!(
        "Generated Game of Life frame: generation {}, {}x{} pixels ({} bytes)",
        game_state.generation_count,
        CANVAS_WIDTH,
        CANVAS_HEIGHT,
        frame_data.len()
    );

    create_frame_message(frame_data)
}

pub fn advance_generation() -> Message {
    {
        // Advance the game by one generation
        GAME_STATE.write().unwrap().step();
    }

    // Convert current state to RGB data
    let game_state = GAME_STATE.read().unwrap();
    let frame_data = game_state.to_rgb_data();

    debug!(
        "Advanced generation: current generation {}, {}x{} pixels ({} bytes)",
        game_state.generation_count,
        CANVAS_WIDTH,
        CANVAS_HEIGHT,
        frame_data.len()
    );

    create_frame_message(frame_data)
}

// Utility functions to control Game of Life patterns
pub fn reset_game_of_life_random() {
    GAME_STATE.write().unwrap().initialize_random();
    debug!("Reset Game of Life with random pattern");
}

#[allow(dead_code)]
pub fn reset_game_of_life_glider() {
    GAME_STATE.write().unwrap().initialize_glider();
    debug!("Reset Game of Life with glider pattern");
}

#[allow(dead_code)]
pub fn reset_game_of_life_blinker() {
    GAME_STATE.write().unwrap().initialize_blinker();
    debug!("Reset Game of Life with blinker pattern");
}
