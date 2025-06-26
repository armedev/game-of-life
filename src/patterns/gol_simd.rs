use rand::Rng;
use std::arch::{aarch64::*, is_aarch64_feature_detected};
use tracing::debug;

use crate::{constants::DEAD_CELL_R_G_B, utils::create_random_rgb};

const BIT_LENGTH: usize = 64;

#[derive(Clone)]
pub struct GameOfLifeBits {
    pub width: u16,
    pub height: u16,
    // Store as bits: each u64 holds 64 cells
    pub current_generation: Vec<u64>,
    pub next_generation: Vec<u64>,
    pub generation_count: u64,
    // Width in u64 chunks (rounded up)
    width_chunks: usize,
}

impl GameOfLifeBits {
    pub fn new(width: u16, height: u16) -> Self {
        let width_chunks = ((width as usize) + BIT_LENGTH - 1) / BIT_LENGTH; // Round up to nearest 64
        let total_chunks = width_chunks * height as usize;

        let mut game = Self {
            width,
            height,
            current_generation: vec![0u64; total_chunks],
            next_generation: vec![0u64; total_chunks],
            generation_count: 0,
            width_chunks,
        };
        game.initialize_random();
        game
    }

    #[inline]
    fn get_chunk_index(&self, x: usize, y: usize) -> (usize, usize) {
        let chunk_x = x / 64;
        let bit_x = x % 64;
        let chunk_index = y * self.width_chunks + chunk_x;
        (chunk_index, bit_x)
    }

    #[inline]
    fn get_cell(&self, x: usize, y: usize) -> bool {
        if x >= self.width as usize || y >= self.height as usize {
            return false;
        }
        let (chunk_index, bit_x) = self.get_chunk_index(x, y);
        if chunk_index >= self.current_generation.len() {
            return false;
        }
        (self.current_generation[chunk_index] >> bit_x) & 1 == 1
    }

    #[inline]
    fn set_cell(&mut self, x: usize, y: usize, alive: bool) {
        if x >= self.width as usize || y >= self.height as usize {
            return;
        }
        let (chunk_index, bit_x) = self.get_chunk_index(x, y);
        if chunk_index >= self.current_generation.len() {
            return;
        }

        if alive {
            self.current_generation[chunk_index] |= 1u64 << bit_x;
        } else {
            self.current_generation[chunk_index] &= !(1u64 << bit_x);
        }
    }

    #[inline]
    fn set_next_cell(&mut self, x: usize, y: usize, alive: bool) {
        if x >= self.width as usize || y >= self.height as usize {
            return;
        }
        let (chunk_index, bit_x) = self.get_chunk_index(x, y);
        if chunk_index >= self.next_generation.len() {
            return;
        }

        if alive {
            self.next_generation[chunk_index] |= 1u64 << bit_x;
        }
    }

    pub fn initialize_random(&mut self) {
        let mut rng = rand::rng();
        for chunk in &mut self.current_generation {
            *chunk = 0;
        }

        for y in 0..self.height {
            for x in 0..self.width {
                if rng.random::<f32>() < 0.3 {
                    self.set_cell(x as usize, y as usize, true);
                }
            }
        }
        self.generation_count = 0;
        debug!("Initialized Game of Life with random pattern");
    }

