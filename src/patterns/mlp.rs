use crate::{
    constants::{CANVAS_HEIGHT, CANVAS_WIDTH},
    utils::{create_frame_message, create_pixel_message},
};
use axum_tws::Message;
use once_cell::sync::Lazy;
use std::sync::RwLock;
use tracing::debug;

// Mona Lisa painting state
#[derive(Debug, Clone)]
pub struct MonaLisaPainting {
    canvas: Vec<Vec<[u8; 3]>>, // RGB canvas
    reveal_progress: usize,    // How much has been revealed
    brush_strokes: Vec<BrushStroke>,
    current_stroke: usize,
    painting_complete: bool,
}

#[derive(Debug, Clone)]
pub struct BrushStroke {
    points: Vec<(usize, usize)>, // (x, y) coordinates
    color: [u8; 3],              // RGB color
}

// Global Mona Lisa state
static MONA_LISA_STATE: Lazy<RwLock<MonaLisaPainting>> = Lazy::new(|| {
    RwLock::new(MonaLisaPainting::new(
        CANVAS_WIDTH as usize,
        CANVAS_HEIGHT as usize,
    ))
});

impl MonaLisaPainting {
    pub fn new(width: usize, height: usize) -> Self {
        let canvas = vec![vec![[240, 235, 220]; width]; height]; // Cream background
        let brush_strokes = Self::generate_mona_lisa_strokes(width, height);

        Self {
            canvas,
            reveal_progress: 0,
            brush_strokes,
            current_stroke: 0,
            painting_complete: false,
        }
    }

