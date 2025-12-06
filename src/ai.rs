use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::audio::{PlaySound, SoundEffect};
use crate::bird::{BirdStats, BirdType};
use crate::components::{AiBird, AiSparrow, Bird, BodyPart, Captured, HuntingStrike, Obstacle, Player, PlayerScore, Prey, Wing};

/// AI bird movement system
pub fn ai_bird_movement(
    mut query: Query<(&Bird, &mut Transform), With<AiBird>>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (bird, mut transform) in query.iter_mut() {
        let direction = Vec3::new(
            bird.yaw.sin() * bird.pitch.cos(),
            -bird.pitch.sin(),
            bird.yaw.cos() * bird.pitch.cos(),
        )
        .normalize();

        transform.translation += direction * bird.speed * dt;
        transform.translation.y = transform.translation.y.max(2.0);

        let target_rotation = Quat::from_rotation_y(bird.yaw)
            * Quat::from_rotation_x(bird.pitch)
            * Quat::from_rotation_z(bird.roll);
        transform.rotation = transform.rotation.slerp(target_rotation, 5.0 * dt);
    }
}

/// AI bird behavior/wandering system
pub fn ai_bird_behavior(
    mut query: Query<(&mut Bird, &mut AiBird, &Transform)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (mut bird, mut ai, transform) in query.iter_mut() {
        ai.wander_timer -= dt;
        if ai.wander_timer <= 0.0 {
            let mut rng = rand::thread_rng();
            ai.wander_timer = rng.gen_range(2.0..5.0);
            ai.wander_direction = rng.gen_range(-0.3..0.3);
            ai.target_height = rng.gen_range(12.0..28.0);
        }

        bird.yaw += ai.wander_direction * dt;

        let height_diff = ai.target_height - transform.translation.y;
        bird.pitch = (height_diff * 0.1).clamp(-0.3, 0.3);

        if bird.yaw.abs() > PI / 4.0 {
            bird.yaw *= 0.95;
        }
    }
}

