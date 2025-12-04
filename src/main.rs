use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use flight_shared::{ClientMessage, NetBirdType, PlayerState, ServerMessage};
use rand::Rng;
use std::collections::HashSet;
use std::f32::consts::PI;
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::time::Instant;



#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
enum AppState {
    #[default]
    BirdSelection,
    Playing,
}

#[derive(Resource, Default)]
struct GameState {
    game_over: bool,
    won: bool,
}

#[derive(Resource)]
struct SelectedBirdType(BirdType);

// Networking resources
#[derive(Resource)]
struct NetworkState {
    socket: UdpSocket,
    player_id: Option<u32>,
    connected: bool,
    last_send: Instant,
    server_addr: String,
}

impl NetworkState {
    fn new(server_addr: &str) -> Option<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.set_nonblocking(true).ok()?;
        Some(Self {
            socket,
            player_id: None,
            connected: false,
            last_send: Instant::now(),
            server_addr: server_addr.to_string(),
        })
    }

    fn send(&self, msg: &ClientMessage) {
        let data = flight_shared::encode(msg);
        let _ = self.socket.send_to(&data, &self.server_addr);
    }
}

#[derive(Component)]
struct RemotePlayer {
    player_id: u32,
    last_update: Instant,
    bird_type: NetBirdType,
    wings_closed: bool,
    flap_timer: f32,
    grounded: bool,
    is_walking: bool,
}

fn main() {
    // Get server address from args or use default
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
    if let Some(network) = NetworkState::new(&server_addr) {
        println!("Connecting to server at {}", server_addr);
        app.insert_resource(network);
        app.add_systems(OnEnter(AppState::Playing), network_connect);
        app.add_systems(Update, (
            network_send_state,
            network_receive,
            cleanup_stale_players,
            remote_player_wing_animation,
        ).run_if(in_state(AppState::Playing)));
    } else {
        println!("Failed to initialize networking, running in offline mode");
    }

    // Selection screen
    app.add_systems(Startup, setup_environment)
        .add_systems(OnEnter(AppState::BirdSelection), setup_selection_ui)
        .add_systems(Update, selection_button_system.run_if(in_state(AppState::BirdSelection)))
        .add_systems(OnExit(AppState::BirdSelection), cleanup_selection_ui);

    // Game systems
    app.add_systems(OnEnter(AppState::Playing), (setup_player, grab_cursor))
        .add_systems(Update, (
            cursor_toggle,
            player_input,
            bird_movement,
            obstacle_collision,
            ai_bird_movement,
            camera_follow,
            energy_system,
            drafting_system,
            ai_bird_behavior,
            wing_flap,
            update_ui,
            check_goal,
            chunk_management,
        ).run_if(in_state(AppState::Playing)))
        .run();
}

// Components
#[derive(Component)]
struct Player;

// Bird types with different characteristics
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BirdType {
    Sparrow,   // Small, agile, fast flapping, poor glide
    Hawk,      // Medium, balanced
    Eagle,     // Large, slow turning, excellent glide, powerful
    Albatross, // Very large, best glide, slowest but highest speed
}

impl BirdType {
    fn name(&self) -> &'static str {
        match self {
            BirdType::Sparrow => "Sparrow",
            BirdType::Hawk => "Hawk",
            BirdType::Eagle => "Eagle",
            BirdType::Albatross => "Albatross",
        }
    }

    fn color(&self) -> Color {
        match self {
            BirdType::Sparrow => Color::srgb(0.6, 0.4, 0.2),   // Brown
            BirdType::Hawk => Color::srgb(0.5, 0.3, 0.2),      // Dark brown
            BirdType::Eagle => Color::srgb(0.2, 0.2, 0.3),     // Dark gray/blue
            BirdType::Albatross => Color::srgb(0.9, 0.9, 0.95), // White
        }
    }

    fn scale(&self) -> f32 {
        match self {
            BirdType::Sparrow => 0.6,
            BirdType::Hawk => 1.0,
            BirdType::Eagle => 1.4,
            BirdType::Albatross => 1.8,
        }
    }
}

// Stats that vary by bird type
#[derive(Component, Clone)]
struct BirdStats {
    bird_type: BirdType,
    // Gliding
    glide_efficiency: f32,      // Lower = less gravity when gliding (0.5-2.0)
    min_glide_speed: f32,       // Minimum speed for good glide
    perfect_glide_speed: f32,   // Speed for perfect glide
    // Turning
    turn_rate: f32,             // How fast bird can turn (mouse sensitivity)
    roll_rate: f32,             // Auto-roll speed
    // Flapping
    flap_thrust: f32,           // Force per flap
    flap_energy_cost: f32,      // Energy per flap
    flap_cooldown: f32,         // Time between flaps
    // Speed
    max_dive_speed: f32,        // Terminal velocity when diving
    acceleration: f32,          // Dive acceleration
    // Walking
    walk_speed: f32,
}

impl BirdStats {
    fn for_type(bird_type: BirdType) -> Self {
        match bird_type {
            BirdType::Sparrow => BirdStats {
                bird_type,
                glide_efficiency: 1.8,      // Poor glide
                min_glide_speed: 10.0,      // Increased from 8.0
                perfect_glide_speed: 32.0,  // Higher cruise speed (increased from 25.0)
                turn_rate: 1.5,             // Very agile
                roll_rate: 4.0,
                flap_thrust: 13.5,          // 1.5x power (9.0 * 1.5)
                flap_energy_cost: 5.0,      // Same cost for all birds
                flap_cooldown: 0.15,        // Fast flapping
                max_dive_speed: 90.0,       // Increased from 80.0
                acceleration: 35.0,         // Increased from 30.0 (20.0 * 1.75)
                walk_speed: 6.0,            // Quick walker
            },
            BirdType::Hawk => BirdStats {
                bird_type,
                glide_efficiency: 1.0,      // Balanced
                min_glide_speed: 10.0,
                perfect_glide_speed: 20.0,
                turn_rate: 1.0,
                roll_rate: 3.0,
                flap_thrust: 12.0,          // 8.0 * 1.5
                flap_energy_cost: 5.0,      // Same cost for all birds
                flap_cooldown: 0.25,
                max_dive_speed: 95.0,
                acceleration: 37.5,          // 25.0 * 1.5
                walk_speed: 5.0,
            },
            BirdType::Eagle => BirdStats {
                bird_type,
                glide_efficiency: 0.6,      // Good glide
                min_glide_speed: 12.0,
                perfect_glide_speed: 25.0,
                turn_rate: 0.7,             // Slower turning
                roll_rate: 2.0,
                flap_thrust: 18.0,          // Powerful flaps (12.0 * 1.5)
                flap_energy_cost: 5.0,      // Same cost for all birds
                flap_cooldown: 0.4,         // Slow flapping
                max_dive_speed: 110.0,
                acceleration: 45.0,          // 30.0 * 1.5
                walk_speed: 4.0,            // Slow walker
            },
            BirdType::Albatross => BirdStats {
                bird_type,
                glide_efficiency: 0.3,      // Excellent glide
                min_glide_speed: 15.0,
                perfect_glide_speed: 30.0,
                turn_rate: 0.5,             // Very slow turning
                roll_rate: 1.5,
                flap_thrust: 22.5,          // Very powerful (15.0 * 1.5)
                flap_energy_cost: 5.0,      // Same cost for all birds
                flap_cooldown: 0.6,         // Very slow flapping
                max_dive_speed: 130.0,
                acceleration: 52.5,          // 35.0 * 1.5
                walk_speed: 3.0,            // Slowest walker
            },
        }
    }
}

#[derive(Component)]
struct Bird {
    speed: f32,
    pitch: f32,
    yaw: f32,
    roll: f32,
    velocity: Vec3,  // Full 3D velocity for physics
    grounded: bool,  // Is bird on ground/perched
    damage_timer: f32,  // Timer for damage animation
    walk_timer: f32,  // Timer for walking animation
    is_walking: bool,  // Currently walking
}

#[derive(Component)]
struct Obstacle {
    half_extents: Vec3,  // Half-size of the bounding box
}

#[derive(Component)]
struct Energy {
    current: f32,
    max: f32,
    _drain_rate: f32,
}

#[derive(Component)]
struct Drafting {
    is_drafting: bool,
    draft_bonus: f32,
}

#[derive(Component)]
struct AiBird {
    target_height: f32,
    wander_timer: f32,
    wander_direction: f32,
}

#[derive(Component)]
struct Goal;

#[derive(Component)]
struct EnergyBar;

#[derive(Component)]
struct DraftIndicator;

#[derive(Component)]
struct DistanceText;

#[derive(Component)]
struct SelectionUI;

#[derive(Component)]
struct BirdButton(BirdType);

#[derive(Component)]
struct Wing {
    is_left: bool,
    _base_x: f32,
}

#[derive(Component)]
enum BodyPart {
    Body,
    Head,
    Wing,
    Tail,
}

// Chunk and LOD components
#[derive(Component)]
struct Chunk {
    x: i32,
    z: i32,
    lod_level: u8,
}

#[derive(Component)]
struct VoxelChunk;  // Marker for voxel chunk entities

#[derive(Resource)]
struct ChunkManager {
    loaded_chunks: std::collections::HashSet<(i32, i32, u8)>,
    chunk_size: f32,
    view_distance: f32,
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self {
            loaded_chunks: std::collections::HashSet::new(),
            chunk_size: 500.0,  // 500x500 units per chunk (increased for better coverage)
            view_distance: 5000.0,  // Load chunks within 5000 units (10 chunks radius)
        }
    }
}

#[derive(Resource, Default)]
struct FlapState {
    timer: f32,
    cooldown: f32,  // Time until next flap allowed
    space_held: bool,
    wings_closed_time: f32,  // How long wings have been closed
}

fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();

    // === ENVIRONMENT ===

    // Materials for chunk generation (kept for chunk system)
    let _grass_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.52, 0.25),
        perceptual_roughness: 0.9,
        ..default()
    });
    let _dirt_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.3, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });
    
    // === BIG FLOATING ISLAND ===
    // Create a large floating island as the main play area with collision on all sides
    let island_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.52, 0.25),
        perceptual_roughness: 0.9,
        ..default()
    });
    

    let island_bottom_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.25, 0.2),
        perceptual_roughness: 0.9,
        ..default()
    });
    
    // Island dimensions: 2000x2000 units, 20 units thick
    let island_width = 2000.0;
    let island_length = 2000.0;
    let island_height = 20.0;
    let island_y_position = 100.0;  // Floating high in the sky
    let island_surface_y = island_y_position + island_height;  // Top surface at y=120
    
    // Island top (main surface)
    let _island_top = commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(island_width, island_height, island_length)),
            material: island_material,
            transform: Transform::from_xyz(0.0, island_y_position + island_height/2.0, 0.0),
            ..default()
        },
        Obstacle {
            half_extents: Vec3::new(island_width/2.0, island_height/2.0, island_length/2.0),
        },
    )).id();
    
    // Island bottom (collision for flying underneath)
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(island_width, 5.0, island_length)),
            material: island_bottom_material,
            transform: Transform::from_xyz(0.0, island_y_position - island_height/2.0 - 2.5, 0.0),
            ..default()
        },
        Obstacle {
            half_extents: Vec3::new(island_width/2.0, 2.5, island_length/2.0),
        },
    ));
    

    
    // Disable or limit chunk generation for the island area
    // We'll keep chunk manager but reduce its view distance to avoid generating floor chunks
    
    // Voxel river with winding path
    let water_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.5, 0.7, 0.8),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        metallic: 0.3,
        ..default()
    });
    let river_bed_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.25, 0.15),
        perceptual_roughness: 0.9,
        ..default()
    });
    
    // Create winding river using voxel cubes
    let river_voxel = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let river_width = 15.0;
    let river_length = 800.0;
    let river_segments = 80;
    
    for i in 0..river_segments {
        let t = i as f32 / river_segments as f32;
        let z = t * river_length - 100.0;
        let x = (t * 4.0).sin() * 40.0 - 30.0; // Winding path
        
        // River bed (brown voxels at bottom)
        for dx in -river_width as i32..=river_width as i32 {
            for dz in -1..=1 {
                let x_pos = x + dx as f32 * 1.5;
                let z_pos = z + dz as f32 * 1.5;
                commands.spawn(PbrBundle {
                    mesh: river_voxel.clone(),
                    material: river_bed_material.clone(),
                    transform: Transform::from_xyz(x_pos, island_surface_y - 0.5, z_pos)
                        .with_scale(Vec3::new(1.5, 0.5, 1.5)),
                    ..default()
                });
            }
        }
        
        // Water surface (blue voxels)
        for dx in -river_width as i32 / 2..=river_width as i32 / 2 {
            for dz in -1..=1 {
                let x_pos = x + dx as f32 * 1.5;
                let z_pos = z + dz as f32 * 1.5;
                commands.spawn(PbrBundle {
                    mesh: river_voxel.clone(),
                    material: water_material.clone(),
                    transform: Transform::from_xyz(x_pos, island_surface_y + 0.1, z_pos)
                        .with_scale(Vec3::new(1.5, 0.2, 1.5)),
                    ..default()
                });
            }
        }
        
        // River banks with grass
        let bank_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.45, 0.2),
            perceptual_roughness: 0.9,
            ..default()
        });
        
        for side in [-1, 1] {
            let bank_x = x + side as f32 * (river_width + 2.0);
            for dz in -2..=2 {
                let z_pos = z + dz as f32 * 1.5;
                commands.spawn((
                    PbrBundle {
                        mesh: river_voxel.clone(),
                        material: bank_material.clone(),
                        transform: Transform::from_xyz(bank_x, island_surface_y + 0.5, z_pos)
                            .with_scale(Vec3::new(2.0, 1.0, 1.5)),
                        ..default()
                    },
                    Obstacle {
                        half_extents: Vec3::new(1.0, 0.5, 0.75),
                    },
                ));
            }
        }
    }

    // Voxel trees
    let trunk_mesh = meshes.add(Cuboid::new(1.0, 4.0, 1.0));
    let trunk_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.3, 0.15),
        ..default()
    });
    let leaves_mesh = meshes.add(Cuboid::new(3.0, 3.0, 3.0));
    let leaves_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.5, 0.15),
        ..default()
    });
    let dark_leaves = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.4, 0.1),
        ..default()
    });

    // Spawn trees along the path
    for _ in 0..80 {
        let x: f32 = rng.gen_range(-800.0..800.0);
        let z: f32 = rng.gen_range(-800.0..800.0);
        // Avoid placing trees in the center flight path
        if x.abs() < 25.0 { continue; }

        let trunk_height = rng.gen_range(8.0..20.0);  // Much taller trunks
        let tree_top = trunk_height + 12.0;  // Top of leaves
        let _tree_center_y = tree_top / 2.0;
        let tree_width = trunk_height * 0.4;  // Wider trees

        // Trunk collision - half_extents should be half the visual size
        let trunk_width = tree_width * 0.3;
        let trunk_visual_scale = Vec3::new(trunk_width, trunk_height / 4.0, trunk_width);
        // Visual size: trunk_width x trunk_height x trunk_width (mesh is 1x4x1 * scale)
        // Half extents = half of visual size
        let trunk_half_extents = Vec3::new(trunk_width / 2.0, trunk_height / 2.0, trunk_width / 2.0);
        commands.spawn((
            PbrBundle {
                mesh: trunk_mesh.clone(),
                material: trunk_material.clone(),
                transform: Transform::from_xyz(x, island_surface_y + trunk_height / 2.0, z)
                    .with_scale(trunk_visual_scale),
                ..default()
            },
            Obstacle { half_extents: trunk_half_extents },
        ));

        // Leaves layers
        let leaf_mat = if rng.gen_bool(0.5) { leaves_material.clone() } else { dark_leaves.clone() };
        for layer in 0..4 {
            let size = (tree_width * 2.0) - layer as f32 * 1.5;
            let leaf_scale = Vec3::splat(size / 3.0);
            // Visual size: size x size x size (mesh is 3x3x3 * scale of size/3)
            // Half extents = half of visual size
            let leaf_half_extents = Vec3::splat(size / 2.0);
            let leaf_y = island_surface_y + trunk_height + 2.0 + layer as f32 * 3.0;
            commands.spawn((
                PbrBundle {
                    mesh: leaves_mesh.clone(),
                    material: leaf_mat.clone(),
                    transform: Transform::from_xyz(x, leaf_y, z)
                        .with_scale(leaf_scale),
                    ..default()
                },
                Obstacle { half_extents: leaf_half_extents },
            ));
        }
    }

    // Mountains in background
    let mountain_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let mountain_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.35, 0.3),
        ..default()
    });
    let snow_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.95, 0.98),
        ..default()
    });

    for _ in 0..15 {
        let x = rng.gen_range(-800.0..800.0);
        let z = rng.gen_range(-800.0..800.0);
        let height = rng.gen_range(20.0..40.0);
        let width = rng.gen_range(15.0..30.0);
        // Mountain body
        commands.spawn((
            PbrBundle {
                mesh: mountain_mesh.clone(),
                material: mountain_material.clone(),
                transform: Transform::from_xyz(x, island_surface_y + height / 2.0, z)
                    .with_scale(Vec3::new(width, height, width)),
                ..default()
            },
            Obstacle {
                half_extents: Vec3::new(width / 2.0, height / 2.0, width / 2.0),
            },
        ));
        // Snow cap (visual only)
        commands.spawn(PbrBundle {
            mesh: mountain_mesh.clone(),
            material: snow_material.clone(),
            transform: Transform::from_xyz(x, island_surface_y + height * 0.85, z)
                .with_scale(Vec3::new(width * 0.6, height * 0.3, width * 0.6)),
            ..default()
        });
    }

    // Clouds
    let cloud_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let cloud_material = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.85),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });

    for _ in 0..30 {
        let x = rng.gen_range(-800.0..800.0);
        let y = island_surface_y + rng.gen_range(20.0..60.0);
        let z = rng.gen_range(-800.0..800.0);
        let size = rng.gen_range(8.0..20.0);

        // Cloud puffs
        for _ in 0..5 {
            let ox = rng.gen_range(-size..size);
            let oy = rng.gen_range(-2.0..2.0);
            let oz = rng.gen_range(-size * 0.5..size * 0.5);
            let puff_size = rng.gen_range(4.0..10.0);
            commands.spawn(PbrBundle {
                mesh: cloud_mesh.clone(),
                material: cloud_material.clone(),
                transform: Transform::from_xyz(x + ox, y + oy, z + oz)
                    .with_scale(Vec3::new(puff_size, puff_size * 0.5, puff_size * 0.7)),
                ..default()
            });
        }
    }

    // Hills/terrain variation
    let hill_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.32, 0.55, 0.28),
        ..default()
    });
    for _ in 0..20 {
        let x = rng.gen_range(-800.0..800.0);
        let z = rng.gen_range(-800.0..800.0);
        let size = rng.gen_range(20.0..50.0);
        let height = rng.gen_range(3.0..10.0);
        commands.spawn(PbrBundle {
            mesh: meshes.add(Sphere::new(1.0)),
            material: hill_material.clone(),
            transform: Transform::from_xyz(x, island_surface_y - size * 0.3 + height, z)
                .with_scale(Vec3::new(size, height, size)),
            ..default()
        });
    }

    // === VOXEL BIRD MESHES ===
    let voxel = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // AI birds (blue jays style)
    let ai_body = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.5, 0.8),
        ..default()
    });
    let ai_wing = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.7),
        ..default()
    });
    let ai_belly = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.9),
        ..default()
    });

    for i in 0..5 {
        let x = rng.gen_range(-800.0..800.0);
        let z = rng.gen_range(-800.0..800.0);
        let height = rng.gen_range(130.0..150.0);

        commands.spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: ai_body.clone(),
                transform: Transform::from_xyz(x, height, z)

                    .with_scale(Vec3::new(0.6, 1.0, 0.6)),
                ..default()
            },
            Bird {
                speed: 12.0 + i as f32 * 0.5,
                pitch: 0.0,
                yaw: 0.0,
                roll: 0.0,
                velocity: Vec3::new(0.0, 0.0, 12.0 + i as f32 * 0.5),
                grounded: false,
                damage_timer: 0.0,
                walk_timer: 0.0,
                is_walking: false,
            },
            AiBird {
                target_height: height,
                wander_timer: rng.gen_range(0.0..3.0),
                wander_direction: rng.gen_range(-0.5..0.5),
            },
        )).with_children(|parent| {
            // Head
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: ai_body.clone(),
                transform: Transform::from_xyz(0.0, 0.0, 0.5)
                    .with_scale(Vec3::splat(0.5)),
                ..default()
            });
            // Belly
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: ai_belly.clone(),
                transform: Transform::from_xyz(0.0, -0.2, 0.0)
                    .with_scale(Vec3::new(0.5, 0.3, 0.8)),
                ..default()
            });
            // Wings
            parent.spawn((
                PbrBundle {
                    mesh: voxel.clone(),
                    material: ai_wing.clone(),
                    transform: Transform::from_xyz(-0.6, 0.0, 0.0)
                        .with_scale(Vec3::new(0.8, 0.08, 0.4)),
                    ..default()
                },
                Wing { is_left: true, _base_x: -0.6 },
            ));
            parent.spawn((
                PbrBundle {
                    mesh: voxel.clone(),
                    material: ai_wing.clone(),
                    transform: Transform::from_xyz(0.6, 0.0, 0.0)
                        .with_scale(Vec3::new(0.8, 0.08, 0.4)),
                    ..default()
                },
                Wing { is_left: false, _base_x: 0.6 },
            ));
        });
    }

    // Goal - Nest in a tree
    let nest_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.35, 0.2),
        ..default()
    });
    // Goal - Nest in a tree (random location on island)
    // Generate random position within island bounds, at least 300 units from start
    let mut goal_x: f32;
    let mut goal_z: f32;
    loop {
        goal_x = rng.gen_range(-800.0..800.0);
        goal_z = rng.gen_range(-800.0..800.0);
        if goal_x * goal_x + goal_z * goal_z > 300.0 * 300.0 {
            break;
        }
    }
    info!("Goal spawned at ({:.1}, {:.1})", goal_x, goal_z);
    println!("Goal spawned at ({:.1}, {:.1})", goal_x, goal_z);
    let trunk_scale = Vec3::new(2.0, 3.0, 2.0);
    let trunk_height = 4.0 * trunk_scale.y; // mesh height = 4
    let trunk_bottom_y = island_surface_y;
    let trunk_center_y = trunk_bottom_y + trunk_height / 2.0;
    
    let leaves_scale = Vec3::new(3.0, 2.0, 3.0);
    let leaves_height = 3.0 * leaves_scale.y; // mesh height = 3
    let leaves_center_y = trunk_center_y + trunk_height / 2.0 + leaves_height / 2.0 - 2.0;
    
    let nest_y = trunk_center_y + trunk_height / 2.0 - 1.0;
    let glow_y = nest_y + 1.0;
    
    // Tree for nest
    commands.spawn((
        PbrBundle {
            mesh: trunk_mesh.clone(),
            material: trunk_material.clone(),
            transform: Transform::from_xyz(goal_x, trunk_center_y, goal_z)
                .with_scale(trunk_scale),
            ..default()
        },
        Obstacle {
            half_extents: Vec3::new(trunk_scale.x * 0.5, trunk_scale.y * 2.0, trunk_scale.z * 0.5),
        },
    ));
    commands.spawn((
        PbrBundle {
            mesh: leaves_mesh.clone(),
            material: leaves_material.clone(),
            transform: Transform::from_xyz(goal_x, leaves_center_y, goal_z)
                .with_scale(leaves_scale),
            ..default()
        },
        Obstacle {
            half_extents: Vec3::new(leaves_scale.x * 1.5, leaves_scale.y * 1.5, leaves_scale.z * 1.5),
        },
    ));
    // Nest
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Torus::new(1.5, 2.5)),
            material: nest_material,
            transform: Transform::from_xyz(goal_x, nest_y, goal_z)
                .with_rotation(Quat::from_rotation_x(PI / 2.0)),
            ..default()
        },
        Goal,
    ));
    // Nest glow indicator
    commands.spawn(PbrBundle {
        mesh: meshes.add(Sphere::new(1.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.9, 0.5, 0.5),
            emissive: LinearRgba::new(2.0, 1.5, 0.5, 1.0),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        transform: Transform::from_xyz(goal_x, glow_y, goal_z),
        ..default()
    });

    // Lighting - Sun
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            color: Color::srgb(1.0, 0.95, 0.8),
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -PI / 3.0,
            PI / 4.0,
            0.0,
        )),
        ..default()
    });

    // Ambient light for softer shadows
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.6, 0.7, 0.9),
        brightness: 200.0,
    });

    // Camera (initial position, will be updated by camera_follow)
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 40.0, -50.0)
            .looking_at(Vec3::new(0.0, 30.0, 20.0), Vec3::Y),
        projection: PerspectiveProjection {
            near: 1.0,
            far: 50000.0,
            ..default()
        }.into(),
        ..default()
    });

    // UI
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        },
        ..default()
    }).with_children(|parent| {
        // Energy bar background
        parent.spawn(NodeBundle {
            style: Style {
                width: Val::Px(300.0),
                height: Val::Px(30.0),
                ..default()
            },
            background_color: Color::srgba(0.2, 0.2, 0.2, 0.8).into(),
            ..default()
        }).with_children(|parent| {
            // Energy bar fill
            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    background_color: Color::srgb(0.2, 0.8, 0.2).into(),
                    ..default()
                },
                EnergyBar,
            ));
        });

        // Draft indicator
        parent.spawn((
            TextBundle::from_section(
                "",
                TextStyle {
                    font_size: 24.0,
                    color: Color::srgb(0.2, 0.8, 1.0),
                    ..default()
                },
            ).with_style(Style {
                margin: UiRect::top(Val::Px(10.0)),
                ..default()
            }),
            DraftIndicator,
        ));

        // Distance to goal
        parent.spawn((
            TextBundle::from_section(
                "Distance: 500m",
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            ).with_style(Style {
                margin: UiRect::top(Val::Px(10.0)),
                ..default()
            }),
            DistanceText,
        ));

    });
}

