mod ai;
mod bird;
mod components;
mod network;
mod player;
mod state;
mod ui;
mod world;

use bevy::prelude::*;

use bird::BirdType;
use components::{ChunkManager, FlapState};
use network::NetworkState;
use state::{AppState, GameState, SelectedBirdType};

fn main() {
    let server_addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7777".to_string());

    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::srgb(0.5, 0.7, 0.9)))
        .init_resource::<GameState>()
        .init_resource::<FlapState>()
        .init_resource::<ChunkManager>()
        .init_state::<AppState>()
        .insert_state(AppState::BirdSelection)
        .insert_resource(SelectedBirdType(BirdType::Hawk));

    // Initialize networking
    if let Some(net) = NetworkState::new(&server_addr) {
        println!("Connecting to server at {}", server_addr);
        app.insert_resource(net);
        app.add_systems(OnEnter(AppState::Playing), network::network_connect);
        app.add_systems(
            Update,
            (
                network::network_send_state,
                network::network_receive,
                network::cleanup_stale_players,
                network::remote_player_wing_animation,
            )
                .run_if(in_state(AppState::Playing)),
        );
    } else {
        println!("Failed to initialize networking, running in offline mode");
    }

    // Selection screen
    app.add_systems(Startup, world::setup_environment)
        .add_systems(OnEnter(AppState::BirdSelection), ui::setup_selection_ui)
        .add_systems(
            Update,
            ui::selection_button_system.run_if(in_state(AppState::BirdSelection)),
        )
        .add_systems(OnExit(AppState::BirdSelection), ui::cleanup_selection_ui);

    // Game systems
    app.add_systems(
        OnEnter(AppState::Playing),
        (player::setup_player, player::grab_cursor),
    )
    .add_systems(
        Update,
        (
            player::cursor_toggle,
            player::player_input,
            player::bird_movement,
            player::obstacle_collision,
            ai::ai_bird_movement,
            player::camera_follow,
            player::drafting_system,
            ai::ai_bird_behavior,
            player::wing_flap,
            player::wind_particle_spawner,
            player::wind_particle_update,
            ui::update_ui,
            ui::check_goal,
            world::chunk_management,
            player::reset_to_selection,
        )
            .run_if(in_state(AppState::Playing)),
    )
    .add_systems(OnExit(AppState::Playing), player::cleanup_player)
    .run();
}