/// Spawn AI sparrows around the map
pub fn spawn_ai_sparrows(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();
    let sparrow_count = 12;

    // Sparrow colors (same as player sparrow)
    let base_color = BirdType::Sparrow.color();
    let rgba = base_color.to_srgba();
    let head_color = Color::srgb(
        (rgba.red * 1.15).min(1.0),
        (rgba.green * 1.15).min(1.0),
        (rgba.blue * 1.15).min(1.0),
    );
    let wing_color = Color::srgb(rgba.red * 0.75, rgba.green * 0.75, rgba.blue * 0.75);
    let wing_tip_color = Color::srgb(rgba.red * 0.5, rgba.green * 0.5, rgba.blue * 0.5);
    let belly_color = Color::srgb(
        (rgba.red * 1.3).min(1.0),
        (rgba.green * 1.3).min(1.0),
        (rgba.blue * 1.3).min(1.0),
    );

    let voxel = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let sparrow_scale = BirdType::Sparrow.scale();
    let sparrow_stats = BirdStats::for_type(BirdType::Sparrow);

    let body_material = materials.add(StandardMaterial {
        base_color,
        perceptual_roughness: 0.8,
        fog_enabled: false,
        ..default()
    });
    let head_material = materials.add(StandardMaterial {
        base_color: head_color,
        perceptual_roughness: 0.7,
        fog_enabled: false,
        ..default()
    });
    let wing_material = materials.add(StandardMaterial {
        base_color: wing_color,
        perceptual_roughness: 0.9,
        fog_enabled: false,
        ..default()
    });
    let wing_tip_material = materials.add(StandardMaterial {
        base_color: wing_tip_color,
        perceptual_roughness: 0.9,
        fog_enabled: false,
        ..default()
    });
    let belly_material = materials.add(StandardMaterial {
        base_color: belly_color,
        perceptual_roughness: 0.6,
        fog_enabled: false,
        ..default()
    });
    let beak_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.7, 0.2),
        perceptual_roughness: 0.5,
        fog_enabled: false,
        ..default()
    });
    let feet_material = beak_material.clone();
    let eye_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.05),
        perceptual_roughness: 0.1,
        metallic: 0.5,
        fog_enabled: false,
        ..default()
    });
    let eye_white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.98, 0.98, 0.98),
        perceptual_roughness: 0.3,
        fog_enabled: false,
        ..default()
    });

    for _ in 0..sparrow_count {
        let x = rng.gen_range(-400.0..400.0);
        let y = rng.gen_range(80.0..200.0);
        let z = rng.gen_range(-400.0..400.0);
        let yaw = rng.gen_range(0.0..PI * 2.0);

        commands
            .spawn((
                PbrBundle {
                    mesh: voxel.clone(),
                    material: body_material.clone(),
                    transform: Transform::from_xyz(x, y, z).with_scale(Vec3::new(
                        0.75 * sparrow_scale * 3.0,
                        0.7 * sparrow_scale * 3.0,
                        1.1 * sparrow_scale * 3.0,
                    )),
                    ..default()
                },
                Bird {
                    speed: sparrow_stats.perfect_glide_speed,
                    pitch: 0.0,
                    yaw,
                    roll: 0.0,
                    velocity: Vec3::ZERO,
                    grounded: false,
                    damage_timer: 0.0,
                    walk_timer: 0.0,
                    is_walking: false,
                },
                sparrow_stats.clone(),
                AiSparrow {
                    wander_timer: rng.gen_range(0.0..3.0),
                    wander_direction: Vec3::new(
                        rng.gen_range(-1.0..1.0),
                        0.0,
                        rng.gen_range(-1.0..1.0),
                    ).normalize_or_zero(),
                    flee_target: None,
                },
                Prey,
                BodyPart::Body,
            ))
            .with_children(|parent| {
                // Belly
                parent.spawn(PbrBundle {
                    mesh: voxel.clone(),
                    material: belly_material.clone(),
                    transform: Transform::from_xyz(0.0, -0.35, 0.0)
                        .with_scale(Vec3::new(0.85, 0.4, 0.9)),
                    ..default()
                });

                // Chest
                parent.spawn(PbrBundle {
                    mesh: voxel.clone(),
                    material: body_material.clone(),
                    transform: Transform::from_xyz(0.0, 0.1, 0.4)
                        .with_scale(Vec3::new(0.7, 0.6, 0.4)),
                    ..default()
                });

                // Head
                parent
                    .spawn((
                        PbrBundle {
                            mesh: voxel.clone(),
                            material: head_material.clone(),
                            transform: Transform::from_xyz(0.0, 0.15, 0.65)
                                .with_scale(Vec3::new(0.65, 0.6, 0.6)),
                            ..default()
                        },
                        BodyPart::Head,
                    ))
                    .with_children(|head| {
                        head.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: wing_material.clone(),
                            transform: Transform::from_xyz(0.0, 0.45, -0.1)
                                .with_scale(Vec3::new(0.5, 0.25, 0.5)),
                            ..default()
                        });
                        head.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: beak_material.clone(),
                            transform: Transform::from_xyz(0.0, 0.05, 0.55)
                                .with_scale(Vec3::new(0.3, 0.15, 0.5)),
                            ..default()
                        });
                        head.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: beak_material.clone(),
                            transform: Transform::from_xyz(0.0, -0.1, 0.5)
                                .with_scale(Vec3::new(0.22, 0.1, 0.35)),
                            ..default()
                        });
                        head.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: eye_white.clone(),
                            transform: Transform::from_xyz(-0.38, 0.15, 0.25)
                                .with_scale(Vec3::new(0.18, 0.28, 0.22)),
                            ..default()
                        });
                        head.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: eye_material.clone(),
                            transform: Transform::from_xyz(-0.45, 0.15, 0.32)
                                .with_scale(Vec3::splat(0.12)),
                            ..default()
                        });
                        head.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: eye_white.clone(),
                            transform: Transform::from_xyz(0.38, 0.15, 0.25)
                                .with_scale(Vec3::new(0.18, 0.28, 0.22)),
                            ..default()
                        });
                        head.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: eye_material.clone(),
                            transform: Transform::from_xyz(0.45, 0.15, 0.32)
                                .with_scale(Vec3::splat(0.12)),
                            ..default()
                        });
                    });

                // Tail base
                parent.spawn((
                    PbrBundle {
                        mesh: voxel.clone(),
                        material: wing_material.clone(),
                        transform: Transform::from_xyz(0.0, 0.0, -0.7)
                            .with_scale(Vec3::new(0.35, 0.15, 0.35)),
                        ..default()
                    },
                    BodyPart::Tail,
                ));

                // Tail feathers
                for i in 0..7 {
                    let spread = (i as f32 - 3.0) * 0.12;
                    let length = 0.55 - (i as f32 - 3.0).abs() * 0.05;
                    let z_offset = -0.85 - (3.0 - (i as f32 - 3.0).abs()) * 0.08;
                    parent.spawn((
                        PbrBundle {
                            mesh: voxel.clone(),
                            material: wing_tip_material.clone(),
                            transform: Transform::from_xyz(spread, 0.0, z_offset)
                                .with_scale(Vec3::new(0.08, 0.03, length)),
                            ..default()
                        },
                        BodyPart::Tail,
                    ));
                }

                // Left wing
                parent
                    .spawn((
                        PbrBundle {
                            mesh: voxel.clone(),
                            material: wing_material.clone(),
                            transform: Transform::from_xyz(-0.55, 0.0, 0.05)
                                .with_scale(Vec3::new(0.6, 0.08, 0.45)),
                            ..default()
                        },
                        Wing {
                            is_left: true,
                            _base_x: -0.55,
                        },
                        BodyPart::Wing,
                    ))
                    .with_children(|wing| {
                        wing.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: wing_material.clone(),
                            transform: Transform::from_xyz(-0.85, 0.0, -0.05)
                                .with_scale(Vec3::new(0.9, 0.9, 0.85)),
                            ..default()
                        });
                        wing.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: wing_tip_material.clone(),
                            transform: Transform::from_xyz(-1.5, 0.0, -0.15)
                                .with_scale(Vec3::new(0.55, 0.7, 0.7)),
                            ..default()
                        });
                        for j in 0..5 {
                            wing.spawn(PbrBundle {
                                mesh: voxel.clone(),
                                material: wing_tip_material.clone(),
                                transform: Transform::from_xyz(
                                    -1.8 - j as f32 * 0.12,
                                    0.0,
                                    -0.2 - j as f32 * 0.08,
                                )
                                .with_scale(Vec3::new(0.18, 0.5, 0.35 - j as f32 * 0.04)),
                                ..default()
                            });
                        }
                    });

                // Right wing
                parent
                    .spawn((
                        PbrBundle {
                            mesh: voxel.clone(),
                            material: wing_material.clone(),
                            transform: Transform::from_xyz(0.55, 0.0, 0.05)
                                .with_scale(Vec3::new(0.6, 0.08, 0.45)),
                            ..default()
                        },
                        Wing {
                            is_left: false,
                            _base_x: 0.55,
                        },
                        BodyPart::Wing,
                    ))
                    .with_children(|wing| {
                        wing.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: wing_material.clone(),
                            transform: Transform::from_xyz(0.85, 0.0, -0.05)
                                .with_scale(Vec3::new(0.9, 0.9, 0.85)),
                            ..default()
                        });
                        wing.spawn(PbrBundle {
                            mesh: voxel.clone(),
                            material: wing_tip_material.clone(),
                            transform: Transform::from_xyz(1.5, 0.0, -0.15)
                                .with_scale(Vec3::new(0.55, 0.7, 0.7)),
                            ..default()
                        });
                        for j in 0..5 {
                            wing.spawn(PbrBundle {
                                mesh: voxel.clone(),
                                material: wing_tip_material.clone(),
                                transform: Transform::from_xyz(
                                    1.8 + j as f32 * 0.12,
                                    0.0,
                                    -0.2 - j as f32 * 0.08,
                                )
                                .with_scale(Vec3::new(0.18, 0.5, 0.35 - j as f32 * 0.04)),
                                ..default()
                            });
                        }
                    });

                // Feet
                for side in [-1.0, 1.0] {
                    parent.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: feet_material.clone(),
                        transform: Transform::from_xyz(side * 0.2, -0.45, -0.1)
                            .with_scale(Vec3::new(0.08, 0.25, 0.15)),
                        ..default()
                    });
                }
            });
    }
}