fn grab_cursor(mut windows: Query<&mut Window>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
}

fn cursor_toggle(
    mut windows: Query<&mut Window>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let Ok(mut window) = windows.get_single_mut() else { return };

    // ESC releases cursor
    if keyboard.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }

    // Click to re-capture
    if mouse.just_pressed(MouseButton::Left) && window.cursor.grab_mode == CursorGrabMode::None {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
}

fn player_input(
    mut query: Query<(&mut Bird, &BirdStats), With<Player>>,
    mut motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    time: Res<Time>,
) {
    let Ok((mut bird, stats)) = query.get_single_mut() else { return };
    let dt = time.delta_seconds();

    // Mouse controls pitch and yaw - sensitivity affected by bird's turn_rate
    let base_sensitivity = 0.0015;
    let mouse_sensitivity = base_sensitivity * stats.turn_rate;
    let mut yaw_delta = 0.0;

    for event in motion_events.read() {
        yaw_delta = -event.delta.x * mouse_sensitivity;
        bird.yaw += yaw_delta;
        bird.pitch = (bird.pitch + event.delta.y * mouse_sensitivity).clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
    }

    // Auto-roll when turning: bank into the turn
    let target_roll = -yaw_delta * 30.0;  // Roll in direction of turn (reduced amount)
    bird.roll = bird.roll + (target_roll - bird.roll) * stats.roll_rate * dt;

    // Gradually return roll to level
    bird.roll *= 0.95;

    // Auto-return to optimal glide angle when not moving mouse
    // Optimal angle is level (0.0) for comfortable gliding
    let optimal_pitch = 0.0;
    let no_input = yaw_delta.abs() < 0.0001;

    if no_input {
        // Smoothly return to optimal glide angle
        bird.pitch = bird.pitch + (optimal_pitch - bird.pitch) * 2.0 * dt;
    }
}

