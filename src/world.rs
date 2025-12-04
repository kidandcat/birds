use bevy::prelude::*;
use rand::Rng;
use std::collections::HashSet;
use std::f32::consts::PI;

use crate::components::{
    AiBird, Bird, Chunk, ChunkManager, DistanceText, DraftIndicator, EnergyBar, Goal, Obstacle,
    Player, VoxelChunk, Wing,
};

/// Setup game environment
pub fn setup_environment(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();

    // Island material
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

    // Island dimensions
    let island_width = 2000.0;
    let island_length = 2000.0;
    let island_height = 20.0;
    let island_y_position = 100.0;
    let island_surface_y = island_y_position + island_height;

    // Island top
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(island_width, island_height, island_length)),
            material: island_material,
            transform: Transform::from_xyz(0.0, island_y_position + island_height / 2.0, 0.0),
            ..default()
        },
        Obstacle {
            half_extents: Vec3::new(island_width / 2.0, island_height / 2.0, island_length / 2.0),
        },
    ));

    // Island bottom
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(island_width, 5.0, island_length)),
            material: island_bottom_material,
            transform: Transform::from_xyz(
                0.0,
                island_y_position - island_height / 2.0 - 2.5,
                0.0,
            ),
            ..default()
        },
        Obstacle {
            half_extents: Vec3::new(island_width / 2.0, 2.5, island_length / 2.0),
        },
    ));

    // River
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

    let river_voxel = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let river_width = 15.0;
    let river_length = 800.0;
    let river_segments = 80;

    for i in 0..river_segments {
        let t = i as f32 / river_segments as f32;
        let z = t * river_length - 100.0;
        let x = (t * 4.0).sin() * 40.0 - 30.0;

        // River bed
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

        // Water surface
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

        // River banks
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

    // Trees
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

    for _ in 0..80 {
        let x: f32 = rng.gen_range(-800.0..800.0);
        let z: f32 = rng.gen_range(-800.0..800.0);
        if x.abs() < 25.0 {
            continue;
        }

        let trunk_height = rng.gen_range(8.0..20.0);
        let tree_width = trunk_height * 0.4;

        let trunk_width = tree_width * 0.3;
        let trunk_visual_scale = Vec3::new(trunk_width, trunk_height / 4.0, trunk_width);
        let trunk_half_extents = Vec3::new(trunk_width / 2.0, trunk_height / 2.0, trunk_width / 2.0);
        commands.spawn((
            PbrBundle {
                mesh: trunk_mesh.clone(),
                material: trunk_material.clone(),
                transform: Transform::from_xyz(x, island_surface_y + trunk_height / 2.0, z)
                    .with_scale(trunk_visual_scale),
                ..default()
            },
            Obstacle {
                half_extents: trunk_half_extents,
            },
        ));

        let leaf_mat = if rng.gen_bool(0.5) {
            leaves_material.clone()
        } else {
            dark_leaves.clone()
        };
        for layer in 0..4 {
            let size = (tree_width * 2.0) - layer as f32 * 1.5;
            let leaf_scale = Vec3::splat(size / 3.0);
            let leaf_half_extents = Vec3::splat(size / 2.0);
            let leaf_y = island_surface_y + trunk_height + 2.0 + layer as f32 * 3.0;
            commands.spawn((
                PbrBundle {
                    mesh: leaves_mesh.clone(),
                    material: leaf_mat.clone(),
                    transform: Transform::from_xyz(x, leaf_y, z).with_scale(leaf_scale),
                    ..default()
                },
                Obstacle {
                    half_extents: leaf_half_extents,
                },
            ));
        }
    }

    // Mountains
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

    // Hills
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

    // AI birds
    let voxel = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
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

        commands
            .spawn((
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
            ))
            .with_children(|parent| {
                parent.spawn(PbrBundle {
                    mesh: voxel.clone(),
                    material: ai_body.clone(),
                    transform: Transform::from_xyz(0.0, 0.0, 0.5).with_scale(Vec3::splat(0.5)),
                    ..default()
                });
                parent.spawn(PbrBundle {
                    mesh: voxel.clone(),
                    material: ai_belly.clone(),
                    transform: Transform::from_xyz(0.0, -0.2, 0.0)
                        .with_scale(Vec3::new(0.5, 0.3, 0.8)),
                    ..default()
                });
                parent.spawn((
                    PbrBundle {
                        mesh: voxel.clone(),
                        material: ai_wing.clone(),
                        transform: Transform::from_xyz(-0.6, 0.0, 0.0)
                            .with_scale(Vec3::new(0.8, 0.08, 0.4)),
                        ..default()
                    },
                    Wing {
                        is_left: true,
                        _base_x: -0.6,
                    },
                ));
                parent.spawn((
                    PbrBundle {
                        mesh: voxel.clone(),
                        material: ai_wing.clone(),
                        transform: Transform::from_xyz(0.6, 0.0, 0.0)
                            .with_scale(Vec3::new(0.8, 0.08, 0.4)),
                        ..default()
                    },
                    Wing {
                        is_left: false,
                        _base_x: 0.6,
                    },
                ));
            });
    }

    // Goal - Nest
    let nest_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.35, 0.2),
        ..default()
    });
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
    let trunk_height_val = 4.0 * trunk_scale.y;
    let trunk_bottom_y = island_surface_y;
    let trunk_center_y = trunk_bottom_y + trunk_height_val / 2.0;

    let leaves_scale = Vec3::new(3.0, 2.0, 3.0);
    let leaves_height = 3.0 * leaves_scale.y;
    let leaves_center_y = trunk_center_y + trunk_height_val / 2.0 + leaves_height / 2.0 - 2.0;

    let nest_y = trunk_center_y + trunk_height_val / 2.0 - 1.0;
    let glow_y = nest_y + 1.0;

    commands.spawn((
        PbrBundle {
            mesh: trunk_mesh.clone(),
            material: trunk_material.clone(),
            transform: Transform::from_xyz(goal_x, trunk_center_y, goal_z).with_scale(trunk_scale),
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
            transform: Transform::from_xyz(goal_x, leaves_center_y, goal_z).with_scale(leaves_scale),
            ..default()
        },
        Obstacle {
            half_extents: Vec3::new(
                leaves_scale.x * 1.5,
                leaves_scale.y * 1.5,
                leaves_scale.z * 1.5,
            ),
        },
    ));
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

    // Lighting
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

    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.6, 0.7, 0.9),
        brightness: 200.0,
    });

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 40.0, -50.0)
            .looking_at(Vec3::new(0.0, 30.0, 20.0), Vec3::Y),
        projection: PerspectiveProjection {
            near: 1.0,
            far: 50000.0,
            ..default()
        }
        .into(),
        ..default()
    });

    // UI
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(300.0),
                        height: Val::Px(30.0),
                        ..default()
                    },
                    background_color: Color::srgba(0.2, 0.2, 0.2, 0.8).into(),
                    ..default()
                })
                .with_children(|parent| {
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

            parent.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size: 24.0,
                        color: Color::srgb(0.2, 0.8, 1.0),
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                }),
                DraftIndicator,
            ));

            parent.spawn((
                TextBundle::from_section(
                    "Distance: 500m",
                    TextStyle {
                        font_size: 20.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                }),
                DistanceText,
            ));
        });
}