/// AI Sparrow movement
pub fn ai_sparrow_movement(
    mut query: Query<(&mut Bird, &mut Transform, &AiSparrow), Without<Captured>>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    const MAX_HEIGHT: f32 = 520.0;  // 2x highest mountain
    const GROUND_LEVEL: f32 = 3.0;   // Approximate ground level

    for (mut bird, mut transform, _ai_sparrow) in query.iter_mut() {
        // If grounded, stay on ground
        if bird.grounded {
            transform.translation.y = GROUND_LEVEL;
            // Keep orientation level when grounded
            transform.rotation = transform.rotation.slerp(
                Quat::from_rotation_y(bird.yaw),
                5.0 * dt
            );
            continue;
        }

        // Calculate direction from yaw/pitch
        let direction = Vec3::new(
            bird.yaw.sin() * bird.pitch.cos(),
            -bird.pitch.sin(),
            bird.yaw.cos() * bird.pitch.cos(),
        ).normalize_or_zero();

        // Move sparrow
        transform.translation += direction * bird.speed * dt;

        // Enforce height limits - min when flying, max always
        transform.translation.y = transform.translation.y.clamp(GROUND_LEVEL, MAX_HEIGHT);

        // Soft bounds - let behavior system handle turning back
        let hard_bounds = 1000.0;
        if transform.translation.x.abs() > hard_bounds {
            bird.yaw = PI - bird.yaw;
            transform.translation.x = transform.translation.x.clamp(-hard_bounds, hard_bounds);
        }
        if transform.translation.z.abs() > hard_bounds {
            bird.yaw = -bird.yaw;
            transform.translation.z = transform.translation.z.clamp(-hard_bounds, hard_bounds);
        }

        // Rotation
        let target_rotation = Quat::from_rotation_y(bird.yaw)
            * Quat::from_rotation_x(bird.pitch)
            * Quat::from_rotation_z(bird.roll);
        transform.rotation = transform.rotation.slerp(target_rotation, 5.0 * dt);
    }
}