fn bird_movement(
    mut query: Query<(&mut Bird, &mut Transform, Option<&BirdStats>), Without<AiBird>>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<Bird>)>,
    obstacle_query: Query<(&Transform, &Obstacle), Without<Bird>>,
    flap_state: Res<FlapState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    // Drag coefficient will be calculated per-bird based on size

    // Get camera forward and right vectors for movement reference
    let (cam_forward, cam_right) = camera_query
        .get_single()
        .map(|t| {
            let forward = t.forward();
            let right = t.right();
            // Project onto XZ plane and normalize
            let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
            let right_xz = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
            (forward_xz, right_xz)
        })
        .unwrap_or((Vec3::NEG_Z, Vec3::X));

    // Default stats for birds without BirdStats component
    let default_stats = BirdStats::for_type(BirdType::Hawk);

    for (mut bird, mut transform, stats_opt) in query.iter_mut() {
        let stats = stats_opt.unwrap_or(&default_stats);

        // Handle grounded walking with WASD
        if bird.grounded {
            // Check if still over a surface (ground or obstacle)
            let bird_pos = transform.translation;
            let mut has_ground_support = bird_pos.y <= 1.0;  // Near ground level

            if !has_ground_support {
                // Check if over any obstacle
                for (obs_transform, obstacle) in obstacle_query.iter() {
                    let obs_pos = obs_transform.translation;
                    let half = obstacle.half_extents;
                    let obstacle_top = obs_pos.y + half.y;

                    // Check if bird is within XZ bounds of obstacle and near its top
                    // Use same tolerance as landing check (1.0) to prevent stuck loop
                    let in_xz_bounds = bird_pos.x > obs_pos.x - half.x - 1.0
                        && bird_pos.x < obs_pos.x + half.x + 1.0
                        && bird_pos.z > obs_pos.z - half.z - 1.0
                        && bird_pos.z < obs_pos.z + half.z + 1.0;

                    // Match landing tolerance: y between obstacle_top - 2.0 and obstacle_top + 3.0
                    let near_top = bird_pos.y < obstacle_top + 3.0 && bird_pos.y > obstacle_top - 2.0;

                    if in_xz_bounds && near_top {
                        has_ground_support = true;
                        break;
                    }
                }
            }

            // If no ground support, start falling
            if !has_ground_support {
                bird.grounded = false;
                bird.velocity = Vec3::new(0.0, -2.0, 0.0);  // Start falling
                continue;  // Skip walking code, let physics take over
            }

            let walk_speed = stats.walk_speed;
            let mut world_move = Vec3::ZERO;
            let mut should_turn = false;

            // Get WASD input (relative to camera view)
            if keyboard.pressed(KeyCode::KeyW) {
                world_move += cam_forward;  // Forward where camera looks
                should_turn = true;  // Only turn when moving forward
            }
            if keyboard.pressed(KeyCode::KeyS) {
                world_move -= cam_forward;  // Backward (no turn, walk backward)
            }
            if keyboard.pressed(KeyCode::KeyA) {
                world_move -= cam_right;  // Sidestep left (no turn)
            }
            if keyboard.pressed(KeyCode::KeyD) {
                world_move += cam_right;  // Sidestep right (no turn)
            }

            // Check if walking
            bird.is_walking = world_move.length_squared() > 0.0;

            if bird.is_walking {
                // Normalize movement
                world_move = world_move.normalize();

                // Move horizontally only (XZ plane)
                transform.translation.x += world_move.x * walk_speed * dt;
                transform.translation.z += world_move.z * walk_speed * dt;

                // Update walk animation timer
                bird.walk_timer += dt * 12.0;  // Fast bobbing

                // Only rotate to face direction when moving forward (W key)
                if should_turn {
                    let target_yaw = cam_forward.x.atan2(cam_forward.z);
                    // Smoothly interpolate yaw (handle angle wrapping)
                    let mut yaw_diff = target_yaw - bird.yaw;
                    while yaw_diff > PI { yaw_diff -= 2.0 * PI; }
                    while yaw_diff < -PI { yaw_diff += 2.0 * PI; }
                    bird.yaw += yaw_diff * 8.0 * dt;
                }

                // Walking pose: upright with slight forward lean and body bob
                let bob_angle = (bird.walk_timer).sin() * 0.1;  // Body tilts up/down
                let walk_rotation = Quat::from_rotation_y(bird.yaw)
                    * Quat::from_rotation_x(-PI / 2.0 + 0.15 + bob_angle)  // Upright + forward lean + bob
                    * Quat::from_rotation_z((bird.walk_timer).sin() * 0.08);  // Side sway
                transform.rotation = transform.rotation.slerp(walk_rotation, 10.0 * dt);
            } else {
                // Reset walk timer when stopped
                bird.walk_timer = 0.0;

                // Idle standing pose: upright with head on top
                let idle_rotation = Quat::from_rotation_y(bird.yaw)
                    * Quat::from_rotation_x(-PI / 2.0);
                transform.rotation = transform.rotation.slerp(idle_rotation, 5.0 * dt);
            }
            continue;
        }
        // Bird's orientation quaternion
        let bird_rotation = Quat::from_rotation_y(bird.yaw)
            * Quat::from_rotation_x(bird.pitch)
            * Quat::from_rotation_z(bird.roll);

        // Forward direction (where bird is pointing)
        let forward = bird_rotation * Vec3::Z;
        // Up direction (bird's local up, for lift)
        let up = bird_rotation * Vec3::Y;

        // Current speed
        let speed = bird.velocity.length();
        bird.speed = speed;

        // Horizontal speed for lift calculation
        let horizontal_speed = Vec3::new(bird.velocity.x, 0.0, bird.velocity.z).length();

        // === SPEED-BASED GRAVITY & LIFT ===
        if flap_state.space_held {
            // Wings closed: gravity increases over time (stoop)
            // Smaller birds reach max gravity faster
            let (max_gravity, ramp_time) = match stats.bird_type {
                BirdType::Sparrow => (1000.0, 3.0),
                BirdType::Hawk => (1000.0, 4.5),
                BirdType::Eagle => (1000.0, 5.5),
                BirdType::Albatross => (1000.0, 7.0),
            };
            let gravity_factor = (flap_state.wings_closed_time / ramp_time).min(1.0);
            bird.velocity.y -= max_gravity * gravity_factor * dt;

            // Wings closed: convert falling energy to forward speed
            // Smaller birds gain speed faster
            let fall_speed = (-bird.velocity.y).max(0.0);  // How fast falling
            if fall_speed > 1.0 {
                // Get horizontal forward direction
                let horizontal_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
                let size_multiplier = match stats.bird_type {
                    BirdType::Sparrow => 10.0,
                    BirdType::Hawk => 6.0,
                    BirdType::Eagle => 4.0,
                    BirdType::Albatross => 3.0,
                };
                let acceleration = fall_speed * size_multiplier;
                bird.velocity += horizontal_forward * acceleration * dt;

                // Cap at max dive speed
                if bird.velocity.length() > stats.max_dive_speed {
                    bird.velocity = bird.velocity.normalize_or_zero() * stats.max_dive_speed;
                }
            }
        } else {
            // Wings open: gravity depends on horizontal speed
            // Fast = less fall, slow = fall faster
            // glide_efficiency: lower = better glide (less gravity)
            let min_speed_for_glide = stats.min_glide_speed;
            let perfect_glide_speed = stats.perfect_glide_speed;
            let base_gravity = 15.0 * stats.glide_efficiency;

            if horizontal_speed >= perfect_glide_speed {
                // At good speed: gentle fall
                let wing_level = up.y.max(0.0);
                let min_fall = 5.0 * stats.glide_efficiency;
                bird.velocity.y -= min_fall * (1.0 - wing_level * 0.5) * dt;
            } else if horizontal_speed >= min_speed_for_glide {
                // Between min and perfect: proportional fall
                let glide_quality = (horizontal_speed - min_speed_for_glide) / (perfect_glide_speed - min_speed_for_glide);
                let wing_level = up.y.max(0.0);
                let gravity = base_gravity * (1.0 - glide_quality * 0.5 * wing_level);
                bird.velocity.y -= gravity * dt;
            } else {
                // Too slow - fall much faster
                let slow_factor = (min_speed_for_glide - horizontal_speed) / min_speed_for_glide;
                let gravity = base_gravity + slow_factor * 50.0;
                bird.velocity.y -= gravity * dt;
            }
        }

        // === PITCH-BASED SPEED CHANGE (only when gliding with wings open) ===
        if !flap_state.space_held {
            // forward.y < 0 means diving (nose down), forward.y > 0 means climbing (nose up)
            // Diving: convert potential energy to speed (accelerate)
            // Climbing: convert speed to potential energy (decelerate)
            let pitch_factor = -forward.y;  // positive when diving, negative when climbing

            if pitch_factor > 0.05 {
                // Diving - accelerate progressively (even slight dives gain speed)
                let acceleration = pitch_factor * stats.acceleration;
                bird.velocity += forward * acceleration * dt;
                // Cap at max dive speed
                if bird.velocity.length() > stats.max_dive_speed {
                    bird.velocity = bird.velocity.normalize_or_zero() * stats.max_dive_speed;
                }
            } else if pitch_factor < -0.3 {
                // Only decelerate when climbing steeply (more than ~18 degrees up)
                let climb_amount = (-pitch_factor - 0.3).max(0.0);
                let deceleration = climb_amount * 6.0;  // Gentle deceleration
                let speed_loss = (deceleration * dt).min(speed * 0.2);  // Max 20% per frame
                if speed > 5.0 {
                    let vel_dir = bird.velocity.normalize_or_zero();
                    bird.velocity -= vel_dir * speed_loss;
                }
            }
        }

        // === DRAG ===
        // Smaller birds lose speed faster, bigger birds preserve speed longer
        // Drag only applies above cruise speed (perfect_glide_speed)
        let cruise_speed = stats.perfect_glide_speed;
        if speed > cruise_speed && !flap_state.space_held {
            let base_drag = match stats.bird_type {
                BirdType::Sparrow => 0.01_f32,
                BirdType::Hawk => 0.003,
                BirdType::Eagle => 0.001,
                BirdType::Albatross => 0.0003,
            };
            let drag = bird.velocity * bird.velocity.length() * base_drag;
            bird.velocity -= drag * dt;

            // Don't let drag reduce speed below cruise speed
            let current_speed = bird.velocity.length();
            if current_speed < cruise_speed && current_speed > 0.1 {
                bird.velocity = bird.velocity.normalize_or_zero() * cruise_speed;
            }
        } else if flap_state.space_held {
            // When diving, still apply some drag
            let drag = bird.velocity * bird.velocity.length() * 0.005;
            bird.velocity -= drag * dt;
        }

        // === YAW TURNING ===
        if speed > 1.0 {
            let target_vel = forward * speed;
            bird.velocity = bird.velocity.lerp(target_vel, 2.0 * dt);
        }

        // === APPLY VELOCITY ===
        transform.translation += bird.velocity * dt;

        // Ground collision is handled by obstacle_collision system
        // No hardcoded ground level - terrain is made of voxels

        // === ROTATE BIRD ===
        if bird.grounded {
            // Idle pose: standing upright (undo the flight pitch rotation)
            let idle_rotation = Quat::from_rotation_y(bird.yaw);  // Standing upright
            transform.rotation = transform.rotation.slerp(idle_rotation, 5.0 * dt);
        } else {
            // Flying pose
            let mut target_rotation = Quat::from_rotation_y(bird.yaw)
                * Quat::from_rotation_x(bird.pitch)
                * Quat::from_rotation_z(bird.roll);

            // Damage shake animation
            if bird.damage_timer > 0.0 {
                let shake = (bird.damage_timer * 40.0).sin() * 0.3;
                target_rotation = target_rotation * Quat::from_rotation_z(shake);
            }

            transform.rotation = transform.rotation.slerp(target_rotation, 5.0 * dt);
        }
    }
}