    fn generate_mona_lisa_strokes(width: usize, height: usize) -> Vec<BrushStroke> {
        let mut strokes = Vec::new();
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let scale = (width.min(height) as f32) / 400.0; // Scale factor for different canvas sizes

        // Background - atmospheric perspective with distant landscape
        // Sky gradient (blue-green to pale yellow)
        for y in 0..height {
            for x in 0..width {
                let sky_progress = y as f32 / height as f32;
                let distance_factor =
                    ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt()
                        / (width as f32 + height as f32);

                let base_blue = 120.0 - sky_progress * 40.0;
                let base_green = 140.0 - sky_progress * 20.0;
                let base_brown = 80.0 + sky_progress * 30.0;

                strokes.push(BrushStroke {
                    points: vec![(x, y)],
                    color: [
                        (base_brown + distance_factor * 20.0).min(255.0) as u8,
                        (base_green + distance_factor * 15.0).min(255.0) as u8,
                        (base_blue + distance_factor * 10.0).min(255.0) as u8,
                    ],
                });
            }
        }

        // Distant mountains and landscape (sfumato technique)
        for layer in 0..5 {
            let mountain_y = (height as f32 * 0.3) + (layer as f32 * 20.0);
            let opacity = 255 - (layer * 40);

            for x in 0..width {
                let noise = ((x as f32 * 0.02).sin() + (x as f32 * 0.05).cos()) * 15.0;
                let y = mountain_y + noise;

                if y >= 0.0 && y < height as f32 {
                    strokes.push(BrushStroke {
                        points: vec![(x, y as usize)],
                        color: [
                            (100 + layer * 20).min(255) as u8,
                            (120 + layer * 15).min(255) as u8,
                            (140 + layer * 10).min(255) as u8,
                        ],
                    });
                }
            }
        }

        // Subject positioning (three-quarter view)
        let face_center_x = center_x - (20.0 * scale);
        let face_center_y = center_y - (30.0 * scale);

        // Hair - elaborate Renaissance hairstyle with curls and waves
        let hair_color = [45, 30, 20];
        let hair_highlight = [80, 60, 40];

        // Main hair mass
        for angle in 0..720 {
            // More detailed hair outline
            let rad = angle as f32 * std::f32::consts::PI / 360.0;
            let hair_radius = 80.0 * scale + (angle as f32 * 0.1).sin() * 10.0 * scale;
            let x = face_center_x + hair_radius * rad.cos() * 1.2;
            let y = face_center_y + hair_radius * rad.sin() * 0.8 - (30.0 * scale);

            if x >= 0.0 && x < width as f32 && y >= 0.0 && y < height as f32 {
                strokes.push(BrushStroke {
                    points: vec![(x as usize, y as usize)],
                    color: if angle % 20 < 5 {
                        hair_highlight
                    } else {
                        hair_color
                    },
                });
            }
        }

        // Hair curls and texture
        for i in 0..300 {
            let curl_x = face_center_x
                + ((i as f32 * 0.1).cos() * 60.0 + (i as f32 * 0.3).sin() * 20.0) * scale;
            let curl_y = face_center_y + ((i as f32 * 0.1).sin() * 40.0 - 40.0) * scale;

            if curl_x >= 0.0 && curl_x < width as f32 && curl_y >= 0.0 && curl_y < height as f32 {
                strokes.push(BrushStroke {
                    points: vec![(curl_x as usize, curl_y as usize)],
                    color: hair_color,
                });
            }
        }

        // Face shape - realistic oval with proper proportions
        let face_color = [240, 220, 195];
        let face_shadow = [210, 185, 160];

        for angle in 0..360 {
            let rad = angle as f32 * std::f32::consts::PI / 180.0;
            let face_width = 50.0 * scale;
            let face_height = 65.0 * scale;
            let x = face_center_x + face_width * rad.cos();
            let y = face_center_y + face_height * rad.sin();

            if x >= 0.0 && x < width as f32 && y >= 0.0 && y < height as f32 {
                // Add subtle shading based on angle (light from upper left)
                let light_factor = (rad.cos() + rad.sin() + 2.0) / 4.0;
                let color = if light_factor > 0.6 {
                    face_color
                } else {
                    face_shadow
                };

                strokes.push(BrushStroke {
                    points: vec![(x as usize, y as usize)],
                    color,
                });
            }
        }

        // Eyes - the famous enigmatic gaze
        let eye_color = [60, 40, 25];
        let eye_white = [250, 245, 240];
        let iris_color = [80, 60, 40];

        // Left eye (viewer's right)
        let left_eye_x = face_center_x - (18.0 * scale);
        let left_eye_y = face_center_y - (15.0 * scale);

        // Eye socket and lid
        for dy in -8..(8.0 * scale) as i32 {
            for dx in -12..(12.0 * scale) as i32 {
                let x = left_eye_x + dx as f32;
                let y = left_eye_y + dy as f32;
                let dist = (dx * dx + dy * dy) as f32;

                if x >= 0.0
                    && x < width as f32
                    && y >= 0.0
                    && y < height as f32
                    && dist < (10.0 * scale).powi(2)
                {
                    let color = if dist < (6.0 * scale).powi(2) {
                        if dist < (4.0 * scale).powi(2) {
                            iris_color
                        } else {
                            eye_white
                        }
                    } else {
                        face_shadow
                    };

                    strokes.push(BrushStroke {
                        points: vec![(x as usize, y as usize)],
                        color,
                    });
                }
            }
        }

        // Right eye (viewer's left)
        let right_eye_x = face_center_x + (18.0 * scale);
        let right_eye_y = face_center_y - (15.0 * scale);

        for dy in -8..(8.0 * scale) as i32 {
            for dx in -12..(12.0 * scale) as i32 {
                let x = right_eye_x + dx as f32;
                let y = right_eye_y + dy as f32;
                let dist = (dx * dx + dy * dy) as f32;

                if x >= 0.0
                    && x < width as f32
                    && y >= 0.0
                    && y < height as f32
                    && dist < (10.0 * scale).powi(2)
                {
                    let color = if dist < (6.0 * scale).powi(2) {
                        if dist < (4.0 * scale).powi(2) {
                            iris_color
                        } else {
                            eye_white
                        }
                    } else {
                        face_shadow
                    };

                    strokes.push(BrushStroke {
                        points: vec![(x as usize, y as usize)],
                        color,
                    });
                }
            }
        }

        // Nose - subtle shading and form
        let nose_x = face_center_x;
        let nose_y = face_center_y + (5.0 * scale);

        for i in 0..20 {
            let y_offset = i as f32 * 2.0 * scale;
            let width_factor = (1.0 - (i as f32 / 20.0) * 0.5) * scale;

            for dx in -(3.0 * width_factor) as i32..(3.0 * width_factor) as i32 {
                let x = nose_x + dx as f32;
                let y = nose_y + y_offset;

                if x >= 0.0 && x < width as f32 && y >= 0.0 && y < height as f32 {
                    strokes.push(BrushStroke {
                        points: vec![(x as usize, y as usize)],
                        color: if dx.abs() < 2 {
                            face_shadow
                        } else {
                            face_color
                        },
                    });
                }
            }
        }

        // The famous smile - subtle and enigmatic
        let smile_color = [220, 195, 170];
        let lip_color = [200, 160, 140];

        for i in 0..60 {
            let progress = i as f32 / 60.0;
            let smile_curve = (progress * std::f32::consts::PI).sin() * 3.0 * scale;
            let x = face_center_x + (progress - 0.5) * 40.0 * scale;
            let y = face_center_y + 25.0 * scale + smile_curve;

            if x >= 0.0 && x < width as f32 && y >= 0.0 && y < height as f32 {
                strokes.push(BrushStroke {
                    points: vec![(x as usize, y as usize)],
                    color: if i % 10 < 3 { lip_color } else { smile_color },
                });
            }
        }

        // Hands - the famous folded hands pose
        let hand_color = [235, 210, 185];
        let hand_shadow = [200, 175, 150];

        // Left hand (viewer's right)
        let left_hand_x = face_center_x - (60.0 * scale);
        let left_hand_y = center_y + (100.0 * scale);

        for finger in 0..5 {
            for segment in 0..3 {
                let finger_x = left_hand_x + (finger as f32 - 2.0) * 8.0 * scale;
                let finger_y = left_hand_y + segment as f32 * 12.0 * scale;

                for dy in 0..(8.0 * scale) as i32 {
                    for dx in 0..(6.0 * scale) as i32 {
                        let x = finger_x + dx as f32;
                        let y = finger_y + dy as f32;

                        if x >= 0.0 && x < width as f32 && y >= 0.0 && y < height as f32 {
                            strokes.push(BrushStroke {
                                points: vec![(x as usize, y as usize)],
                                color: if dx < 3 { hand_color } else { hand_shadow },
                            });
                        }
                    }
                }
            }
        }

        // Right hand (viewer's left) - partially visible
        let right_hand_x = face_center_x + (40.0 * scale);
        let right_hand_y = center_y + (110.0 * scale);

        for dy in 0..(25.0 * scale) as i32 {
            for dx in 0..(20.0 * scale) as i32 {
                let x = right_hand_x + dx as f32;
                let y = right_hand_y + dy as f32;

                if x >= 0.0 && x < width as f32 && y >= 0.0 && y < height as f32 {
                    strokes.push(BrushStroke {
                        points: vec![(x as usize, y as usize)],
                        color: if dx < 10 { hand_color } else { hand_shadow },
                    });
                }
            }
        }

        // Dress - dark Renaissance gown with detailed folds
        let dress_dark = [25, 20, 15];
        let dress_mid = [40, 35, 25];
        let dress_light = [60, 50, 35];

        // Main dress body
        for y in (center_y + 80.0 * scale) as usize..height.min((center_y + 200.0 * scale) as usize)
        {
            let dress_width = 120.0 * scale - (y as f32 - center_y - 80.0 * scale) * 0.2;
            let dress_start_x = face_center_x - dress_width / 2.0;
            let dress_end_x = face_center_x + dress_width / 2.0;

            for x in dress_start_x as usize..dress_end_x.min(width as f32) as usize {
                // Create fabric folds
                let fold_pattern = ((x as f32 * 0.1).sin() + (y as f32 * 0.05).cos()) * 0.5 + 0.5;
                let color = if fold_pattern > 0.7 {
                    dress_light
                } else if fold_pattern > 0.3 {
                    dress_mid
                } else {
                    dress_dark
                };

                strokes.push(BrushStroke {
                    points: vec![(x, y)],
                    color,
                });
            }
        }

        // Add atmospheric effects and subtle details
        // Veil/transparent fabric (very subtle)
        for i in 0..100 {
            let veil_x = face_center_x + ((i as f32 * 0.2).sin() * 30.0) * scale;
            let veil_y = face_center_y - (20.0 * scale) + i as f32 * 0.5 * scale;

            if veil_x >= 0.0 && veil_x < width as f32 && veil_y >= 0.0 && veil_y < height as f32 {
                strokes.push(BrushStroke {
                    points: vec![(veil_x as usize, veil_y as usize)],
                    color: [250, 245, 240],
                });
            }
        }

        strokes
    }