/// AI Sparrow behavior - wandering and fleeing
pub fn ai_sparrow_behavior(
    mut sparrow_query: Query<(&mut Bird, &mut AiSparrow, &Transform), Without<Captured>>,
    hunter_query: Query<(&Transform, &BirdStats), (With<Player>, Without<AiSparrow>)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    let mut rng = rand::thread_rng();

    // Height and bounds constants
    const MAX_HEIGHT: f32 = 520.0;  // 2x highest mountain (~260)
    const ISLAND_RADIUS: f32 = 800.0;  // Main island area

    // Get hunter position and scale if it's a big bird
    let hunter_info = hunter_query.get_single().ok().and_then(|(t, stats)| {
        if stats.bird_type != BirdType::Sparrow {
            // Fear distance: 15x hunter size (between 10-20x)
            let fear_distance = stats.bird_type.scale() * 5.0 * 15.0;
            Some((t.translation, fear_distance))
        } else {
            None
        }
    });

    for (mut bird, mut ai, transform) in sparrow_query.iter_mut() {
        let pos = transform.translation;

        // Check if hunter is nearby - flee!
        if let Some((hunter_pos, fear_distance)) = hunter_info {
            let to_hunter = hunter_pos - pos;
            let dist = to_hunter.length();

            if dist < fear_distance {
                // Flee from hunter - take off if grounded
                bird.grounded = false;
                ai.flee_target = Some(hunter_pos);
                let flee_dir = -to_hunter.normalize_or_zero();
                let target_yaw = flee_dir.x.atan2(flee_dir.z);

                // Quick turn away
                let mut yaw_diff = target_yaw - bird.yaw;
                while yaw_diff > PI { yaw_diff -= 2.0 * PI; }
                while yaw_diff < -PI { yaw_diff += 2.0 * PI; }
                bird.yaw += yaw_diff * 5.0 * dt;

                // Climb to escape at normal cruising speed (no boost)
                bird.speed = 8.0;
                bird.pitch = -0.3;
                continue;
            }
        }

        // Normal wandering
        ai.flee_target = None;

        // Check if too high - descend
        if pos.y > MAX_HEIGHT {
            bird.pitch = 0.4;  // Dive down
            bird.speed = 8.0;
            continue;
        }

        // Check if leaving island - turn back
        let dist_from_center = (pos.x * pos.x + pos.z * pos.z).sqrt();
        if dist_from_center > ISLAND_RADIUS {
            // Turn toward center
            let to_center = -Vec3::new(pos.x, 0.0, pos.z).normalize_or_zero();
            let target_yaw = to_center.x.atan2(to_center.z);

            let mut yaw_diff = target_yaw - bird.yaw;
            while yaw_diff > PI { yaw_diff -= 2.0 * PI; }
            while yaw_diff < -PI { yaw_diff += 2.0 * PI; }
            bird.yaw += yaw_diff * 3.0 * dt;
            bird.speed = 8.0;
            continue;
        }

        // Maybe land sometimes when safe
        if !bird.grounded && pos.y < 25.0 && rng.gen_ratio(1, 500) {
            // Chance to decide to land
            bird.grounded = true;
            bird.speed = 0.0;
            bird.pitch = 0.0;
        }

        // If grounded, stay still unless scared
        if bird.grounded {
            bird.speed = 0.0;
            bird.pitch = 0.0;
            // Random chance to take off again
            if rng.gen_ratio(1, 300) {
                bird.grounded = false;
                bird.speed = 8.0;
                bird.pitch = -0.3;  // Climb on takeoff
            }
            continue;
        }

        bird.speed = 8.0; // Normal flight speed

        ai.wander_timer -= dt;
        if ai.wander_timer <= 0.0 {
            ai.wander_timer = rng.gen_range(2.0..5.0);
            ai.wander_direction = Vec3::new(
                rng.gen_range(-1.0..1.0),
                0.0,
                rng.gen_range(-1.0..1.0),
            ).normalize_or_zero();
        }

        // Gentle turning
        bird.yaw += ai.wander_direction.x * 0.5 * dt;

        // Vary target height - sometimes fly low, sometimes high
        let target_height = if rng.gen_ratio(1, 100) {
            rng.gen_range(30.0..300.0)
        } else {
            100.0
        };
        let height_diff = target_height - pos.y;
        bird.pitch = (height_diff * 0.02).clamp(-0.3, 0.3);

        // Gentle roll based on turn
        bird.roll = bird.roll * 0.95;
    }
}