fn obstacle_collision(
    mut bird_query: Query<(&mut Bird, &mut Transform, &BirdStats), With<Player>>,
    obstacle_query: Query<(&Transform, &Obstacle), Without<Player>>,
    time: Res<Time>,
) {
    let Ok((mut bird, mut bird_transform, bird_stats)) = bird_query.get_single_mut() else { return };
    let dt = time.delta_seconds();

    // Update damage timer
    if bird.damage_timer > 0.0 {
        bird.damage_timer -= dt;
    }

    // Skip collision if grounded
    if bird.grounded { return; }

    let bird_pos = bird_transform.translation;
    // Scale collision radius based on bird size
    let bird_scale = bird_stats.bird_type.scale();
    let bird_radius = 2.4 * bird_scale;

    for (obs_transform, obstacle) in obstacle_query.iter() {
        let obs_pos = obs_transform.translation;
        let half = obstacle.half_extents;
        let obstacle_top = obs_pos.y + half.y;

        // Simple AABB + sphere collision check
        let closest_x = bird_pos.x.clamp(obs_pos.x - half.x, obs_pos.x + half.x);
        let closest_y = bird_pos.y.clamp(obs_pos.y - half.y, obs_pos.y + half.y);
        let closest_z = bird_pos.z.clamp(obs_pos.z - half.z, obs_pos.z + half.z);

        let distance = ((bird_pos.x - closest_x).powi(2)
            + (bird_pos.y - closest_y).powi(2)
            + (bird_pos.z - closest_z).powi(2))
        .sqrt();

        if distance > bird_radius {
            continue; // No collision
        }

        // Check if this is a landing situation (on top of obstacle)
        let in_xz_bounds = bird_pos.x > obs_pos.x - half.x - 1.0
            && bird_pos.x < obs_pos.x + half.x + 1.0
            && bird_pos.z > obs_pos.z - half.z - 1.0
            && bird_pos.z < obs_pos.z + half.z + 1.0;

        let bird_above_obstacle = bird_pos.y > obs_pos.y;  // Bird is above obstacle center
        let moving_down = bird.velocity.y < 0.0;  // Must be actually falling
        let is_slow = bird.velocity.length() < 5.0;  // Very slow (nearly stopped)

        // Debug: print collision with floor voxels


        if in_xz_bounds && bird_above_obstacle && (moving_down || is_slow) {

            // Land on top - scale landing height based on bird size
            let landing_height = 0.5;  // Small offset above obstacle
            bird_transform.translation.y = obstacle_top + landing_height;

            bird.velocity = Vec3::ZERO;

            bird.grounded = true;
            bird.pitch = 0.0;
            bird.roll = 0.0;
            return;  // Stop checking, we landed
        }

        // Skip side collision if near top (allow landing approach)
        if bird_pos.y > obstacle_top - 5.0 {
            continue;
        }

        // Side collision
        let speed = bird.velocity.length();
        let to_bird = (bird_pos - obs_pos).normalize_or_zero();

        if speed > 5.0 && bird.damage_timer <= 0.0 {
            bird.damage_timer = 0.5;
            let bounce_dir = Vec3::new(to_bird.x, 0.3, to_bird.z).normalize_or_zero();
            bird.velocity = bounce_dir * speed * 0.5;
            bird_transform.translation += to_bird * (bird_radius + 0.5);
        } else {
            bird_transform.translation += to_bird * 0.5;
            bird.velocity *= 0.5;
        }
    }
}

fn ai_bird_movement(
    mut query: Query<(&Bird, &mut Transform), With<AiBird>>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (bird, mut transform) in query.iter_mut() {
        // Calculate direction from pitch and yaw
        let direction = Vec3::new(
            bird.yaw.sin() * bird.pitch.cos(),
            -bird.pitch.sin(),
            bird.yaw.cos() * bird.pitch.cos(),
        ).normalize();

        // Move forward
        transform.translation += direction * bird.speed * dt;

        // Keep above ground
        transform.translation.y = transform.translation.y.max(2.0);

        // Rotate bird to face direction
        let target_rotation = Quat::from_rotation_y(bird.yaw)
            * Quat::from_rotation_x(bird.pitch)
            * Quat::from_rotation_z(bird.roll);
        transform.rotation = transform.rotation.slerp(target_rotation, 5.0 * dt);
    }
}