/// Chunk management system
pub fn chunk_management(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut chunk_manager: ResMut<ChunkManager>,
    chunk_query: Query<(Entity, &Chunk)>,
) {
    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let player_pos = player_transform.translation;

    let chunk_size = chunk_manager.chunk_size;
    let view_distance = chunk_manager.view_distance;

    let player_chunk_x = (player_pos.x / chunk_size).floor() as i32;
    let player_chunk_z = (player_pos.z / chunk_size).floor() as i32;

    let view_chunks = (view_distance / chunk_size).ceil() as i32;
    let mut chunks_to_load = HashSet::new();

    for dx in -view_chunks..=view_chunks {
        for dz in -view_chunks..=view_chunks {
            let chunk_x = player_chunk_x + dx;
            let chunk_z = player_chunk_z + dz;

            let chunk_center_x = (chunk_x as f32 + 0.5) * chunk_size;
            let chunk_center_z = (chunk_z as f32 + 0.5) * chunk_size;
            let distance = Vec2::new(chunk_center_x - player_pos.x, chunk_center_z - player_pos.z).length();

            if distance <= view_distance {
                let lod_level = if distance < view_distance * 0.3 {
                    0
                } else if distance < view_distance * 0.6 {
                    1
                } else {
                    2
                };

                chunks_to_load.insert((chunk_x, chunk_z, lod_level));
            }
        }
    }

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

    for &(chunk_x, chunk_z, lod_level) in chunks_to_load.iter() {
        let key = (chunk_x, chunk_z, lod_level);
        if !chunk_manager.loaded_chunks.contains(&key) {
            spawn_chunk(&mut commands, chunk_x, chunk_z, lod_level, chunk_manager.chunk_size);
            chunk_manager.loaded_chunks.insert(key);
        }
    }
}

/// Spawn a chunk entity
fn spawn_chunk(commands: &mut Commands, chunk_x: i32, chunk_z: i32, lod_level: u8, chunk_size: f32) {
    let chunk_world_x = chunk_x as f32 * chunk_size;
    let chunk_world_z = chunk_z as f32 * chunk_size;

    commands.spawn((
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
    ));
}