/// Hunting system - big birds catch sparrows
pub fn hunting_system(
    mut commands: Commands,
    hunter_query: Query<(Entity, &Transform, &BirdStats), (With<Player>, Without<Captured>, Without<HuntingStrike>)>,
    prey_query: Query<(Entity, &Transform), (With<Prey>, Without<Captured>)>,
    mut score: ResMut<PlayerScore>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut sound_events: EventWriter<PlaySound>,
) {
    let Ok((hunter_entity, hunter_transform, hunter_stats)) = hunter_query.get_single() else {
        return;
    };

    // Only big birds can hunt
    if hunter_stats.bird_type == BirdType::Sparrow {
        return;
    }

    let hunter_pos = hunter_transform.translation;
    // Catch distance based on bird size - 2.5x the bird's scale
    let catch_distance = hunter_stats.bird_type.scale() * 2.5 * 5.0; // 5.0 is base bird body size

    for (prey_entity, prey_transform) in prey_query.iter() {
        let prey_pos = prey_transform.translation;
        let dist = (hunter_pos - prey_pos).length();

        if dist < catch_distance {
            // Caught! Add capture component to prey
            commands.entity(prey_entity).insert(Captured {
                timer: 0.5,
                hunter_pos,
            });

            // Add strike animation to hunter (blocks movement, cinematic camera)
            commands.entity(hunter_entity).insert(HuntingStrike {
                timer: 0.8,  // Longer for cinematic effect
                original_scale: hunter_transform.scale,
                original_pos: hunter_pos,
                prey_pos,
            });

            // Play hunting strike sound
            sound_events.send(PlaySound {
                sound: SoundEffect::HuntingStrike,
            });

            // Play capture success sound
            sound_events.send(PlaySound {
                sound: SoundEffect::CaptureSuccess,
            });

            // Award points
            score.points += 10;
            println!("Caught a sparrow! Score: {}", score.points);

            // Spawn replacement sparrow far from hunter
            spawn_replacement_sparrow(&mut commands, &mut meshes, &mut materials, hunter_pos);

            // Only catch one at a time
            break;
        }
    }
}