fn wing_flap(
    mut wing_query: Query<(&Wing, &mut Transform)>,
    mut head_query: Query<(&BodyPart, &mut Transform), Without<Wing>>,
    mut player_query: Query<(&mut Energy, &mut Bird, &BirdStats, &Children), With<Player>>,
    mut flap_state: ResMut<FlapState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {

    let flap_duration = 0.3;
    let wing_closed_angle = 1.5;  // Wings folded up when closed
    let dt = time.delta_seconds();

    // Get bird state for animation and player's children
    let player_data = player_query
        .get_single()
        .map(|(_, b, _, children)| (b.grounded, b.is_walking, b.walk_timer, children.iter().copied().collect::<Vec<_>>()))
        .ok();

    let (is_grounded, is_walking, walk_timer, player_children) = match player_data {
        Some((g, w, t, c)) => (g, w, t, c),
        None => {

            return;
        }
    };
    


    // Update cooldown timer
    if flap_state.cooldown > 0.0 {
        flap_state.cooldown -= dt;
    }

    // Track space held state - holding space always closes wings
    // Cooldown only affects whether a flap happens, not wing position
    let was_space_held = flap_state.space_held;
    if keyboard.pressed(KeyCode::Space) {
        flap_state.space_held = true;
        flap_state.wings_closed_time += dt;
        if !was_space_held {

        }
    } else {
        flap_state.space_held = false;
        flap_state.wings_closed_time = 0.0;  // Reset gravity ramp
        if was_space_held {

        }
    }

    // Cancel flap animation if pressing space during cooldown
    if keyboard.just_pressed(KeyCode::Space) && flap_state.cooldown > 0.0 {
        flap_state.timer = 0.0;
    }

    // Start new flap on space press (with cooldown check)
    if keyboard.just_pressed(KeyCode::Space) && flap_state.cooldown <= 0.0 {

        // Deduct energy and add thrust relative to bird rotation
        if let Ok((mut energy, mut bird, bird_stats, _)) = player_query.get_single_mut() {
            // Only start animation and cooldown if we actually flap
            flap_state.timer = flap_duration;
            flap_state.cooldown = bird_stats.flap_cooldown;  // Set cooldown based on bird type

            energy.current = (energy.current - bird_stats.flap_energy_cost).max(0.0);

            // Take off if grounded
            if bird.grounded {
                bird.grounded = false;
                bird.velocity = Vec3::new(0.0, 15.0, bird_stats.perfect_glide_speed);  // 1.5x takeoff impulse
                // Rotate velocity to face current yaw
                let yaw_rotation = Quat::from_rotation_y(bird.yaw);
                bird.velocity = yaw_rotation * bird.velocity;
            } else {
                // Calculate bird's local up direction based on rotation
                let bird_rotation = Quat::from_rotation_y(bird.yaw)
                    * Quat::from_rotation_x(bird.pitch)
                    * Quat::from_rotation_z(bird.roll);

                // Flap thrust goes in bird's local up direction
                let local_up = bird_rotation * Vec3::Y;
                // Also add some forward thrust
                let local_forward = bird_rotation * Vec3::Z;

                let thrust = bird_stats.flap_thrust;
                bird.velocity += local_up * thrust + local_forward * (thrust * 0.3);
            }
        }
    }

    // Update flap timer
    if flap_state.timer > 0.0 {
        flap_state.timer -= dt;
    }

    // Only update player's wings (not remote players)

    for &child in player_children.iter() {
        if let Ok((wing, mut transform)) = wing_query.get_mut(child) {

            // Base rotation: identity since wings are already flat/horizontal
            let base_rotation = Quat::IDENTITY;

            if is_grounded {
                let fold_angle = 1.2;
                if is_walking {
                    // Walking animation: wings swing back and forth alternately
                    let swing = (walk_timer).sin() * 0.4;
                    if wing.is_left {
                        // Left wing swings opposite to right
                        transform.rotation = base_rotation
                            * Quat::from_rotation_z(fold_angle)
                            * Quat::from_rotation_y(swing);
                    } else {
                        transform.rotation = base_rotation
                            * Quat::from_rotation_z(-fold_angle)
                            * Quat::from_rotation_y(-swing);
                    }
                } else {
                    // Wings folded at sides when standing still
                    if wing.is_left {
                        transform.rotation = base_rotation * Quat::from_rotation_z(fold_angle);
                    } else {
                        transform.rotation = base_rotation * Quat::from_rotation_z(-fold_angle);
                    }
                }
            } else if flap_state.timer > 0.0 {
                // Flap animation: quick downward stroke
                let progress = 1.0 - (flap_state.timer / flap_duration);
                let angle = (progress * PI).sin() * 0.8;


                if wing.is_left {
                    transform.rotation = base_rotation * Quat::from_rotation_z(angle);  // Flap down
                } else {
                    transform.rotation = base_rotation * Quat::from_rotation_z(-angle);
                }

            } else if flap_state.space_held {
                // Wings closed (folded up) while holding space

                if wing.is_left {
                    transform.rotation = base_rotation * Quat::from_rotation_z(wing_closed_angle);
                } else {
                    transform.rotation = base_rotation * Quat::from_rotation_z(-wing_closed_angle);
                }

            } else {
                // Wings open (gliding) when space is released
                transform.rotation = base_rotation;
            }
        }
    }

    // Update head rotation based on grounded state
    for &child in player_children.iter() {
        if let Ok((body_part, mut transform)) = head_query.get_mut(child) {
            if matches!(body_part, BodyPart::Head) {
                if is_grounded {
                    // When standing upright, rotate head forward so beak points forward
                    // The body is rotated -90° around X, so head needs +90° to face forward
                    let head_forward = Quat::from_rotation_x(PI / 2.0);
                    transform.rotation = transform.rotation.slerp(head_forward, 10.0 * dt);
                } else {
                    // In flight, head faces forward (no rotation needed)
                    transform.rotation = transform.rotation.slerp(Quat::IDENTITY, 10.0 * dt);
                }
            }
        }
    }
}

fn camera_follow(
    player_query: Query<(&Bird, &Transform), With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
    time: Res<Time>,
    mut frame_count: Local<u32>,
) {
    let Ok((bird, player_transform)) = player_query.get_single() else { return };
    let Ok(mut camera_transform) = camera_query.get_single_mut() else { return };
    let dt = time.delta_seconds();

    // Flight rotation (physics) - same as in bird_movement line 1050-1052
    let flight_rotation = if bird.grounded {
        // When grounded, no roll - just yaw and pitch for looking around
        Quat::from_rotation_y(bird.yaw) * Quat::from_rotation_x(bird.pitch)
    } else {
        Quat::from_rotation_y(bird.yaw)
            * Quat::from_rotation_x(bird.pitch)
            * Quat::from_rotation_z(bird.roll)
    };
    
    // Flight forward direction (where bird is actually flying)
    let flight_forward = flight_rotation * Vec3::Z;
    
    // Bird's local up direction (includes roll when flying)
    let bird_up = flight_rotation * Vec3::Y;
    
    // Camera position: behind bird in bird's local space
    // -Z is forward for bird, so behind = -forward * distance
    // +Y is up for bird, so above = up * height
    let camera_distance_behind = 45.0;  // Increased from 40.0 for better view
    let camera_height_above = 5.0;      // Reduced from 8.0 to center bird more
    
    let target_pos = player_transform.translation 
        - flight_forward * camera_distance_behind
        + bird_up * camera_height_above;
    
    // Camera rotation: looks where bird faces (camera's -Z points in bird's forward direction)
    // and rolls with bird. Need to rotate 180° around Y so camera looks forward.
    let target_rotation = flight_rotation * Quat::from_rotation_y(PI);
    
    // Smooth camera movement and rotation
    let smooth_factor = (10.0 * dt).min(1.0);
    camera_transform.translation = camera_transform.translation.lerp(target_pos, smooth_factor);
    camera_transform.rotation = camera_transform.rotation.slerp(target_rotation, smooth_factor);
    
    // Debug output (reduced frequency)
    *frame_count += 1;
    if *frame_count <= 5 || *frame_count % 120 == 0 {
        let cam_look_direction = target_rotation * Vec3::NEG_Z; // Camera actual forward (looking direction)
        let bird_to_camera = camera_transform.translation - player_transform.translation;
        let camera_relative = bird_to_camera.normalize_or_zero();
        println!("Camera[{}]: bird_pos={:?}, cam_pos={:?}, target_pos={:?}, bird(ypr)=({:.1},{:.1},{:.1})", 
            *frame_count, player_transform.translation, camera_transform.translation, target_pos,
            bird.yaw.to_degrees(), bird.pitch.to_degrees(), bird.roll.to_degrees());
        println!("  flight_forward={:?}, cam_look_dir={:?}, bird_up={:?}", 
            flight_forward, cam_look_direction, bird_up);
        println!("  bird_to_camera={:?}, dist={:.1}, relative={:?}", 
            bird_to_camera, bird_to_camera.length(), camera_relative);
    }
}

fn energy_system(
    mut query: Query<(&mut Energy, &Drafting, &BirdStats), With<Player>>,
    time: Res<Time>,
) {
    let Ok((mut energy, drafting, stats)) = query.get_single_mut() else { return };

    // Continuous energy regeneration - smaller birds recover faster
    let base_regen = match stats.bird_type {
        BirdType::Sparrow => 6.0,    // Fastest recovery
        BirdType::Hawk => 4.0,
        BirdType::Eagle => 2.5,
        BirdType::Albatross => 1.5,  // Slowest recovery
    };
    let draft_bonus = if drafting.is_drafting { drafting.draft_bonus } else { 0.0 };  // Bonus when drafting

    let regen_rate = base_regen + draft_bonus;
    energy.current = (energy.current + regen_rate * time.delta_seconds()).min(energy.max);
}

fn drafting_system(
    mut player_query: Query<(&Transform, &mut Drafting), With<Player>>,
    ai_query: Query<&Transform, (With<AiBird>, Without<Player>)>,
) {
    let Ok((player_transform, mut drafting)) = player_query.get_single_mut() else { return };
    let player_pos = player_transform.translation;

    drafting.is_drafting = false;

    for ai_transform in ai_query.iter() {
        let ai_pos = ai_transform.translation;

        // Check if player is behind an AI bird
        let to_ai = ai_pos - player_pos;
        let distance = to_ai.length();

        // Draft zone: behind the bird, within certain distance
        if distance < 8.0 && distance > 1.0 {
            // Check if we're roughly behind (AI is ahead in Z)
            if to_ai.z > 0.0 && to_ai.z.abs() > to_ai.x.abs() * 2.0 {
                // Check vertical alignment
                if (to_ai.y).abs() < 3.0 {
                    drafting.is_drafting = true;
                    break;
                }
            }
        }
    }
}

fn ai_bird_behavior(
    mut query: Query<(&mut Bird, &mut AiBird, &Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (mut bird, mut ai, transform) in query.iter_mut() {
        // Update wander timer
        ai.wander_timer -= dt;
        if ai.wander_timer <= 0.0 {
            let mut rng = rand::thread_rng();
            ai.wander_timer = rng.gen_range(2.0..5.0);
            ai.wander_direction = rng.gen_range(-0.3..0.3);
            ai.target_height = rng.gen_range(12.0..28.0);
        }

        // Gentle wandering
        bird.yaw += ai.wander_direction * dt;

        // Height adjustment
        let height_diff = ai.target_height - transform.translation.y;
        bird.pitch = (height_diff * 0.1).clamp(-0.3, 0.3);

        // Keep moving forward (towards +Z generally)
        if bird.yaw.abs() > PI / 4.0 {
            bird.yaw *= 0.95;
        }
    }
}

fn update_ui(
    energy_query: Query<(&Energy, &Drafting), With<Player>>,
    mut energy_bar_query: Query<&mut Style, With<EnergyBar>>,
    mut draft_text_query: Query<&mut Text, With<DraftIndicator>>,
    mut distance_text_query: Query<&mut Text, (With<DistanceText>, Without<DraftIndicator>)>,
    player_query: Query<&Transform, With<Player>>,
    goal_query: Query<&Transform, With<Goal>>,
) {
    let Ok((energy, drafting)) = energy_query.get_single() else { return };
    let Ok(mut bar_style) = energy_bar_query.get_single_mut() else { return };
    let Ok(mut draft_text) = draft_text_query.get_single_mut() else { return };
    let Ok(mut distance_text) = distance_text_query.get_single_mut() else { return };

    // Update energy bar
    let percent = (energy.current / energy.max * 100.0).clamp(0.0, 100.0);
    bar_style.width = Val::Percent(percent);

    // Update draft indicator
    if drafting.is_drafting {
        draft_text.sections[0].value = "DRAFTING! Energy saved!".to_string();
    } else {
        draft_text.sections[0].value = "".to_string();
    }

    // Update distance
    if let Ok(player_transform) = player_query.get_single() {
        if let Ok(goal_transform) = goal_query.get_single() {
            let distance = (goal_transform.translation - player_transform.translation).length();
            distance_text.sections[0].value = format!("Distance to home: {:.0}m", distance);
        }
    }
}

fn check_goal(
    player_query: Query<(&Transform, &Energy), With<Player>>,
    goal_query: Query<&Transform, With<Goal>>,
    mut game_state: ResMut<GameState>,
) {
    if game_state.game_over {
        return;
    }

    let Ok((player_transform, energy)) = player_query.get_single() else { return };
    let Ok(goal_transform) = goal_query.get_single() else { return };

    let distance = (goal_transform.translation - player_transform.translation).length();

    if distance < 5.0 {
        println!("You made it home! Energy remaining: {:.1}%", energy.current);
        game_state.game_over = true;
        game_state.won = true;
    }

    if energy.current <= 0.0 {
        println!("Out of energy! You fell from the sky...");
        game_state.game_over = true;
    }
}

// === BIRD SELECTION SCREEN ===

fn setup_selection_ui(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.7).into(),
                ..default()
            },
            SelectionUI,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn(TextBundle::from_section(
                "Choose Your Bird",
                TextStyle {
                    font_size: 48.0,
                    color: Color::WHITE,
                    ..default()
                },
            ).with_style(Style {
                margin: UiRect::bottom(Val::Px(40.0)),
                ..default()
            }));

            // Bird buttons container
            parent.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(20.0),
                    ..default()
                },
                ..default()
            }).with_children(|parent| {
                for bird_type in [BirdType::Sparrow, BirdType::Hawk, BirdType::Eagle, BirdType::Albatross] {
                    let stats = BirdStats::for_type(bird_type);
                    parent.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(150.0),
                                height: Val::Px(180.0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                padding: UiRect::all(Val::Px(10.0)),
                                ..default()
                            },
                            background_color: bird_type.color().into(),
                            ..default()
                        },
                        BirdButton(bird_type),
                    )).with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            bird_type.name(),
                            TextStyle {
                                font_size: 24.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        ));
                        parent.spawn(TextBundle::from_section(
                            format!("Size: {:.1}x", bird_type.scale()),
                            TextStyle {
                                font_size: 14.0,
                                color: Color::srgba(1.0, 1.0, 1.0, 0.8),
                                ..default()
                            },
                        ));
                        parent.spawn(TextBundle::from_section(
                            format!("Speed: {:.0}", stats.perfect_glide_speed),
                            TextStyle {
                                font_size: 14.0,
                                color: Color::srgba(1.0, 1.0, 1.0, 0.8),
                                ..default()
                            },
                        ));
                        parent.spawn(TextBundle::from_section(
                            format!("Agility: {:.1}", stats.turn_rate),
                            TextStyle {
                                font_size: 14.0,
                                color: Color::srgba(1.0, 1.0, 1.0, 0.8),
                                ..default()
                            },
                        ));
                    });
                }
            });

            // Instructions
            parent.spawn(TextBundle::from_section(
                "Click to select",
                TextStyle {
                    font_size: 20.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                    ..default()
                },
            ).with_style(Style {
                margin: UiRect::top(Val::Px(30.0)),
                ..default()
            }));
        });
}

