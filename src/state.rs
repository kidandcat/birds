use bevy::prelude::*;

use crate::bird::BirdType;

/// Application state
#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    BirdSelection,
    Playing,
}

/// Game state resource
#[derive(Resource, Default)]
pub struct GameState {
    pub game_over: bool,
    pub won: bool,
}

/// Selected bird type resource
#[derive(Resource)]
pub struct SelectedBirdType(pub BirdType);
