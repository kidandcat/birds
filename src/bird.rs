use bevy::prelude::*;

/// Bird types with different flight characteristics
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BirdType {
    Sparrow,   // Small, agile, fast flapping, poor glide
    Hawk,      // Medium, balanced
    Eagle,     // Large, slow turning, excellent glide, powerful
    Albatross, // Very large, best glide, slowest but highest speed
}

impl BirdType {
    pub fn name(&self) -> &'static str {
        match self {
            BirdType::Sparrow => "Sparrow",
            BirdType::Hawk => "Hawk",
            BirdType::Eagle => "Eagle",
            BirdType::Albatross => "Albatross",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            BirdType::Sparrow => Color::srgb(0.6, 0.4, 0.2),   // Brown
            BirdType::Hawk => Color::srgb(0.5, 0.3, 0.2),      // Dark brown
            BirdType::Eagle => Color::srgb(0.2, 0.2, 0.3),     // Dark gray/blue
            BirdType::Albatross => Color::srgb(0.9, 0.9, 0.95), // White
        }
    }

    pub fn scale(&self) -> f32 {
        match self {
            BirdType::Sparrow => 0.4,
            BirdType::Hawk => 1.0,
            BirdType::Eagle => 1.4,
            BirdType::Albatross => 1.8,
        }
    }
}

/// Stats that vary by bird type
#[derive(Component, Clone)]
pub struct BirdStats {
    pub bird_type: BirdType,
    pub glide_efficiency: f32,
    pub min_glide_speed: f32,
    pub perfect_glide_speed: f32,
    pub turn_rate: f32,
    pub roll_rate: f32,
    pub flap_thrust: f32,
    pub flap_cooldown: f32,
    pub max_dive_speed: f32,
    pub acceleration: f32,
    pub walk_speed: f32,
}

impl BirdStats {
    pub fn for_type(bird_type: BirdType) -> Self {
        match bird_type {
            BirdType::Sparrow => BirdStats {
                bird_type,
                glide_efficiency: 1.8,
                min_glide_speed: 5.0,
                perfect_glide_speed: 24.0,  // Halved
                turn_rate: 1.2,
                roll_rate: 4.0,
                flap_thrust: 6.75,          // Halved
                flap_cooldown: 0.15,
                max_dive_speed: 90.0,       // Halved
                acceleration: 17.5,         // Halved
                walk_speed: 6.0,
            },
            BirdType::Hawk => BirdStats {
                bird_type,
                glide_efficiency: 1.0,
                min_glide_speed: 15.0,
                perfect_glide_speed: 67.5,   // 1.5x faster
                turn_rate: 1.0,
                roll_rate: 3.0,
                flap_thrust: 27.0,           // 1.5x thrust
                flap_cooldown: 0.18,
                max_dive_speed: 570.0,       // 1.5x faster
                acceleration: 56.25,         // 1.5x faster
                walk_speed: 5.0,
            },
            BirdType::Eagle => BirdStats {
                bird_type,
                glide_efficiency: 0.6,
                min_glide_speed: 18.0,
                perfect_glide_speed: 84.4,   // 1.5x faster
                turn_rate: 0.7,
                roll_rate: 2.0,
                flap_thrust: 42.0,           // 1.5x thrust
                flap_cooldown: 0.35,
                max_dive_speed: 660.0,       // 1.5x faster
                acceleration: 67.5,          // 1.5x faster
                walk_speed: 4.0,
            },
            BirdType::Albatross => BirdStats {
                bird_type,
                glide_efficiency: 0.3,
                min_glide_speed: 22.5,
                perfect_glide_speed: 101.25, // 1.5x faster
                turn_rate: 0.5,
                roll_rate: 1.5,
                flap_thrust: 33.75,          // 1.5x thrust
                flap_cooldown: 0.6,
                max_dive_speed: 1080.0,      // 1.5x faster
                acceleration: 78.75,         // 1.5x faster
                walk_speed: 3.0,
            },
        }
    }
}
