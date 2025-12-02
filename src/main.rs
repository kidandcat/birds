use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use rand::Rng;
use std::f32::consts::PI;

#[derive(Resource, Default)]
struct GameState {
    game_over: bool,
    won: bool,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::srgb(0.5, 0.7, 0.9)))
        .init_resource::<GameState>()
        .init_resource::<FlapState>()
        .add_systems(Startup, (setup, grab_cursor))
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
            bird_selection,
        ))
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
                min_glide_speed: 8.0,
                perfect_glide_speed: 25.0,  // Higher cruise speed
                turn_rate: 1.5,             // Very agile
                roll_rate: 4.0,
                flap_thrust: 9.0,           // 1.5x power
                flap_energy_cost: 5.0,      // Same cost for all birds
                flap_cooldown: 0.15,        // Fast flapping
                max_dive_speed: 80.0,
                acceleration: 20.0,
                walk_speed: 6.0,            // Quick walker
            },
            BirdType::Hawk => BirdStats {
                bird_type,
                glide_efficiency: 1.0,      // Balanced
                min_glide_speed: 10.0,
                perfect_glide_speed: 20.0,
                turn_rate: 1.0,
                roll_rate: 3.0,
                flap_thrust: 8.0,
                flap_energy_cost: 5.0,      // Same cost for all birds
                flap_cooldown: 0.25,
                max_dive_speed: 95.0,
                acceleration: 25.0,
                walk_speed: 5.0,
            },
            BirdType::Eagle => BirdStats {
                bird_type,
                glide_efficiency: 0.6,      // Good glide
                min_glide_speed: 12.0,
                perfect_glide_speed: 25.0,
                turn_rate: 0.7,             // Slower turning
                roll_rate: 2.0,
                flap_thrust: 12.0,          // Powerful flaps
                flap_energy_cost: 5.0,      // Same cost for all birds
                flap_cooldown: 0.4,         // Slow flapping
                max_dive_speed: 110.0,
                acceleration: 30.0,
                walk_speed: 4.0,            // Slow walker
            },
            BirdType::Albatross => BirdStats {
                bird_type,
                glide_efficiency: 0.3,      // Excellent glide
                min_glide_speed: 15.0,
                perfect_glide_speed: 30.0,
                turn_rate: 0.5,             // Very slow turning
                roll_rate: 1.5,
                flap_thrust: 15.0,          // Very powerful
                flap_energy_cost: 5.0,      // Same cost for all birds
                flap_cooldown: 0.6,         // Very slow flapping
                max_dive_speed: 130.0,
                acceleration: 35.0,
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
    drain_rate: f32,
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
struct BirdTypeText;

#[derive(Component)]
struct Wing {
    is_left: bool,
    base_x: f32,
}

#[derive(Resource, Default)]
struct FlapState {
    timer: f32,
    cooldown: f32,  // Time until next flap allowed
    space_held: bool,
    wings_closed_time: f32,  // How long wings have been closed
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();

    // === ENVIRONMENT ===

    // Main grass ground
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(2000.0, 2000.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.28, 0.52, 0.25),
            perceptual_roughness: 0.9,
            ..default()
        }),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });

    // Water/river along the path
    let water_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.5, 0.7, 0.8),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.1,
        metallic: 0.3,
        ..default()
    });
    for i in 0..10 {
        let z = i as f32 * 60.0 - 50.0;
        let x_offset = (i as f32 * 0.7).sin() * 30.0;
        commands.spawn(PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(40.0, 70.0)),
            material: water_material.clone(),
            transform: Transform::from_xyz(x_offset - 60.0, 0.1, z),
            ..default()
        });
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
        let x: f32 = rng.gen_range(-150.0..150.0);
        let z: f32 = rng.gen_range(-100.0..600.0);
        // Avoid placing trees in the center flight path
        if x.abs() < 25.0 { continue; }

        let trunk_height = rng.gen_range(8.0..20.0);  // Much taller trunks
        let tree_top = trunk_height + 12.0;  // Top of leaves
        let tree_center_y = tree_top / 2.0;
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
                transform: Transform::from_xyz(x, trunk_height / 2.0, z)
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
            let leaf_y = trunk_height + 2.0 + layer as f32 * 3.0;
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

    for i in 0..15 {
        let x = -300.0 + i as f32 * 40.0 + rng.gen_range(-10.0..10.0);
        let height = rng.gen_range(60.0..120.0);
        let width = rng.gen_range(30.0..60.0);
        // Mountain body
        commands.spawn((
            PbrBundle {
                mesh: mountain_mesh.clone(),
                material: mountain_material.clone(),
                transform: Transform::from_xyz(x, height / 2.0, 700.0)
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
            transform: Transform::from_xyz(x, height * 0.85, 700.0)
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
        let x = rng.gen_range(-200.0..200.0);
        let y = rng.gen_range(40.0..80.0);
        let z = rng.gen_range(-50.0..650.0);
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
        let x = rng.gen_range(-200.0..200.0);
        let z = rng.gen_range(-100.0..600.0);
        let size = rng.gen_range(20.0..50.0);
        let height = rng.gen_range(3.0..10.0);
        commands.spawn(PbrBundle {
            mesh: meshes.add(Sphere::new(1.0)),
            material: hill_material.clone(),
            transform: Transform::from_xyz(x, -size * 0.3 + height, z)
                .with_scale(Vec3::new(size, height, size)),
            ..default()
        });
    }

    // === VOXEL BIRD MESHES ===
    let voxel = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Player bird colors
    let body_orange = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.6, 0.2),
        ..default()
    });
    let body_yellow = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.85, 0.3),
        ..default()
    });
    let wing_orange = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.5, 0.15),
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

    // Default bird type for player (can be changed via UI)
    let player_bird_type = BirdType::Hawk;  // Medium balanced bird
    let player_stats = BirdStats::for_type(player_bird_type);
    let player_scale = player_bird_type.scale();

    // Spawn player voxel bird
    commands.spawn((
        PbrBundle {
            mesh: voxel.clone(),
            material: body_orange.clone(),
            transform: Transform::from_xyz(0.0, 20.0, 0.0)
                .with_rotation(Quat::from_rotation_x(-PI / 2.0))
                .with_scale(Vec3::new(0.8 * player_scale, 1.2 * player_scale, 0.8 * player_scale)),
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
            drain_rate: 2.0,
        },
        Drafting {
            is_drafting: false,
            draft_bonus: 0.7,
        },
    )).with_children(|parent| {
        // Head
        parent.spawn(PbrBundle {
            mesh: voxel.clone(),
            material: body_yellow.clone(),
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
            material: eye_white.clone(),
            transform: Transform::from_xyz(0.3, 0.15, 0.85)
                .with_scale(Vec3::splat(0.2)),
            ..default()
        });
        parent.spawn(PbrBundle {
            mesh: voxel.clone(),
            material: eye_material.clone(),
            transform: Transform::from_xyz(0.35, 0.15, 0.9)
                .with_scale(Vec3::splat(0.1)),
            ..default()
        });
        // Tail feathers
        for i in 0..3 {
            let spread = (i as f32 - 1.0) * 0.25;
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: wing_orange.clone(),
                transform: Transform::from_xyz(spread, 0.0, -0.9 - i as f32 * 0.1)
                    .with_scale(Vec3::new(0.15, 0.05, 0.5)),
                ..default()
            });
        }
        // Left wing (multiple feathers)
        parent.spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: wing_orange.clone(),
                transform: Transform::from_xyz(-0.8, 0.0, 0.0)
                    .with_scale(Vec3::new(1.0, 0.1, 0.5)),
                ..default()
            },
            Wing { is_left: true, base_x: -0.8 },
        ));
        // Right wing
        parent.spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: wing_orange.clone(),
                transform: Transform::from_xyz(0.8, 0.0, 0.0)
                    .with_scale(Vec3::new(1.0, 0.1, 0.5)),
                ..default()
            },
            Wing { is_left: false, base_x: 0.8 },
        ));
    });

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
        let x = rng.gen_range(-20.0..20.0);
        let z = rng.gen_range(50.0..150.0);
        let height = rng.gen_range(15.0..25.0);

        commands.spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: ai_body.clone(),
                transform: Transform::from_xyz(x, height, z)
                    .with_rotation(Quat::from_rotation_x(-PI / 2.0))
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
                Wing { is_left: true, base_x: -0.6 },
            ));
            parent.spawn((
                PbrBundle {
                    mesh: voxel.clone(),
                    material: ai_wing.clone(),
                    transform: Transform::from_xyz(0.6, 0.0, 0.0)
                        .with_scale(Vec3::new(0.8, 0.08, 0.4)),
                    ..default()
                },
                Wing { is_left: false, base_x: 0.6 },
            ));
        });
    }

    // Goal - Nest in a tree
    let nest_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.35, 0.2),
        ..default()
    });
    // Tree for nest
    commands.spawn(PbrBundle {
        mesh: trunk_mesh.clone(),
        material: trunk_material.clone(),
        transform: Transform::from_xyz(0.0, 6.0, 500.0)
            .with_scale(Vec3::new(2.0, 3.0, 2.0)),
        ..default()
    });
    commands.spawn(PbrBundle {
        mesh: leaves_mesh.clone(),
        material: leaves_material.clone(),
        transform: Transform::from_xyz(0.0, 14.0, 500.0)
            .with_scale(Vec3::new(3.0, 2.0, 3.0)),
        ..default()
    });
    // Nest
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Torus::new(1.5, 2.5)),
            material: nest_material,
            transform: Transform::from_xyz(0.0, 12.0, 500.0)
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
        transform: Transform::from_xyz(0.0, 13.0, 500.0),
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

    // Camera with fog effect
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 25.0, -10.0)
                .looking_at(Vec3::new(0.0, 20.0, 10.0), Vec3::Y),
            ..default()
        },
        FogSettings {
            color: Color::srgb(0.7, 0.8, 0.95),
            falloff: FogFalloff::Linear {
                start: 100.0,
                end: 500.0,
            },
            ..default()
        },
    ));

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

        // Bird type selector
        parent.spawn((
            TextBundle::from_section(
                "Bird: Hawk [1-4 to change]",
                TextStyle {
                    font_size: 18.0,
                    color: Color::srgb(0.9, 0.8, 0.5),
                    ..default()
                },
            ).with_style(Style {
                margin: UiRect::top(Val::Px(10.0)),
                ..default()
            }),
            BirdTypeText,
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
        bird.pitch = (bird.pitch + event.delta.y * mouse_sensitivity).clamp(-0.8, 0.8);
    }

    // Auto-roll when turning: bank into the turn
    let target_roll = yaw_delta * 30.0;  // Roll in direction of turn (reduced amount)
    bird.roll = bird.roll + (target_roll - bird.roll) * stats.roll_rate * dt;

    // Gradually return roll to level
    bird.roll *= 0.95;

    // Auto-return to optimal glide angle when not moving mouse
    // Optimal angle is slightly nose-down (-0.05) for efficient gliding
    let optimal_pitch = -0.05;
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

                // Walking pose: slight forward lean with body bob via rotation
                let bob_angle = (bird.walk_timer).sin() * 0.1;  // Body tilts up/down
                let walk_rotation = Quat::from_rotation_y(bird.yaw)
                    * Quat::from_rotation_x(-PI / 2.0 + 0.15 + bob_angle)  // Forward lean + bob
                    * Quat::from_rotation_z((bird.walk_timer).sin() * 0.08);  // Side sway
                transform.rotation = transform.rotation.slerp(walk_rotation, 10.0 * dt);
            } else {
                // Reset walk timer when stopped
                bird.walk_timer = 0.0;

                // Idle standing pose
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
                BirdType::Sparrow => 0.015,
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

        // === GROUND COLLISION ===
        // Bird center should be at ~0.5 above ground (half bird height)
        if transform.translation.y < 0.5 {
            transform.translation.y = 0.5;
            // Land and stop
            bird.velocity = Vec3::ZERO;
            bird.grounded = true;
            bird.pitch = 0.0;
            bird.roll = 0.0;
        }

        // === ROTATE BIRD ===
        if bird.grounded {
            // Idle pose: standing upright (undo the flight pitch rotation)
            let idle_rotation = Quat::from_rotation_y(bird.yaw)
                * Quat::from_rotation_x(-PI / 2.0);  // Same as spawn rotation = standing
            transform.rotation = transform.rotation.slerp(idle_rotation, 5.0 * dt);
        } else {
            // Flying pose
            let mut target_rotation = Quat::from_rotation_y(bird.yaw)
                * Quat::from_rotation_x(-PI / 2.0 + bird.pitch)
                * Quat::from_rotation_y(bird.roll);

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
    mut bird_query: Query<(&mut Bird, &mut Transform), With<Player>>,
    obstacle_query: Query<(&Transform, &Obstacle), Without<Player>>,
    time: Res<Time>,
) {
    let Ok((mut bird, mut bird_transform)) = bird_query.get_single_mut() else { return };
    let dt = time.delta_seconds();

    // Update damage timer
    if bird.damage_timer > 0.0 {
        bird.damage_timer -= dt;
    }

    // Skip collision if grounded
    if bird.grounded { return; }

    let bird_pos = bird_transform.translation;
    let bird_radius = 0.8;

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

        let near_top = bird_pos.y > obstacle_top - 2.0 && bird_pos.y < obstacle_top + 3.0;
        let moving_down = bird.velocity.y < 0.0;  // Must be actually falling
        let is_slow = bird.velocity.length() < 5.0;  // Very slow (nearly stopped)

        if in_xz_bounds && near_top && (moving_down || is_slow) {
            // Land on top
            if bird_transform.translation.y < obstacle_top + 0.5 {
                bird_transform.translation.y = obstacle_top + 0.5;
            }
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
            * Quat::from_rotation_x(-PI / 2.0 + bird.pitch)
            * Quat::from_rotation_y(bird.roll);
        transform.rotation = transform.rotation.slerp(target_rotation, 5.0 * dt);
    }
}

fn wing_flap(
    mut wing_query: Query<(&Wing, &mut Transform)>,
    mut player_query: Query<(&mut Energy, &mut Bird, &BirdStats), With<Player>>,
    mut flap_state: ResMut<FlapState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let flap_duration = 0.3;
    let wing_closed_angle = 1.2;  // Wings folded up when closed
    let dt = time.delta_seconds();

    // Get bird state for animation
    let (is_grounded, is_walking, walk_timer) = player_query
        .get_single()
        .map(|(_, b, _)| (b.grounded, b.is_walking, b.walk_timer))
        .unwrap_or((false, false, 0.0));

    // Update cooldown timer
    if flap_state.cooldown > 0.0 {
        flap_state.cooldown -= dt;
    }

    // Track space held state - holding space always closes wings
    // Cooldown only affects whether a flap happens, not wing position
    if keyboard.pressed(KeyCode::Space) {
        flap_state.space_held = true;
        flap_state.wings_closed_time += dt;
    } else {
        flap_state.space_held = false;
        flap_state.wings_closed_time = 0.0;  // Reset gravity ramp
    }

    // Cancel flap animation if pressing space during cooldown
    if keyboard.just_pressed(KeyCode::Space) && flap_state.cooldown > 0.0 {
        flap_state.timer = 0.0;
    }

    // Start new flap on space press (with cooldown check)
    if keyboard.just_pressed(KeyCode::Space) && flap_state.cooldown <= 0.0 {
        // Deduct energy and add thrust relative to bird rotation
        if let Ok((mut energy, mut bird, bird_stats)) = player_query.get_single_mut() {
            // Only start animation and cooldown if we actually flap
            flap_state.timer = flap_duration;
            flap_state.cooldown = bird_stats.flap_cooldown;  // Set cooldown based on bird type

            energy.current = (energy.current - bird_stats.flap_energy_cost).max(0.0);

            // Take off if grounded
            if bird.grounded {
                bird.grounded = false;
                bird.velocity = Vec3::new(0.0, 10.0, bird_stats.perfect_glide_speed);
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

    for (wing, mut transform) in wing_query.iter_mut() {
        // Base rotation: wings rotated 90° so they're flat/horizontal
        let base_rotation = Quat::from_rotation_x(PI / 2.0);

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

fn camera_follow(
    player_query: Query<(&Transform, &Bird), With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
    time: Res<Time>,
) {
    let Ok((player_transform, bird)) = player_query.get_single() else { return };
    let Ok(mut camera_transform) = camera_query.get_single_mut() else { return };
    let dt = time.delta_seconds();

    // When grounded, don't apply pitch or roll to camera
    let (pitch_factor, roll_factor) = if bird.grounded {
        (0.0, 0.0)
    } else {
        (bird.pitch * 0.5, bird.roll * 0.7)
    };

    // 3rd person FPV style: camera follows bird rotation but looks at bird
    let bird_rotation = Quat::from_rotation_y(bird.yaw)
        * Quat::from_rotation_x(pitch_factor)  // Reduce pitch effect on camera
        * Quat::from_rotation_z(roll_factor);  // Reduce roll effect on camera

    // Offset behind and above the bird in its local space
    let local_offset = Vec3::new(0.0, 3.0, -12.0);
    let world_offset = bird_rotation * local_offset;

    // Smooth camera position
    let target_pos = player_transform.translation + world_offset;
    let smooth_factor = (5.0 * dt).min(1.0);  // Clamped for stability
    camera_transform.translation = camera_transform.translation.lerp(target_pos, smooth_factor);

    // Smooth camera rotation using slerp instead of look_at
    let up = Quat::from_rotation_z(-roll_factor) * Vec3::Y;
    let rotated_up = Quat::from_rotation_y(bird.yaw) * up;
    let look_dir = (player_transform.translation - camera_transform.translation).normalize_or_zero();
    if look_dir.length_squared() > 0.01 {
        let target_rotation = Transform::from_translation(camera_transform.translation)
            .looking_at(player_transform.translation, rotated_up)
            .rotation;
        camera_transform.rotation = camera_transform.rotation.slerp(target_rotation, smooth_factor);
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
    let draft_bonus = if drafting.is_drafting { 2.0 } else { 0.0 };  // Bonus when drafting

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

fn bird_selection(
    mut player_query: Query<(&mut BirdStats, &mut Transform, &mut Bird), With<Player>>,
    mut bird_text_query: Query<&mut Text, With<BirdTypeText>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Check for number keys 1-4 to change bird type
    let new_type = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(BirdType::Sparrow)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(BirdType::Hawk)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(BirdType::Eagle)
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        Some(BirdType::Albatross)
    } else {
        None
    };

    if let Some(bird_type) = new_type {
        if let Ok((mut stats, mut transform, mut bird)) = player_query.get_single_mut() {
            // Update stats
            *stats = BirdStats::for_type(bird_type);

            // Update scale based on new bird type
            let scale = bird_type.scale();
            transform.scale = Vec3::new(0.8 * scale, 1.2 * scale, 0.8 * scale);

            // Reset velocity to new bird's glide speed
            let yaw_rotation = Quat::from_rotation_y(bird.yaw);
            bird.velocity = yaw_rotation * Vec3::new(0.0, 0.0, stats.perfect_glide_speed);
            bird.speed = stats.perfect_glide_speed;

            println!("Changed to {} bird!", bird_type.name());
        }
    }

    // Update UI text to show current bird type
    if let Ok((stats, _, _)) = player_query.get_single() {
        if let Ok(mut text) = bird_text_query.get_single_mut() {
            text.sections[0].value = format!(
                "Bird: {} [1-4 to change]",
                stats.bird_type.name()
            );
        }
    }
}
