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
            BirdType::Sparrow => 0.6,
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
                min_glide_speed: 10.0,
                perfect_glide_speed: 32.0,
                turn_rate: 1.2,
                roll_rate: 4.0,
                flap_thrust: 13.5,
                flap_cooldown: 0.15,
                max_dive_speed: 90.0,
                acceleration: 35.0,
                walk_speed: 6.0,
            },
            BirdType::Hawk => BirdStats {
                bird_type,
                glide_efficiency: 1.0,
                min_glide_speed: 10.0,
                perfect_glide_speed: 20.0,
                turn_rate: 1.0,
                roll_rate: 3.0,
                flap_thrust: 12.0,
                flap_cooldown: 0.25,
                max_dive_speed: 95.0,
                acceleration: 37.5,
                walk_speed: 5.0,
            },
            BirdType::Eagle => BirdStats {
                bird_type,
                glide_efficiency: 0.6,
                min_glide_speed: 12.0,
                perfect_glide_speed: 25.0,
                turn_rate: 0.7,
                roll_rate: 2.0,
                flap_thrust: 18.0,
                flap_cooldown: 0.4,
                max_dive_speed: 110.0,
                acceleration: 45.0,
                walk_speed: 4.0,
            },
            BirdType::Albatross => BirdStats {
                bird_type,
                glide_efficiency: 0.3,
                min_glide_speed: 15.0,
                perfect_glide_speed: 30.0,
                turn_rate: 0.5,
                roll_rate: 1.5,
                flap_thrust: 22.5,
                flap_cooldown: 0.6,
                max_dive_speed: 180.0,
                acceleration: 52.5,
                walk_speed: 3.0,
            },
        }
    }
}