/// Spawn a replacement sparrow away from the hunter
fn spawn_replacement_sparrow(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    hunter_pos: Vec3,
) {
    let mut rng = rand::thread_rng();

    // Spawn at random position, but far from hunter
    let mut x;
    let mut z;
    loop {
        x = rng.gen_range(-400.0..400.0);
        z = rng.gen_range(-400.0..400.0);
        let dist = ((x - hunter_pos.x).powi(2) + (z - hunter_pos.z).powi(2)).sqrt();
        if dist > 200.0 {
            break;
        }
    }
    let y = rng.gen_range(80.0..200.0);
    let yaw = rng.gen_range(0.0..PI * 2.0);

    let base_color = BirdType::Sparrow.color();
    let rgba = base_color.to_srgba();
    let wing_color = Color::srgb(rgba.red * 0.75, rgba.green * 0.75, rgba.blue * 0.75);

    let voxel = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let sparrow_scale = BirdType::Sparrow.scale();
    let sparrow_stats = BirdStats::for_type(BirdType::Sparrow);

    let body_material = materials.add(StandardMaterial {
        base_color,
        perceptual_roughness: 0.8,
        fog_enabled: false,
        ..default()
    });

    let wing_material = materials.add(StandardMaterial {
        base_color: wing_color,
        perceptual_roughness: 0.9,
        fog_enabled: false,
        ..default()
    });

    commands.spawn((
        PbrBundle {
            mesh: voxel.clone(),
            material: body_material,
            transform: Transform::from_xyz(x, y, z).with_scale(Vec3::new(
                0.75 * sparrow_scale * 3.0,
                0.7 * sparrow_scale * 3.0,
                1.1 * sparrow_scale * 3.0,
            )),
            ..default()
        },
        Bird {
            speed: sparrow_stats.perfect_glide_speed,
            pitch: 0.0,
            yaw,
            roll: 0.0,
            velocity: Vec3::ZERO,
            grounded: false,
            damage_timer: 0.0,
            walk_timer: 0.0,
            is_walking: false,
        },
        sparrow_stats,
        AiSparrow {
            wander_timer: rng.gen_range(0.0..3.0),
            wander_direction: Vec3::new(
                rng.gen_range(-1.0..1.0),
                0.0,
                rng.gen_range(-1.0..1.0),
            ).normalize_or_zero(),
            flee_target: None,
        },
        Prey,
        BodyPart::Body,
    )).with_children(|parent| {
        // Left wing - makes sparrow visible
        parent.spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: wing_material.clone(),
                transform: Transform::from_xyz(-0.55, 0.0, 0.05)
                    .with_scale(Vec3::new(0.6, 0.08, 0.45)),
                ..default()
            },
            Wing {
                is_left: true,
                _base_x: -0.55,
            },
            BodyPart::Wing,
        )).with_children(|wing| {
            wing.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: wing_material.clone(),
                transform: Transform::from_xyz(-0.85, 0.0, -0.05)
                    .with_scale(Vec3::new(0.9, 0.9, 0.85)),
                ..default()
            });
        });

        // Right wing
        parent.spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: wing_material.clone(),
                transform: Transform::from_xyz(0.55, 0.0, 0.05)
                    .with_scale(Vec3::new(0.6, 0.08, 0.45)),
                ..default()
            },
            Wing {
                is_left: false,
                _base_x: 0.55,
            },
            BodyPart::Wing,
        )).with_children(|wing| {
            wing.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: wing_material.clone(),
                transform: Transform::from_xyz(0.85, 0.0, -0.05)
                    .with_scale(Vec3::new(0.9, 0.9, 0.85)),
                ..default()
            });
        });
    });
}