fn selection_button_system(
    mut commands: Commands,
    mut interaction_query: Query<(&Interaction, &BirdButton, &mut BackgroundColor), Changed<Interaction>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, bird_button, mut bg_color) in interaction_query.iter_mut() {
        let base_color = bird_button.0.color();
        match *interaction {
            Interaction::Pressed => {
                commands.insert_resource(SelectedBirdType(bird_button.0));
                next_state.set(AppState::Playing);
            }
            Interaction::Hovered => {
                let rgba = base_color.to_srgba();
                *bg_color = Color::srgb(
                    (rgba.red * 1.3).min(1.0),
                    (rgba.green * 1.3).min(1.0),
                    (rgba.blue * 1.3).min(1.0),
                ).into();
            }
            Interaction::None => {
                *bg_color = base_color.into();
            }
        }
    }
}

fn cleanup_selection_ui(
    mut commands: Commands,
    query: Query<Entity, With<SelectionUI>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selected_bird: Res<SelectedBirdType>,
) {
    let player_bird_type = selected_bird.0;
    let player_stats = BirdStats::for_type(player_bird_type);
    let player_scale = player_bird_type.scale();

    let base_color = player_bird_type.color();
    let rgba = base_color.to_srgba();
    let head_color = Color::srgb(
        (rgba.red * 1.2).min(1.0),
        (rgba.green * 1.2).min(1.0),
        (rgba.blue * 1.2).min(1.0),
    );
    let wing_color = Color::srgb(rgba.red * 0.8, rgba.green * 0.8, rgba.blue * 0.8);

    let voxel = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    let body_material = materials.add(StandardMaterial {
        base_color,
        ..default()
    });
    let head_material = materials.add(StandardMaterial {
        base_color: head_color,
        ..default()
    });
    let wing_material = materials.add(StandardMaterial {
        base_color: wing_color,
        ..default()
    });
    let beak_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.7, 0.2),
        ..default()
    });
    let eye_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.1),
        ..default()
    });
    let eye_white = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 1.0),
        ..default()
    });

    // Spawn player voxel bird
    commands.spawn((
        PbrBundle {
            mesh: voxel.clone(),
            material: body_material,
            transform: Transform::from_xyz(0.0, 125.0, 0.0)
                .with_scale(Vec3::new(0.8 * player_scale * 3.0, 0.8 * player_scale * 3.0, 1.2 * player_scale * 3.0)),
            ..default()
        },
        Player,
        Bird {
            speed: player_stats.perfect_glide_speed,
            pitch: 0.0,
            yaw: 0.0,
            roll: 0.0,
            velocity: Vec3::new(0.0, 0.0, player_stats.perfect_glide_speed),
            grounded: false,
            damage_timer: 0.0,
            walk_timer: 0.0,
            is_walking: false,
        },
        player_stats,
        Energy {
            current: 100.0,
            max: 100.0,
            _drain_rate: 2.0,
        },
        Drafting {
            is_drafting: false,
            draft_bonus: 0.7,
        },
        BodyPart::Body,
    )).with_children(|parent| {
        // Head (with beak and eyes as children so they rotate together)
        parent.spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: head_material,
                transform: Transform::from_xyz(0.0, 0.0, 0.7)
                    .with_scale(Vec3::new(0.75, 0.75, 0.75)),
                ..default()
            },
            BodyPart::Head,
        )).with_children(|head| {
            // Beak (position relative to head center, scaled for head's 0.75 scale)
            head.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: beak_material,
                transform: Transform::from_xyz(0.0, 0.0, 0.55)
                    .with_scale(Vec3::new(0.4, 0.27, 0.55)),
                ..default()
            });
            // Eyes (positions relative to head center)
            head.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: eye_white.clone(),
                transform: Transform::from_xyz(-0.4, 0.2, 0.2)
                    .with_scale(Vec3::splat(0.27)),
                ..default()
            });
            head.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: eye_material.clone(),
                transform: Transform::from_xyz(-0.47, 0.2, 0.27)
                    .with_scale(Vec3::splat(0.13)),
                ..default()
            });
            head.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: eye_white,
                transform: Transform::from_xyz(0.4, 0.2, 0.2)
                    .with_scale(Vec3::splat(0.27)),
                ..default()
            });
            head.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: eye_material,
                transform: Transform::from_xyz(0.47, 0.2, 0.27)
                    .with_scale(Vec3::splat(0.13)),
                ..default()
            });
        });
        // Tail feathers
        for i in 0..3 {
            let spread = (i as f32 - 1.0) * 0.25;
            parent.spawn((
                PbrBundle {
                    mesh: voxel.clone(),
                    material: wing_material.clone(),
                    transform: Transform::from_xyz(spread, 0.0, -0.9 - i as f32 * 0.1)
                        .with_scale(Vec3::new(0.15, 0.05, 0.5)),
                    ..default()
                },
                BodyPart::Tail,
            ));
        }
        // Left wing
        parent.spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: wing_material.clone(),
                transform: Transform::from_xyz(-0.8, 0.0, 0.0)
                    .with_scale(Vec3::new(1.0, 0.1, 0.5)),
                ..default()
            },
            Wing { is_left: true, _base_x: -0.8 },
            BodyPart::Wing,
        ));
        // Right wing
        parent.spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: wing_material,
                transform: Transform::from_xyz(0.8, 0.0, 0.0)
                    .with_scale(Vec3::new(1.0, 0.1, 0.5)),
                ..default()
            },
            Wing { is_left: false, _base_x: 0.8 },
            BodyPart::Wing,
        ));
    });

    println!("Playing as {} bird! Scale: {:.1}x", player_bird_type.name(), player_scale * 3.0);
}

// === CHUNK MANAGEMENT SYSTEM ===

fn chunk_management(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut chunk_manager: ResMut<ChunkManager>,
    chunk_query: Query<(Entity, &Chunk)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player_transform) = player_query.get_single() else { return };
    let player_pos = player_transform.translation;
    
    let chunk_size = chunk_manager.chunk_size;
    let view_distance = chunk_manager.view_distance;
    
    // Calculate chunk coordinates for player position
    let player_chunk_x = (player_pos.x / chunk_size).floor() as i32;
    let player_chunk_z = (player_pos.z / chunk_size).floor() as i32;
    
    // Determine chunks in view distance
    let view_chunks = (view_distance / chunk_size).ceil() as i32;
    let mut chunks_to_load = HashSet::new();
    
    for dx in -view_chunks..=view_chunks {
        for dz in -view_chunks..=view_chunks {
            let chunk_x = player_chunk_x + dx;
            let chunk_z = player_chunk_z + dz;
            
            // Calculate distance from player to chunk center
            let chunk_center_x = (chunk_x as f32 + 0.5) * chunk_size;
            let chunk_center_z = (chunk_z as f32 + 0.5) * chunk_size;
            let distance = Vec2::new(
                chunk_center_x - player_pos.x,
                chunk_center_z - player_pos.z
            ).length();
            
            if distance <= view_distance {
                // Determine LOD level based on distance
                let lod_level = if distance < view_distance * 0.3 {
                    0  // High detail
                } else if distance < view_distance * 0.6 {
                    1  // Medium detail
                } else {
                    2  // Low detail
                };
                
                chunks_to_load.insert((chunk_x, chunk_z, lod_level));
            }
        }
    }
    
    // Unload chunks that are no longer needed
    let mut chunks_to_unload = Vec::new();
    for (entity, chunk) in chunk_query.iter() {
        let key = (chunk.x, chunk.z, chunk.lod_level);
        if !chunks_to_load.contains(&key) {
            chunks_to_unload.push(entity);
            chunk_manager.loaded_chunks.remove(&key);
        }
    }
    
    for entity in chunks_to_unload {
        commands.entity(entity).despawn_recursive();
    }
    
    // Load new chunks
    for &(chunk_x, chunk_z, lod_level) in chunks_to_load.iter() {
        let key = (chunk_x, chunk_z, lod_level);
        if !chunk_manager.loaded_chunks.contains(&key) {
            // Generate and spawn chunk
            spawn_chunk(
                &mut commands,
                chunk_x,
                chunk_z,
                lod_level,
                chunk_manager.chunk_size,
                &mut meshes,
                &mut materials,
            );
            chunk_manager.loaded_chunks.insert(key);
        }
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    chunk_x: i32,
    chunk_z: i32,
    lod_level: u8,
    chunk_size: f32,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let voxel_size = match lod_level {
        0 => 50.0,
        1 => 250.0,
        2 => 1500.0,
        _ => 250.0,
    };
    let vertical_thickness = 1000.0;
    let ground_depth = -100000.0;

    // Calculate chunk world position
    let chunk_world_x = chunk_x as f32 * chunk_size;
    let chunk_world_z = chunk_z as f32 * chunk_size;
    
    // Island bounds (centered at 0,0, size 2000x2000)
    let island_half_width = 1000.0;
    let island_half_length = 1000.0;
    
    // Create parent chunk entity for tracking
    println!("Spawning chunk ({}, {}) LOD {}", chunk_x, chunk_z, lod_level);
    let _chunk_entity = commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(chunk_world_x, 0.0, chunk_world_z),
            ..default()
        },
        Chunk {
            x: chunk_x,
            z: chunk_z,
            lod_level,
        },
        VoxelChunk,
    )).id();
    
    // No ground voxels - floating island only
}





// === NETWORKING SYSTEMS ===

fn network_connect(network: ResMut<NetworkState>) {
    // Send join message
    let join_msg = ClientMessage::Join {
        name: "Player".to_string(),
    };
    network.send(&join_msg);
    println!("Sent join request to server");
}

