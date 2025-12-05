use bevy::prelude::*;
use std::collections::HashSet;

use crate::bird::BirdType;

/// Marker for the player entity
#[derive(Component)]
pub struct Player;

/// Bird flight state
#[derive(Component)]
pub struct Bird {
    pub speed: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
    pub velocity: Vec3,
    pub grounded: bool,
    pub damage_timer: f32,
    pub walk_timer: f32,
    pub is_walking: bool,
}

/// Obstacle with collision bounds
#[derive(Component)]
pub struct Obstacle {
    pub half_extents: Vec3,
}

/// Drafting behind other birds
#[derive(Component)]
pub struct Drafting {
    pub is_drafting: bool,
}

/// AI-controlled bird behavior
#[derive(Component)]
pub struct AiBird {
    pub target_height: f32,
    pub wander_timer: f32,
    pub wander_direction: f32,
}

/// Goal/nest marker
#[derive(Component)]
pub struct Goal;

/// UI Components
#[derive(Component)]
pub struct DraftIndicator;

#[derive(Component)]
pub struct DistanceText;

#[derive(Component)]
pub struct ServerStatusIndicator;

#[derive(Component)]
pub struct SelectionUI;

#[derive(Component)]
pub struct BirdButton(pub BirdType);

/// Wing component for animation
#[derive(Component)]
pub struct Wing {
    pub is_left: bool,
    pub _base_x: f32,
}

/// Body part identifier
#[derive(Component)]
pub enum BodyPart {
    Body,
    Head,
    Wing,
    Tail,
}

/// Chunk component for LOD terrain
#[derive(Component)]
pub struct Chunk {
    pub x: i32,
    pub z: i32,
    pub lod_level: u8,
}

/// Marker for voxel chunk entities
#[derive(Component)]
pub struct VoxelChunk;

/// Resource managing loaded chunks
#[derive(Resource)]
pub struct ChunkManager {
    pub loaded_chunks: HashSet<(i32, i32, u8)>,
    pub chunk_size: f32,
    pub view_distance: f32,
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self {
            loaded_chunks: HashSet::new(),
            chunk_size: 500.0,
            view_distance: 5000.0,
        }
    }
}

/// Wing flap state
#[derive(Resource, Default)]
pub struct FlapState {
    pub timer: f32,
    pub cooldown: f32,
    pub space_held: bool,
    pub wings_closed_time: f32,
}

/// Wind particle for Albatross dynamic soaring effect
#[derive(Component)]
pub struct WindParticle {
    pub lifetime: f32,
    pub velocity: Vec3,
}

/// Squished state when hitting an obstacle
#[derive(Component)]
pub struct Squished {
    pub timer: f32,           // Time remaining in squish animation
    pub collision_normal: Vec3, // Direction the bird got squished from
    pub original_scale: Vec3,  // Original scale to restore/animate from
}

/// Marks a bird as prey (sparrows) - can be hunted by big birds
#[derive(Component)]
pub struct Prey;

/// AI Sparrow - flies around as prey
#[derive(Component)]
pub struct AiSparrow {
    pub wander_timer: f32,
    pub wander_direction: Vec3,
    pub flee_target: Option<Vec3>,  // Position to flee from
}

/// Being captured by a hunter
#[derive(Component)]
pub struct Captured {
    pub timer: f32,
    pub hunter_pos: Vec3,
}

/// Player score resource
#[derive(Resource, Default)]
pub struct PlayerScore {
    pub points: u32,
    pub passive_timer: f32,  // For sparrow passive scoring
}

/// UI component for score display
#[derive(Component)]
pub struct ScoreText;

/// Hunter strike animation when catching prey
#[derive(Component)]
pub struct HuntingStrike {
    pub timer: f32,
    pub original_scale: Vec3,  // Scale to restore after animation
    pub original_pos: Vec3,    // Position when strike started
    pub prey_pos: Vec3,        // Position of prey for camera to look at
}