/// AI Sparrow obstacle collision - despawn on collision and respawn elsewhere
pub fn ai_sparrow_obstacle_collision(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    sparrow_query: Query<(Entity, &Transform, &Bird), (With<AiSparrow>, Without<Captured>)>,
    obstacle_query: Query<(&Transform, &Obstacle)>,
) {
    let sparrow_scale = BirdType::Sparrow.scale();
    let sparrow_radius = 2.4 * sparrow_scale;

    for (entity, transform, bird) in sparrow_query.iter() {
        // Skip grounded sparrows
        if bird.grounded {
            continue;
        }

        let sparrow_pos = transform.translation;

        for (obs_transform, obstacle) in obstacle_query.iter() {
            let obs_pos = obs_transform.translation;
            let half = obstacle.half_extents;

            // Check distance to obstacle (AABB)
            let closest_x = sparrow_pos.x.clamp(obs_pos.x - half.x, obs_pos.x + half.x);
            let closest_y = sparrow_pos.y.clamp(obs_pos.y - half.y, obs_pos.y + half.y);
            let closest_z = sparrow_pos.z.clamp(obs_pos.z - half.z, obs_pos.z + half.z);

            let distance = ((sparrow_pos.x - closest_x).powi(2)
                + (sparrow_pos.y - closest_y).powi(2)
                + (sparrow_pos.z - closest_z).powi(2))
            .sqrt();

            if distance < sparrow_radius {
                // Collision! Despawn this sparrow and spawn a new one
                commands.entity(entity).despawn_recursive();
                spawn_replacement_sparrow(&mut commands, &mut meshes, &mut materials, sparrow_pos);
                break;
            }
        }
    }
}

/// Hunter strike animation - cinematic lunge toward prey
pub fn hunting_strike_animation(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &HuntingStrike), With<Player>>,
    mut wing_query: Query<(&Parent, &mut Transform, &Wing), Without<Player>>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();
    let duration = 0.8;

    for (entity, mut transform, strike) in query.iter_mut() {
        let remaining = strike.timer - dt;

        if remaining <= 0.0 {
            // Reset to original position and scale
            transform.translation = strike.original_pos;
            transform.scale = strike.original_scale;
            commands.entity(entity).remove::<HuntingStrike>();
            continue;
        }

        // Update timer
        commands.entity(entity).insert(HuntingStrike {
            timer: remaining,
            original_scale: strike.original_scale,
            original_pos: strike.original_pos,
            prey_pos: strike.prey_pos,
        });

        // Animation progress (0 to 1)
        let progress = 1.0 - (remaining / duration);

        // Lunge toward prey then back (sine wave: 0 -> 1 -> 0)
        let lunge_factor = (progress * PI).sin();

        // Move toward prey position
        transform.translation = strike.original_pos.lerp(strike.prey_pos, lunge_factor);

        // Stretch body forward during strike
        let base = strike.original_scale;
        transform.scale = Vec3::new(
            base.x * (1.0 - lunge_factor * 0.2),   // 20% narrower
            base.y * (1.0 - lunge_factor * 0.25),  // 25% flatter
            base.z * (1.0 + lunge_factor * 0.5),   // 50% longer
        );

        // Rapid wing flap during strike
        for (parent, mut wing_transform, wing) in wing_query.iter_mut() {
            if parent.get() == entity {
                let flap_speed = 50.0;
                let flap_angle = (time.elapsed_seconds() * flap_speed).sin() * 1.0;
                let base_y = if wing.is_left { 0.15 } else { -0.15 };
                wing_transform.rotation = Quat::from_rotation_z(base_y + flap_angle * if wing.is_left { 1.0 } else { -1.0 });
            }
        }
    }
}