    pub fn initialize_glider(&mut self) {
        // Clear the grid
        for chunk in &mut self.current_generation {
            *chunk = 0;
        }

        // Create a glider pattern in the top-left
        let glider = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];
        for (x, y) in glider.iter() {
            self.set_cell(*x, *y, true);
        }
        self.generation_count = 0;
        debug!("Initialized Game of Life with glider pattern");
    }

    pub fn initialize_blinker(&mut self) {
        // Clear the grid
        for chunk in &mut self.current_generation {
            *chunk = 0;
        }

        // Create a blinker pattern in the center
        let center_x = (self.width / 2) as usize;
        let center_y = (self.height / 2) as usize;

        if center_x > 0 && center_y > 0 && center_x < (self.width - 1) as usize {
            self.set_cell(center_x - 1, center_y, true);
            self.set_cell(center_x, center_y, true);
            self.set_cell(center_x + 1, center_y, true);
        }
        self.generation_count = 0;
        debug!("Initialized Game of Life with blinker pattern");
    }

    #[inline]
    fn count_neighbors_optimized(&self, x: usize, y: usize) -> u8 {
        let mut count = 0;
        let width = self.width as usize;
        let height = self.height as usize;

        // Unrolled neighbor checking for better performance
        // Top row
        if y > 0 {
            if x > 0 && self.get_cell(x - 1, y - 1) {
                count += 1;
            }
            if self.get_cell(x, y - 1) {
                count += 1;
            }
            if x < width - 1 && self.get_cell(x + 1, y - 1) {
                count += 1;
            }
        }

        // Middle row (left and right)
        if x > 0 && self.get_cell(x - 1, y) {
            count += 1;
        }
        if x < width - 1 && self.get_cell(x + 1, y) {
            count += 1;
        }

        // Bottom row
        if y < height - 1 {
            if x > 0 && self.get_cell(x - 1, y + 1) {
                count += 1;
            }
            if self.get_cell(x, y + 1) {
                count += 1;
            }
            if x < width - 1 && self.get_cell(x + 1, y + 1) {
                count += 1;
            }
        }

        count
    }

    // ARM NEON optimized step function
    pub fn step(&mut self) {
        self.step_parallel();
        self.generation_count += 1;
        debug!("Advanced to generation {}", self.generation_count);
    }

    #[target_feature(enable = "neon")]
    unsafe fn step_neon(&mut self) {
        // Clear next generation using NEON
        let chunks = self.next_generation.len();
        let mut i = 0;

        unsafe {
            // Process 2 u64s at a time with NEON (128-bit registers)
            while i + 1 < chunks {
                let zeros = vdupq_n_u64(0);
                vst1q_u64(self.next_generation.as_mut_ptr().add(i), zeros);
                i += 2;
            }
        }

        // Handle remaining chunks
        while i < chunks {
            self.next_generation[i] = 0;
            i += 1;
        }

        // Process each cell with optimized neighbor counting
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let neighbors = self.count_neighbors_optimized(x, y);
                let current_alive = self.get_cell(x, y);

                let next_alive = match neighbors {
                    2 => current_alive,
                    3 => true,
                    _ => false,
                };

                if next_alive {
                    self.set_next_cell(x, y, true);
                }
            }
        }

        // Swap generations using NEON for bulk copy
        self.swap_generations_neon();
    }

    #[target_feature(enable = "neon")]
    unsafe fn swap_generations_neon(&mut self) {
        let chunks = self.current_generation.len();
        let mut i = 0;

        unsafe {
            // Process 2 u64s at a time
            while i + 1 < chunks {
                let current = vld1q_u64(self.current_generation.as_ptr().add(i));
                let next = vld1q_u64(self.next_generation.as_ptr().add(i));

                vst1q_u64(self.current_generation.as_mut_ptr().add(i), next);
                vst1q_u64(self.next_generation.as_mut_ptr().add(i), current);

                i += 2;
            }
        }

        // Handle remaining chunks
        while i < chunks {
            std::mem::swap(
                &mut self.current_generation[i],
                &mut self.next_generation[i],
            );
            i += 1;
        }
    }

    fn step_fallback(&mut self) {
        // Clear next generation
        for chunk in &mut self.next_generation {
            *chunk = 0;
        }

        // Process each cell
        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                let neighbors = self.count_neighbors_optimized(x, y);
                let current_alive = self.get_cell(x, y);

                let next_alive = match neighbors {
                    2 => current_alive,
                    3 => true,
                    _ => false,
                };

                if next_alive {
                    self.set_next_cell(x, y, true);
                }
            }
        }

        std::mem::swap(&mut self.current_generation, &mut self.next_generation);
    }

    pub fn to_rgb_data(&self) -> Vec<u8> {
        let mut frame_data = Vec::with_capacity(self.width as usize * self.height as usize * 3);

        for y in 0..self.height as usize {
            for x in 0..self.width as usize {
                if self.get_cell(x, y) {
                    frame_data.extend(create_random_rgb());
                } else {
                    frame_data.extend_from_slice(&DEAD_CELL_R_G_B);
                }
            }
        }

        frame_data
    }

    pub fn awaken_random_cell(&mut self) -> (u16, u16) {
        let mut rng = rand::rng();
        let x = rng.random_range(0..self.width as usize);
        let y = rng.random_range(0..self.height as usize);

        self.set_cell(x, y, true);
        (x as u16, y as u16)
    }

    pub fn kill_random_cell(&mut self) -> (u16, u16) {
        let mut rng = rand::rng();
        let x = rng.random_range(0..self.width as usize);
        let y = rng.random_range(0..self.height as usize);

        self.set_cell(x, y, false);
        (x as u16, y as u16)
    }

    // Utility functions using bit manipulation
    pub fn population_count(&self) -> u32 {
        if is_aarch64_feature_detected!("neon") {
            unsafe { self.population_count_neon() }
        } else {
            self.current_generation
                .iter()
                .map(|chunk| chunk.count_ones())
                .sum()
        }
    }

    #[target_feature(enable = "neon")]
    unsafe fn population_count_neon(&self) -> u32 {
        let mut total = 0u32;
        let chunks = self.current_generation.len();
        let mut i = 0;

        // Process 2 u64s at a time
        while i + 1 < chunks {
            let data = vld1q_u64(self.current_generation.as_ptr().add(i));
            // Use ARM's population count instruction
            let count = vcntq_u8(vreinterpretq_u8_u64(data));
            let sum = vaddvq_u8(count);
            total += sum as u32;
            i += 2;
        }

        // Handle remaining chunks
        while i < chunks {
            total += self.current_generation[i].count_ones();
            i += 1;
        }

        total
    }

    pub fn clear(&mut self) {
        if is_aarch64_feature_detected!("neon") {
            unsafe {
                self.clear_neon();
            }
        } else {
            for chunk in &mut self.current_generation {
                *chunk = 0;
            }
        }
        self.generation_count = 0;
    }

    #[target_feature(enable = "neon")]
    unsafe fn clear_neon(&mut self) {
        let chunks = self.current_generation.len();
        let mut i = 0;
        let zeros = vdupq_n_u64(0);

        // Process 2 u64s at a time
        while i + 1 < chunks {
            vst1q_u64(self.current_generation.as_mut_ptr().add(i), zeros);
            i += 2;
        }

        // Handle remaining chunks
        while i < chunks {
            self.current_generation[i] = 0;
            i += 1;
        }
    }

    pub fn invert(&mut self) {
        for (i, chunk) in self.current_generation.iter_mut().enumerate() {
            *chunk = !*chunk;

            // Mask out bits beyond the actual width for the last chunk in each row
            if (i + 1) % self.width_chunks == 0 {
                let bits_in_last_chunk = self.width as usize % 64;
                if bits_in_last_chunk > 0 {
                    *chunk &= (1u64 << bits_in_last_chunk) - 1;
                }
            }
        }
    }

    // Parallel processing using multiple threads (good for Apple Silicon's many cores)
    pub fn step_parallel(&mut self) {
        use std::sync::Arc;
        use std::thread;

        let width = self.width as usize;
        let height = self.height as usize;
        let current_gen = Arc::new(self.current_generation.clone());
        let mut next_gen = vec![0u64; self.current_generation.len()];

        let num_threads = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(8);
        let chunk_size = (height + num_threads - 1) / num_threads;

        let handles: Vec<_> = (0..num_threads)
            .map(|thread_id| {
                let current_gen = Arc::clone(&current_gen);
                let start_y = thread_id * chunk_size;
                let end_y = ((thread_id + 1) * chunk_size).min(height);
                let width_chunks = self.width_chunks;

                thread::spawn(move || {
                    let mut local_next = vec![0u64; width_chunks * (end_y - start_y)];

                    for y in start_y..end_y {
                        for x in 0..width {
                            let neighbors = count_neighbors_for_parallel(
                                &current_gen,
                                x,
                                y,
                                width,
                                height,
                                width_chunks,
                            );
                            let current_alive =
                                get_cell_for_parallel(&current_gen, x, y, width_chunks);

                            let next_alive = match neighbors {
                                2 => current_alive,
                                3 => true,
                                _ => false,
                            };

                            if next_alive {
                                let local_y = y - start_y;
                                let (chunk_idx, bit_x) =
                                    get_chunk_index_for_parallel(x, local_y, width_chunks);
                                if chunk_idx < local_next.len() {
                                    local_next[chunk_idx] |= 1u64 << bit_x;
                                }
                            }
                        }
                    }

                    (start_y, local_next)
                })
            })
            .collect();

        // Collect results
        for handle in handles {
            if let Ok((start_y, local_next)) = handle.join() {
                let start_chunk = start_y * self.width_chunks;
                for (i, &chunk) in local_next.iter().enumerate() {
                    if start_chunk + i < next_gen.len() {
                        next_gen[start_chunk + i] = chunk;
                    }
                }
            }
        }

        self.next_generation = next_gen;
        std::mem::swap(&mut self.current_generation, &mut self.next_generation);
        self.generation_count += 1;
    }
}