fn network_send_state(
    mut network: ResMut<NetworkState>,
    player_query: Query<(&Transform, &Bird, &BirdStats), With<Player>>,
    flap_state: Res<FlapState>,
) {
    // Only send if connected
    if network.player_id.is_none() {
        return;
    }

    // Rate limit: send at ~20 Hz
    if network.last_send.elapsed().as_millis() < 50 {
        return;
    }
    network.last_send = Instant::now();

    let Ok((transform, bird, stats)) = player_query.get_single() else {
        return;
    };

    let net_bird_type = match stats.bird_type {
        BirdType::Sparrow => NetBirdType::Sparrow,
        BirdType::Hawk => NetBirdType::Hawk,
        BirdType::Eagle => NetBirdType::Eagle,
        BirdType::Albatross => NetBirdType::Albatross,
    };

    let state = PlayerState {
        player_id: network.player_id.unwrap_or(0),
        position: [
            transform.translation.x,
            transform.translation.y,
            transform.translation.z,
        ],
        rotation: [
            transform.rotation.x,
            transform.rotation.y,
            transform.rotation.z,
            transform.rotation.w,
        ],
        velocity: [bird.velocity.x, bird.velocity.y, bird.velocity.z],
        bird_type: net_bird_type,
        wings_closed: flap_state.space_held,
        grounded: bird.grounded,
        is_walking: bird.is_walking,
        flap_timer: flap_state.timer,
    };

    network.send(&ClientMessage::StateUpdate(state));
}

fn network_receive(
    mut commands: Commands,
    mut network: ResMut<NetworkState>,
    mut remote_players: Query<(Entity, &mut RemotePlayer, &mut Transform, &Children)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut buf = [0u8; 1024];

    // Process all pending messages
    loop {
        match network.socket.recv_from(&mut buf) {
            Ok((len, _)) => {
                let data = &buf[..len];
                match flight_shared::decode::<ServerMessage>(data) {
                    Ok(ServerMessage::Welcome { player_id }) => {
                        println!("Connected! Player ID: {}", player_id);
                        network.player_id = Some(player_id);
                        network.connected = true;
                    }
                    Ok(ServerMessage::PlayerState(state)) => {
                        // Don't process our own state
                        if Some(state.player_id) == network.player_id {
                            continue;
                        }

                        // Update existing or spawn new
                        let mut found = false;
                        let mut needs_respawn = None;
                        for (entity, mut remote, mut transform, _) in remote_players.iter_mut() {
                            if remote.player_id == state.player_id {
                                // Check if bird type changed
                                if remote.bird_type != state.bird_type {
                                    // Need to respawn with new model
                                    needs_respawn = Some((entity, state.clone()));
                                    found = true;
                                    break;
        }

                                // Update position and rotation
                                transform.translation = Vec3::new(
                                    state.position[0],
                                    state.position[1],
                                    state.position[2],
                                );
                                transform.rotation = Quat::from_xyzw(
                                    state.rotation[0],
                                    state.rotation[1],
                                    state.rotation[2],
                                    state.rotation[3],
                                );
                                // Update wing state
                                remote.wings_closed = state.wings_closed;
                                remote.flap_timer = state.flap_timer;
                                remote.grounded = state.grounded;
                                remote.is_walking = state.is_walking;
                                remote.last_update = Instant::now();
                                found = true;
                                break;
                            }
                        }

                        // Handle respawn if bird type changed
                        if let Some((entity, new_state)) = needs_respawn {
                            commands.entity(entity).despawn_recursive();
                            spawn_remote_player(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                new_state,
                            );
                        } else if !found {
                            // Spawn new remote player
                            spawn_remote_player(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                state,
                            );
                        }
                    }
                    Ok(ServerMessage::PlayerLeft { player_id }) => {
                        println!("Player {} left", player_id);
                        for (entity, remote, _, _) in remote_players.iter() {
                            if remote.player_id == player_id {
                                commands.entity(entity).despawn_recursive();
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to decode server message: {}", e);
                    }
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                // No more messages
                break;
            }
            Err(e) => {
                eprintln!("Network error: {}", e);
                break;
            }
        }
    }
}

fn spawn_remote_player(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    state: PlayerState,
) {
    println!("Spawning remote player {}", state.player_id);

    let bird_type = match state.bird_type {
        NetBirdType::Sparrow => BirdType::Sparrow,
        NetBirdType::Hawk => BirdType::Hawk,
        NetBirdType::Eagle => BirdType::Eagle,
        NetBirdType::Albatross => BirdType::Albatross,
    };

    let scale = bird_type.scale();
    let color = bird_type.color();
    let rgba = color.to_srgba();

    let voxel = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Body color (main)
    let body_material = materials.add(StandardMaterial {
        base_color: color,
        ..default()
    });
    // Lighter belly/head color
    let body_light = materials.add(StandardMaterial {
        base_color: Color::srgb(
            (rgba.red * 1.2).min(1.0),
            (rgba.green * 1.2).min(1.0),
            (rgba.blue * 1.2).min(1.0),
        ),
        ..default()
    });
    // Darker wing color
    let wing_material = materials.add(StandardMaterial {
        base_color: Color::srgb(rgba.red * 0.8, rgba.green * 0.8, rgba.blue * 0.8),
        ..default()
    });
    // Beak
    let beak_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.7, 0.2),
        ..default()
    });
    // Eyes
    let eye_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.1),
        ..default()
    });
    let eye_white = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 1.0),
        ..default()
    });

    commands
        .spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: body_material,
                transform: Transform::from_xyz(
                    state.position[0],
                    state.position[1],
                    state.position[2],
                )
                .with_rotation(Quat::from_xyzw(
                    state.rotation[0],
                    state.rotation[1],
                    state.rotation[2],
                    state.rotation[3],
                ))
                .with_scale(Vec3::new(0.8 * scale, 1.2 * scale, 0.8 * scale)),
                ..default()
            },
            RemotePlayer {
                player_id: state.player_id,
                last_update: Instant::now(),
                bird_type: state.bird_type,
                wings_closed: state.wings_closed,
                flap_timer: state.flap_timer,
                grounded: state.grounded,
                is_walking: state.is_walking,
            },
        ))
        .with_children(|parent| {
            // Head
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: body_light.clone(),
                transform: Transform::from_xyz(0.0, 0.0, 0.7)
                    .with_scale(Vec3::new(0.75, 0.75, 0.75)),
                ..default()
            });
            // Beak
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
            material: beak_material.clone(),
                transform: Transform::from_xyz(0.0, 0.0, 1.1)
                    .with_scale(Vec3::new(0.3, 0.2, 0.4)),
                ..default()
            });
            // Eyes
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: eye_white.clone(),
                transform: Transform::from_xyz(-0.3, 0.15, 0.85)
                    .with_scale(Vec3::splat(0.2)),
                ..default()
            });
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: eye_material.clone(),
                transform: Transform::from_xyz(-0.35, 0.15, 0.9)
                    .with_scale(Vec3::splat(0.1)),
                ..default()
            });
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: eye_white,
                transform: Transform::from_xyz(0.3, 0.15, 0.85)
                    .with_scale(Vec3::splat(0.2)),
                ..default()
            });
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: eye_material,
                transform: Transform::from_xyz(0.35, 0.15, 0.9)
                    .with_scale(Vec3::splat(0.1)),
                ..default()
            });
            // Tail feathers
            for i in 0..3 {
                let spread = (i as f32 - 1.0) * 0.25;
                parent.spawn(PbrBundle {
                    mesh: voxel.clone(),
                    material: wing_material.clone(),
                    transform: Transform::from_xyz(spread, 0.0, -0.9 - i as f32 * 0.1)
                        .with_scale(Vec3::new(0.15, 0.05, 0.5)),
                    ..default()
                });
            }
            // Left wing
            parent.spawn((
                PbrBundle {
                    mesh: voxel.clone(),
                    material: wing_material.clone(),
                    transform: Transform::from_xyz(-0.8, 0.0, 0.0)
                        .with_scale(Vec3::new(1.0, 0.1, 0.5)),
                    ..default()
                },
                Wing {
                    is_left: true,
                    _base_x: -0.8,
                },
            ));
            // Right wing
            parent.spawn((
                PbrBundle {
                    mesh: voxel.clone(),
                    material: wing_material,
                    transform: Transform::from_xyz(0.8, 0.0, 0.0)
                        .with_scale(Vec3::new(1.0, 0.1, 0.5)),
                    ..default()
                },
                Wing {
                    is_left: false,
                    _base_x: 0.8,
                },
            ));
        });
}

fn cleanup_stale_players(
    mut commands: Commands,
    remote_players: Query<(Entity, &RemotePlayer)>,
) {
    for (entity, remote) in remote_players.iter() {
        // Remove players that haven't been updated in 5 seconds
        if remote.last_update.elapsed().as_secs() > 5 {
            println!("Removing stale player {}", remote.player_id);
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn remote_player_wing_animation(
    remote_players: Query<(&RemotePlayer, &Children)>,
    mut wing_query: Query<(&Wing, &mut Transform)>,
) {
    let flap_duration = 0.3;
    let wing_closed_angle = 1.2;

    for (remote, children) in remote_players.iter() {
        for &child in children.iter() {
            if let Ok((wing, mut transform)) = wing_query.get_mut(child) {
                // Base rotation: wings rotated 90° so they're flat/horizontal
                let base_rotation = Quat::from_rotation_x(PI / 2.0);

                if remote.grounded {
                    // Wings folded at sides when grounded
                    let fold_angle = 1.2;
                    if wing.is_left {
                        transform.rotation = base_rotation * Quat::from_rotation_z(fold_angle);
                    } else {
                        transform.rotation = base_rotation * Quat::from_rotation_z(-fold_angle);
                    }
                } else if remote.flap_timer > 0.0 {
                    // Flap animation
                    let progress = 1.0 - (remote.flap_timer / flap_duration);
                    let angle = (progress * PI).sin() * 0.8;

                    if wing.is_left {
                        transform.rotation = base_rotation * Quat::from_rotation_z(angle);
                    } else {
                        transform.rotation = base_rotation * Quat::from_rotation_z(-angle);
                    }
                } else if remote.wings_closed {
                    // Wings closed (folded up)
                    if wing.is_left {
                        transform.rotation = base_rotation * Quat::from_rotation_z(wing_closed_angle);
                    } else {
                        transform.rotation = base_rotation * Quat::from_rotation_z(-wing_closed_angle);
                    }
                } else {
                    // Wings open (gliding)
                    transform.rotation = base_rotation;
                }
            }
        }
    }
}