/// Capture animation and despawn
pub fn capture_animation(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Captured)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (entity, mut transform, mut captured) in query.iter_mut() {
        captured.timer -= dt;

        // Squish animation toward hunter
        let progress = 1.0 - (captured.timer / 0.5).max(0.0);
        let squish = 1.0 - progress * 0.9;

        transform.scale.x *= 1.0 + progress * 0.3;
        transform.scale.y *= squish;
        transform.scale.z *= squish;

        // Move toward hunter
        let to_hunter = (captured.hunter_pos - transform.translation).normalize_or_zero();
        transform.translation += to_hunter * 20.0 * dt;

        if captured.timer <= 0.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Respawn sparrows to maintain population
pub fn respawn_sparrows(
    query: Query<&AiSparrow>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut respawn_timer: Local<f32>,
) {
    let current_count = query.iter().count();
    let target_count = 12;

    if current_count >= target_count {
        *respawn_timer = 0.0;
        return;
    }

    *respawn_timer += time.delta_seconds();

    // Respawn one sparrow every 3 seconds if below target
    if *respawn_timer >= 3.0 {
        *respawn_timer = 0.0;

        let mut rng = rand::thread_rng();

        // Simplified sparrow spawn (single mesh for respawns)
        let base_color = BirdType::Sparrow.color();
        let voxel = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
        let sparrow_scale = BirdType::Sparrow.scale();
        let sparrow_stats = BirdStats::for_type(BirdType::Sparrow);

        let body_material = materials.add(StandardMaterial {
            base_color,
            perceptual_roughness: 0.8,
            fog_enabled: false,
            ..default()
        });

        let x = rng.gen_range(-400.0..400.0);
        let y = rng.gen_range(120.0..200.0);
        let z = rng.gen_range(-400.0..400.0);
        let yaw = rng.gen_range(0.0..PI * 2.0);

        // Spawn a simple sparrow (can be expanded to full model later)
        commands.spawn((
            PbrBundle {
                mesh: voxel,
                material: body_material,
                transform: Transform::from_xyz(x, y, z).with_scale(Vec3::new(
                    0.75 * sparrow_scale * 3.0,
                    0.7 * sparrow_scale * 3.0,
                    1.1 * sparrow_scale * 3.0,
                )),
                ..default()
            },
            Bird {
                speed: sparrow_stats.perfect_glide_speed,
                pitch: 0.0,
                yaw,
                roll: 0.0,
                velocity: Vec3::ZERO,
                grounded: false,
                damage_timer: 0.0,
                walk_timer: 0.0,
                is_walking: false,
            },
            sparrow_stats,
            AiSparrow {
                wander_timer: rng.gen_range(0.0..3.0),
                wander_direction: Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    0.0,
                    rng.gen_range(-1.0..1.0),
                ).normalize_or_zero(),
                flee_target: None,
            },
            Prey,
        ));
    }
}

/// AI Sparrow wing flapping animation
pub fn ai_sparrow_wing_animation(
    sparrow_query: Query<(&Children, &AiSparrow), Without<Captured>>,
    mut wing_query: Query<(&Wing, &mut Transform)>,
    time: Res<Time>,
) {
    let t = time.elapsed_seconds();

    for (children, ai_sparrow) in sparrow_query.iter() {
        // Faster flapping when fleeing
        let flap_speed = if ai_sparrow.flee_target.is_some() { 25.0 } else { 15.0 };
        let flap_angle = (t * flap_speed).sin() * 0.6;

        for &child in children.iter() {
            if let Ok((wing, mut transform)) = wing_query.get_mut(child) {
                if wing.is_left {
                    transform.rotation = Quat::from_rotation_z(flap_angle);
                } else {
                    transform.rotation = Quat::from_rotation_z(-flap_angle);
                }
            }
        }
    }
}

/// Passive scoring for sparrow players - based on distance traveled
pub fn sparrow_passive_scoring(
    player_query: Query<(&BirdStats, &Bird), With<Player>>,
    mut score: ResMut<PlayerScore>,
    time: Res<Time>,
    mut sound_events: EventWriter<PlaySound>,
) {
    let Ok((stats, bird)) = player_query.get_single() else {
        return;
    };

    // Only sparrows get passive points
    if stats.bird_type != BirdType::Sparrow {
        return;
    }

    // Calculate distance traveled this frame
    let distance_this_frame = bird.velocity.length() * time.delta_seconds();

    // Accumulate distance in passive_timer (repurposed as distance accumulator)
    score.passive_timer += distance_this_frame;

    // 1 point per 50 units traveled
    if score.passive_timer >= 50.0 {
        score.passive_timer -= 50.0;
        score.points += 1;

        // Play score point sound
        sound_events.send(PlaySound {
            sound: SoundEffect::ScorePoint,
        });
    }
}