    pub fn to_rgb_data(&self) -> Vec<u8> {
        let mut rgb_data = Vec::with_capacity(self.canvas.len() * self.canvas[0].len() * 3);

        for row in &self.canvas {
            for pixel in row {
                rgb_data.extend_from_slice(pixel);
            }
        }

        rgb_data
    }

    pub fn apply_next_stroke(&mut self) -> Option<(usize, usize, [u8; 3])> {
        if self.current_stroke >= self.brush_strokes.len() {
            self.painting_complete = true;
            return None;
        }

        let stroke = &self.brush_strokes[self.current_stroke];
        let mut last_point = None;

        for &(x, y) in &stroke.points {
            if y < self.canvas.len() && x < self.canvas[0].len() {
                self.canvas[y][x] = stroke.color;
                last_point = Some((x, y, stroke.color));
            }
        }

        self.current_stroke += 1;
        self.reveal_progress = (self.current_stroke * 100) / self.brush_strokes.len();

        last_point
    }

    pub fn apply_multiple_strokes(&mut self, count: usize) -> Vec<(usize, usize, [u8; 3])> {
        let mut applied_strokes = Vec::new();

        for _ in 0..count {
            if let Some(stroke_info) = self.apply_next_stroke() {
                applied_strokes.push(stroke_info);
            } else {
                break;
            }
        }

        applied_strokes
    }