// Helper functions for parallel processing
#[inline]
fn get_chunk_index_for_parallel(x: usize, y: usize, width_chunks: usize) -> (usize, usize) {
    let chunk_x = x / 64;
    let bit_x = x % 64;
    let chunk_index = y * width_chunks + chunk_x;
    (chunk_index, bit_x)
}

#[inline]
fn get_cell_for_parallel(generation: &[u64], x: usize, y: usize, width_chunks: usize) -> bool {
    let (chunk_index, bit_x) = get_chunk_index_for_parallel(x, y, width_chunks);
    if chunk_index >= generation.len() {
        return false;
    }
    (generation[chunk_index] >> bit_x) & 1 == 1
}

fn count_neighbors_for_parallel(
    generation: &[u64],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    width_chunks: usize,
) -> u8 {
    let mut count = 0;

    // Top row
    if y > 0 {
        if x > 0 && get_cell_for_parallel(generation, x - 1, y - 1, width_chunks) {
            count += 1;
        }
        if get_cell_for_parallel(generation, x, y - 1, width_chunks) {
            count += 1;
        }
        if x < width - 1 && get_cell_for_parallel(generation, x + 1, y - 1, width_chunks) {
            count += 1;
        }
    }

    // Middle row
    if x > 0 && get_cell_for_parallel(generation, x - 1, y, width_chunks) {
        count += 1;
    }
    if x < width - 1 && get_cell_for_parallel(generation, x + 1, y, width_chunks) {
        count += 1;
    }

    // Bottom row
    if y < height - 1 {
        if x > 0 && get_cell_for_parallel(generation, x - 1, y + 1, width_chunks) {
            count += 1;
        }
        if get_cell_for_parallel(generation, x, y + 1, width_chunks) {
            count += 1;
        }
        if x < width - 1 && get_cell_for_parallel(generation, x + 1, y + 1, width_chunks) {
            count += 1;
        }
    }

    count
}
