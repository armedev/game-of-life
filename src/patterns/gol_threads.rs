use axum::http::header;
use rand::Rng;
use tracing::debug;

use crate::{constants::DEAD_CELL_R_G_B, utils::create_random_rgb};

#[derive(Clone)]
pub struct GameOfLifeVecs {
    pub width: u16,
    pub height: u16,
    pub current_generation: Vec<Vec<bool>>,
    pub next_generation: Vec<Vec<bool>>,
    pub generation_count: u64,
}

impl GameOfLifeVecs {
    pub fn new(width: u16, height: u16) -> Self {
        let mut game = Self {
            width,
            height,
            current_generation: vec![vec![false; width as usize]; height as usize],
            next_generation: vec![vec![false; width as usize]; height as usize],
            generation_count: 0,
        };
        game.initialize_random();
        game
    }

    pub fn initialize_random(&mut self) {
        let mut rng = rand::rng();
        for y in 0..self.height {
            for x in 0..self.width {
                // 30% chance of a cell being alive initially
                self.current_generation[y as usize][x as usize] = rng.random::<f32>() < 0.3;
            }
        }
        self.generation_count = 0;
        debug!("Initialized Game of Life with random pattern");
    }

    #[allow(dead_code)]
    pub fn initialize_glider(&mut self) {
        // Clear the grid
        for y in 0..self.height {
            for x in 0..self.width {
                self.current_generation[y as usize][x as usize] = false;
            }
        }

        // Create a glider pattern in the top-left
        let glider = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];
        for (dx, dy) in glider.iter() {
            if *dx < self.width && *dy < self.height {
                self.current_generation[*dy as usize][*dx as usize] = true;
            }
        }
        self.generation_count = 0;
        debug!("Initialized Game of Life with glider pattern");
    }

    #[allow(dead_code)]
    pub fn initialize_blinker(&mut self) {
        // Clear the grid
        for y in 0..self.height {
            for x in 0..self.width {
                self.current_generation[y as usize][x as usize] = false;
            }
        }

        // Create a blinker pattern in the center
        let center_x = self.width / 2;
        let center_y = self.height / 2;
        if center_x > 0 && center_y > 0 && center_x < self.width - 1 {
            self.current_generation[center_y as usize][(center_x - 1) as usize] = true;
            self.current_generation[center_y as usize][center_x as usize] = true;
            self.current_generation[center_y as usize][(center_x + 1) as usize] = true;
        }
        self.generation_count = 0;
        debug!("Initialized Game of Life with blinker pattern");
    }

    fn count_live_neighbors(&self, x: u16, y: u16) -> u8 {
        let mut count = 0;
        let x = x as usize;
        let y = y as usize;

        // Use saturating arithmetic to avoid bounds checking in loop
        let start_y = y.saturating_sub(1);
        let end_y = (y + 1).min(self.height as usize - 1);
        let start_x = x.saturating_sub(1);
        let end_x = (x + 1).min(self.width as usize - 1);

        for ny in start_y..=end_y {
            for nx in start_x..=end_x {
                if nx == x && ny == y {
                    continue; // Skip the cell itself
                }
                if self.current_generation[ny][nx] {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn step_fallback(&mut self) {
        // Calculate next generation
        for y in 0..self.height {
            let current_row = &self.current_generation[y as usize];

            for x in 0..self.width {
                let neighbors = self.count_live_neighbors(x as u16, y as u16);
                let current_alive = current_row[x as usize];

                // Conway's Game of Life rules - more explicit and readable
                self.next_generation[y as usize][x as usize] = match neighbors {
                    2 => current_alive, // Stays the same (live stays live, dead stays dead)
                    3 => true,          // Birth or survival
                    _ => false,         // Death or stays dead
                };
            }
        }

        // Swap generations
        std::mem::swap(&mut self.current_generation, &mut self.next_generation);
        self.generation_count += 1;
        debug!("Advanced to generation {}", self.generation_count);
    }

    /// Parallel processing using multiple threads
    pub fn step(&mut self) {
        use std::sync::Arc;
        use std::thread;

        let height = self.height as usize;
        let width = self.width as usize;

        // Create a shared reference to current generation
        let current_gen = Arc::new(self.current_generation.clone());

        // Determine number of threads (use available parallelism or default to 8)
        let num_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8)
            .min(height); // Don't use more threads than rows

        let chunk_size = (height + num_threads - 1) / num_threads;

        // Spawn threads to process different row ranges
        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let current_gen = Arc::clone(&current_gen);
                let start_row = thread_id * chunk_size;
                let end_row = ((thread_id + 1) * chunk_size).min(height);

                thread::spawn(move || {
                    let mut local_next_gen = Vec::new();

                    for y in start_row..end_row {
                        let mut row = Vec::with_capacity(width);

                        for x in 0..width {
                            let neighbors =
                                count_neighbors_parallel(&current_gen, x, y, width, height);
                            let current_alive = current_gen[y][x];

                            let next_alive = match neighbors {
                                2 => current_alive,
                                3 => true,
                                _ => false,
                            };

                            row.push(next_alive);
                        }

                        local_next_gen.push(row);
                    }

                    (start_row, local_next_gen)
                })
            })
            .collect();

        // Collect results from all threads
        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.join() {
                results.push(result);
            }
        }

        // Sort by start_row to maintain order
        results.sort_by_key(|&(start_row, _)| start_row);

        // Reconstruct the next generation
        self.next_generation.clear();
        for (_, mut rows) in results {
            self.next_generation.append(&mut rows);
        }

        // Swap generations
        std::mem::swap(&mut self.current_generation, &mut self.next_generation);
        self.generation_count += 1;
        debug!(
            "Advanced to generation {} (parallel)",
            self.generation_count
        );
    }

    pub fn to_rgb_data(&self) -> Vec<u8> {
        let mut frame_data =
            Vec::with_capacity((self.width as usize * self.height as usize * 3) as usize);

        for y in 0..self.height {
            for x in 0..self.width {
                if self.current_generation[y as usize][x as usize] {
                    frame_data.extend(create_random_rgb());
                } else {
                    frame_data.extend(DEAD_CELL_R_G_B); // R G B
                }
            }
        }

        frame_data
    }

    pub fn awaken_random_cell(&mut self) -> (u16, u16) {
        let mut rng = rand::rng();
        let x: u16 = rng.random_range(0u16..self.width);
        let y: u16 = rng.random_range(0u16..self.height);

        self.current_generation[y as usize][x as usize] = true;
        (x, y)
    }

    pub fn awaken_cell_in(&mut self, x: u16, y: u16) -> (u16, u16) {
        self.current_generation[y as usize][x as usize] = true;
        (x, y)
    }

    pub fn kill_random_cell(&mut self) -> (u16, u16) {
        let mut rng = rand::rng();
        let x: u16 = rng.random_range(0u16..self.width);
        let y: u16 = rng.random_range(0u16..self.height);

        self.current_generation[y as usize][x as usize] = false;
        (x, y)
    }

    pub fn kill_all_cells(&mut self) {
        self.next_generation = vec![vec![false; self.width as usize]; self.height as usize];
        std::mem::swap(&mut self.current_generation, &mut self.next_generation);
        self.generation_count = 0
    }
}

// Helper function for parallel neighbor counting
fn count_neighbors_parallel(
    current_gen: &[Vec<bool>],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> u8 {
    let mut count = 0;

    // Use saturating arithmetic to avoid bounds checking
    let start_y = y.saturating_sub(1);
    let end_y = (y + 1).min(height - 1);
    let start_x = x.saturating_sub(1);
    let end_x = (x + 1).min(width - 1);

    for ny in start_y..=end_y {
        for nx in start_x..=end_x {
            if nx == x && ny == y {
                continue; // Skip the cell itself
            }
            if current_gen[ny][nx] {
                count += 1;
            }
        }
    }
    count
}