    pub fn reset(&mut self) {
        self.canvas = vec![vec![[240, 235, 220]; self.canvas[0].len()]; self.canvas.len()];
        self.current_stroke = 0;
        self.reveal_progress = 0;
        self.painting_complete = false;
    }

    pub fn is_complete(&self) -> bool {
        self.painting_complete
    }

    pub fn progress_percentage(&self) -> usize {
        self.reveal_progress
    }
}

// Public API functions
pub fn start_new_painting() -> Message {
    {
        MONA_LISA_STATE.write().unwrap().reset();
    }
    let painting_state = MONA_LISA_STATE.read().unwrap();
    let frame_data = painting_state.to_rgb_data();
    debug!("Started new Mona Lisa painting");
    create_frame_message(frame_data)
}

pub fn apply_single_brush_stroke() -> Message {
    let stroke_info = { MONA_LISA_STATE.write().unwrap().apply_next_stroke() };

    match stroke_info {
        Some((x, y, [r, g, b])) => {
            let painting_state = MONA_LISA_STATE.read().unwrap();
            debug!(
                "Applied brush stroke at ({}, {}), progress: {}%",
                x,
                y,
                painting_state.progress_percentage()
            );
            create_pixel_message(x as u16, y as u16, r, g, b)
        }
        None => {
            debug!("Mona Lisa painting complete!");
            current_painting_frame()
        }
    }
}

pub fn apply_brush_strokes_batch(count: usize) -> Message {
    {
        MONA_LISA_STATE
            .write()
            .unwrap()
            .apply_multiple_strokes(count);
    }

    let painting_state = MONA_LISA_STATE.read().unwrap();
    let frame_data = painting_state.to_rgb_data();
    debug!(
        "Applied {} brush strokes, progress: {}%",
        count,
        painting_state.progress_percentage()
    );
    create_frame_message(frame_data)
}

pub fn current_painting_frame() -> Message {
    let painting_state = MONA_LISA_STATE.read().unwrap();
    let frame_data = painting_state.to_rgb_data();
    debug!(
        "Current painting frame: {}% complete",
        painting_state.progress_percentage()
    );
    create_frame_message(frame_data)
}

pub fn fast_forward_painting() -> Message {
    let remaining_strokes = {
        let painting_state = MONA_LISA_STATE.read().unwrap();
        if painting_state.is_complete() {
            0
        } else {
            painting_state.brush_strokes.len() - painting_state.current_stroke
        }
    };

    if remaining_strokes > 0 {
        {
            MONA_LISA_STATE
                .write()
                .unwrap()
                .apply_multiple_strokes(remaining_strokes);
        }
        debug!("Fast-forwarded Mona Lisa painting to completion");
    }

    current_painting_frame()
}

pub fn painting_progress() -> usize {
    MONA_LISA_STATE.read().unwrap().progress_percentage()
}

pub fn is_painting_complete() -> bool {
    MONA_LISA_STATE.read().unwrap().is_complete()
}

// Artistic variations
pub fn add_random_detail_stroke() -> Message {
    use rand::Rng;
    let mut rng = rand::rng();

    let (x, y, color) = {
        let mut painting_state = MONA_LISA_STATE.write().unwrap();
        let x = rng.random_range(0..painting_state.canvas[0].len());
        let y = rng.random_range(0..painting_state.canvas.len());

        // Add some artistic variation to existing colors
        let existing_color = painting_state.canvas[y][x];
        let variation = rng.random_range(-20i16..=20i16);
        let new_color = [
            (existing_color[0] as i16 + variation).clamp(0, 255) as u8,
            (existing_color[1] as i16 + variation).clamp(0, 255) as u8,
            (existing_color[2] as i16 + variation).clamp(0, 255) as u8,
        ];

        painting_state.canvas[y][x] = new_color;
        (x, y, new_color)
    };

    debug!("Added random detail stroke at ({}, {})", x, y);
    create_pixel_message(x as u16, y as u16, color[0], color[1], color[2])
}
