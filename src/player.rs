use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use std::f32::consts::PI;

use crate::bird::{BirdStats, BirdType};
use crate::components::{
    AiBird, Bird, BodyPart, Drafting, FlapState, Obstacle, Player, WindParticle, Wing,
};
use crate::state::{AppState, GameState, SelectedBirdType};
use rand::Rng;

/// Setup player bird entity
pub fn setup_player(
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

    let body_material = materials.add(StandardMaterial {
        base_color,
        perceptual_roughness: 0.8,
        ..default()
    });
    let head_material = materials.add(StandardMaterial {
        base_color: head_color,
        perceptual_roughness: 0.7,
        ..default()
    });
    let wing_material = materials.add(StandardMaterial {
        base_color: wing_color,
        perceptual_roughness: 0.9,
        ..default()
    });
    let wing_tip_material = materials.add(StandardMaterial {
        base_color: wing_tip_color,
        perceptual_roughness: 0.9,
        ..default()
    });
    let belly_material = materials.add(StandardMaterial {
        base_color: belly_color,
        perceptual_roughness: 0.6,
        ..default()
    });
    let beak_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.7, 0.2),
        perceptual_roughness: 0.5,
        ..default()
    });
    let feet_material = beak_material.clone();
    let eye_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.05, 0.05),
        perceptual_roughness: 0.1,
        metallic: 0.5,
        ..default()
    });
    let eye_white = materials.add(StandardMaterial {
        base_color: Color::srgb(0.98, 0.98, 0.98),
        perceptual_roughness: 0.3,
        ..default()
    });

    commands
        .spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: body_material.clone(),
                transform: Transform::from_xyz(0.0, 125.0, 0.0).with_scale(Vec3::new(
                    0.75 * player_scale * 3.0,
                    0.7 * player_scale * 3.0,
                    1.1 * player_scale * 3.0,
                )),
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
            Drafting { is_drafting: false },
            BodyPart::Body,
        ))
        .with_children(|parent| {
            // Belly (lighter underside)
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: belly_material,
                transform: Transform::from_xyz(0.0, -0.35, 0.0)
                    .with_scale(Vec3::new(0.85, 0.4, 0.9)),
                ..default()
            });

            // Chest (front body bulge)
            parent.spawn(PbrBundle {
                mesh: voxel.clone(),
                material: body_material,
                transform: Transform::from_xyz(0.0, 0.1, 0.4)
                    .with_scale(Vec3::new(0.7, 0.6, 0.4)),
                ..default()
            });

            // Head
            parent
                .spawn((
                    PbrBundle {
                        mesh: voxel.clone(),
                        material: head_material,
                        transform: Transform::from_xyz(0.0, 0.15, 0.65)
                            .with_scale(Vec3::new(0.65, 0.6, 0.6)),
                        ..default()
                    },
                    BodyPart::Head,
                ))
                .with_children(|head| {
                    // Crown/top of head
                    head.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: wing_material.clone(),
                        transform: Transform::from_xyz(0.0, 0.45, -0.1)
                            .with_scale(Vec3::new(0.5, 0.25, 0.5)),
                        ..default()
                    });
                    // Beak - upper
                    head.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: beak_material.clone(),
                        transform: Transform::from_xyz(0.0, 0.05, 0.55)
                            .with_scale(Vec3::new(0.3, 0.15, 0.5)),
                        ..default()
                    });
                    // Beak - lower
                    head.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: beak_material,
                        transform: Transform::from_xyz(0.0, -0.1, 0.5)
                            .with_scale(Vec3::new(0.22, 0.1, 0.35)),
                        ..default()
                    });
                    // Left eye socket
                    head.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: eye_white.clone(),
                        transform: Transform::from_xyz(-0.38, 0.15, 0.25)
                            .with_scale(Vec3::new(0.18, 0.28, 0.22)),
                        ..default()
                    });
                    // Left pupil
                    head.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: eye_material.clone(),
                        transform: Transform::from_xyz(-0.45, 0.15, 0.32)
                            .with_scale(Vec3::splat(0.12)),
                        ..default()
                    });
                    // Right eye socket
                    head.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: eye_white,
                        transform: Transform::from_xyz(0.38, 0.15, 0.25)
                            .with_scale(Vec3::new(0.18, 0.28, 0.22)),
                        ..default()
                    });
                    // Right pupil
                    head.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: eye_material,
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

            // Tail feathers - fan pattern
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

            // Left wing - multi-segment with feathers
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
                    // Wing mid section
                    wing.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: wing_material.clone(),
                        transform: Transform::from_xyz(-0.85, 0.0, -0.05)
                            .with_scale(Vec3::new(0.9, 0.9, 0.85)),
                        ..default()
                    });
                    // Wing tip
                    wing.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: wing_tip_material.clone(),
                        transform: Transform::from_xyz(-1.5, 0.0, -0.15)
                            .with_scale(Vec3::new(0.55, 0.7, 0.7)),
                        ..default()
                    });
                    // Primary feathers
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

            // Right wing - multi-segment with feathers
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
                    // Wing mid section
                    wing.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: wing_material.clone(),
                        transform: Transform::from_xyz(0.85, 0.0, -0.05)
                            .with_scale(Vec3::new(0.9, 0.9, 0.85)),
                        ..default()
                    });
                    // Wing tip
                    wing.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: wing_tip_material.clone(),
                        transform: Transform::from_xyz(1.5, 0.0, -0.15)
                            .with_scale(Vec3::new(0.55, 0.7, 0.7)),
                        ..default()
                    });
                    // Primary feathers
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

            // Feet/legs (tucked when flying)
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

    println!(
        "Playing as {} bird! Scale: {:.1}x",
        player_bird_type.name(),
        player_scale * 3.0
    );
}

/// Grab cursor on game start
pub fn grab_cursor(mut windows: Query<&mut Window>) {
    if let Ok(mut window) = windows.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
}

/// Toggle cursor grab on ESC/click
pub fn cursor_toggle(
    mut windows: Query<&mut Window>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let Ok(mut window) = windows.get_single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }

    if mouse.just_pressed(MouseButton::Left) && window.cursor.grab_mode == CursorGrabMode::None {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
}

/// Handle player input for pitch/yaw control
pub fn player_input(
    mut query: Query<(&mut Bird, &BirdStats), With<Player>>,
    mut motion_events: EventReader<bevy::input::mouse::MouseMotion>,
    time: Res<Time>,
) {
    let Ok((mut bird, stats)) = query.get_single_mut() else {
        return;
    };
    let dt = time.delta_seconds();

    let base_sensitivity = 0.0015;
    let mouse_sensitivity = base_sensitivity * stats.turn_rate;
    let mut yaw_delta = 0.0;

    for event in motion_events.read() {
        yaw_delta = -event.delta.x * mouse_sensitivity;
        bird.yaw += yaw_delta;
        bird.pitch = (bird.pitch + event.delta.y * mouse_sensitivity)
            .clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
    }

    let target_roll = -yaw_delta * 60.0;
    bird.roll = bird.roll + (target_roll - bird.roll) * stats.roll_rate * dt;
    bird.roll *= 0.97;

    let optimal_pitch = 0.0;
    let no_input = yaw_delta.abs() < 0.0001;

    if no_input {
        bird.pitch = bird.pitch + (optimal_pitch - bird.pitch) * 2.0 * dt;
    }
}

/// Handle bird movement physics
pub fn bird_movement(
    mut query: Query<(&mut Bird, &mut Transform, Option<&BirdStats>), Without<AiBird>>,
    camera_query: Query<&Transform, (With<Camera3d>, Without<Bird>)>,
    obstacle_query: Query<(&Transform, &Obstacle), Without<Bird>>,
    flap_state: Res<FlapState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    let (cam_forward, cam_right) = camera_query
        .get_single()
        .map(|t| {
            let forward = t.forward();
            let right = t.right();
            let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
            let right_xz = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
            (forward_xz, right_xz)
        })
        .unwrap_or((Vec3::NEG_Z, Vec3::X));

    let default_stats = BirdStats::for_type(BirdType::Hawk);

    for (mut bird, mut transform, stats_opt) in query.iter_mut() {
        let stats = stats_opt.unwrap_or(&default_stats);

        // Handle grounded walking
        if bird.grounded {
            let bird_pos = transform.translation;
            let mut has_ground_support = bird_pos.y <= 1.0;

            if !has_ground_support {
                for (obs_transform, obstacle) in obstacle_query.iter() {
                    let obs_pos = obs_transform.translation;
                    let half = obstacle.half_extents;
                    let obstacle_top = obs_pos.y + half.y;

                    let in_xz_bounds = bird_pos.x > obs_pos.x - half.x - 1.0
                        && bird_pos.x < obs_pos.x + half.x + 1.0
                        && bird_pos.z > obs_pos.z - half.z - 1.0
                        && bird_pos.z < obs_pos.z + half.z + 1.0;

                    let near_top = bird_pos.y < obstacle_top + 3.0 && bird_pos.y > obstacle_top - 2.0;

                    if in_xz_bounds && near_top {
                        has_ground_support = true;
                        break;
                    }
                }
            }

            if !has_ground_support {
                bird.grounded = false;
                bird.velocity = Vec3::new(0.0, -2.0, 0.0);
                continue;
            }

            let walk_speed = stats.walk_speed;
            let mut world_move = Vec3::ZERO;
            let mut should_turn = false;

            if keyboard.pressed(KeyCode::KeyW) {
                world_move += cam_forward;
                should_turn = true;
            }
            if keyboard.pressed(KeyCode::KeyS) {
                world_move -= cam_forward;
            }
            if keyboard.pressed(KeyCode::KeyA) {
                world_move -= cam_right;
            }
            if keyboard.pressed(KeyCode::KeyD) {
                world_move += cam_right;
            }

            bird.is_walking = world_move.length_squared() > 0.0;

            if bird.is_walking {
                world_move = world_move.normalize();
                transform.translation.x += world_move.x * walk_speed * dt;
                transform.translation.z += world_move.z * walk_speed * dt;
                bird.walk_timer += dt * 12.0;

                if should_turn {
                    let target_yaw = cam_forward.x.atan2(cam_forward.z);
                    let mut yaw_diff = target_yaw - bird.yaw;
                    while yaw_diff > PI {
                        yaw_diff -= 2.0 * PI;
                    }
                    while yaw_diff < -PI {
                        yaw_diff += 2.0 * PI;
                    }
                    bird.yaw += yaw_diff * 8.0 * dt;
                }

                let bob_angle = (bird.walk_timer).sin() * 0.1;
                let walk_rotation = Quat::from_rotation_y(bird.yaw)
                    * Quat::from_rotation_x(-PI / 2.0 + 0.15 + bob_angle)
                    * Quat::from_rotation_z((bird.walk_timer).sin() * 0.08);
                transform.rotation = transform.rotation.slerp(walk_rotation, 10.0 * dt);
            } else {
                bird.walk_timer = 0.0;
                let idle_rotation =
                    Quat::from_rotation_y(bird.yaw) * Quat::from_rotation_x(-PI / 2.0);
                transform.rotation = transform.rotation.slerp(idle_rotation, 5.0 * dt);
            }
            continue;
        }

        // Flying physics
        let bird_rotation = Quat::from_rotation_y(bird.yaw)
            * Quat::from_rotation_x(bird.pitch)
            * Quat::from_rotation_z(bird.roll);

        let forward = bird_rotation * Vec3::Z;
        let up = bird_rotation * Vec3::Y;

        let speed = bird.velocity.length();
        bird.speed = speed;

        let horizontal_speed = Vec3::new(bird.velocity.x, 0.0, bird.velocity.z).length();

        // Speed-based gravity & lift - different abilities per bird type
        if flap_state.space_held {
            match stats.bird_type {
                BirdType::Sparrow => {
                    // QUICK DASH: Forward burst, maintains altitude
                    let dash_acceleration = 60.0;
                    let horizontal_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
                    bird.velocity += horizontal_forward * dash_acceleration * dt;

                    // Slight upward force to maintain altitude
                    bird.velocity.y += 5.0 * dt;

                    // Cap horizontal speed
                    let max_dash_speed = 70.0;
                    let h_speed = Vec3::new(bird.velocity.x, 0.0, bird.velocity.z).length();
                    if h_speed > max_dash_speed {
                        let h_dir = Vec3::new(bird.velocity.x, 0.0, bird.velocity.z).normalize_or_zero();
                        bird.velocity.x = h_dir.x * max_dash_speed;
                        bird.velocity.z = h_dir.z * max_dash_speed;
                    }
                }
                BirdType::Hawk => {
                    // POWER DIVE: Close wings, dive down, gain massive speed
                    let max_gravity = 1000.0;
                    let ramp_time = 4.5;
                    let gravity_factor = (flap_state.wings_closed_time / ramp_time).min(1.0);
                    bird.velocity.y -= max_gravity * gravity_factor * dt;

                    let fall_speed = (-bird.velocity.y).max(0.0);
                    if fall_speed > 1.0 {
                        let horizontal_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
                        let acceleration = fall_speed * 6.0;
                        bird.velocity += horizontal_forward * acceleration * dt;

                        if bird.velocity.length() > stats.max_dive_speed {
                            bird.velocity = bird.velocity.normalize_or_zero() * stats.max_dive_speed;
                        }
                    }
                }
                BirdType::Eagle => {
                    // THERMAL SOAR: Catch updraft, gain altitude without flapping
                    let thermal_strength = 25.0;
                    let ramp_time = 2.0;
                    let thermal_factor = (flap_state.wings_closed_time / ramp_time).min(1.0);

                    // Rise upward
                    bird.velocity.y += thermal_strength * thermal_factor * dt;

                    // Slight forward momentum loss while soaring
                    let h_speed = Vec3::new(bird.velocity.x, 0.0, bird.velocity.z).length();
                    if h_speed > stats.min_glide_speed {
                        bird.velocity.x *= 1.0 - 0.3 * dt;
                        bird.velocity.z *= 1.0 - 0.3 * dt;
                    }

                    // Cap upward speed
                    bird.velocity.y = bird.velocity.y.min(20.0);
                }
                BirdType::Albatross => {
                    // DYNAMIC SOARING: Use wind currents to elevate gradually
                    let wind_lift = 18.0;
                    let ramp_time = 3.0;
                    let soar_factor = (flap_state.wings_closed_time / ramp_time).min(1.0);

                    // Gradual lift from wind
                    bird.velocity.y += wind_lift * soar_factor * dt;

                    // Also gains some forward speed from wind
                    let horizontal_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
                    bird.velocity += horizontal_forward * 8.0 * soar_factor * dt;

                    // Cap speeds
                    bird.velocity.y = bird.velocity.y.min(15.0);
                    if bird.velocity.length() > stats.max_dive_speed * 0.5 {
                        bird.velocity = bird.velocity.normalize_or_zero() * stats.max_dive_speed * 0.5;
                    }
                }
            }
        } else {
            let min_speed_for_glide = stats.min_glide_speed;
            let perfect_glide_speed = stats.perfect_glide_speed;
            let base_gravity = 15.0 * stats.glide_efficiency;

            if horizontal_speed >= perfect_glide_speed {
                let wing_level = up.y.max(0.0);
                let min_fall = 5.0 * stats.glide_efficiency;
                bird.velocity.y -= min_fall * (1.0 - wing_level * 0.5) * dt;
            } else if horizontal_speed >= min_speed_for_glide {
                let glide_quality =
                    (horizontal_speed - min_speed_for_glide) / (perfect_glide_speed - min_speed_for_glide);
                let wing_level = up.y.max(0.0);
                let gravity = base_gravity * (1.0 - glide_quality * 0.5 * wing_level);
                bird.velocity.y -= gravity * dt;
            } else {
                let slow_factor = (min_speed_for_glide - horizontal_speed) / min_speed_for_glide;
                let gravity = base_gravity + slow_factor * 50.0;
                bird.velocity.y -= gravity * dt;
            }
        }

        // Pitch-based speed change
        if !flap_state.space_held {
            let pitch_factor = -forward.y;

            if pitch_factor > 0.05 {
                let acceleration = pitch_factor * stats.acceleration;
                bird.velocity += forward * acceleration * dt;
                if bird.velocity.length() > stats.max_dive_speed {
                    bird.velocity = bird.velocity.normalize_or_zero() * stats.max_dive_speed;
                }
            } else if pitch_factor < -0.3 {
                let climb_amount = (-pitch_factor - 0.3).max(0.0);
                let deceleration = climb_amount * 6.0;
                let speed_loss = (deceleration * dt).min(speed * 0.2);
                if speed > 5.0 {
                    let vel_dir = bird.velocity.normalize_or_zero();
                    bird.velocity -= vel_dir * speed_loss;
                }
            }
        }

        // Drag
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

            let current_speed = bird.velocity.length();
            if current_speed < cruise_speed && current_speed > 0.1 {
                bird.velocity = bird.velocity.normalize_or_zero() * cruise_speed;
            }
        } else if flap_state.space_held {
            let drag = bird.velocity * bird.velocity.length() * 0.005;
            bird.velocity -= drag * dt;
        }

        // Yaw turning
        if speed > 1.0 {
            let target_vel = forward * speed;
            bird.velocity = bird.velocity.lerp(target_vel, 2.0 * dt);
        }

        // Apply velocity
        transform.translation += bird.velocity * dt;

        // Rotate bird
        if bird.grounded {
            let idle_rotation = Quat::from_rotation_y(bird.yaw);
            transform.rotation = transform.rotation.slerp(idle_rotation, 5.0 * dt);
        } else {
            let mut target_rotation = Quat::from_rotation_y(bird.yaw)
                * Quat::from_rotation_x(bird.pitch)
                * Quat::from_rotation_z(bird.roll);

            if bird.damage_timer > 0.0 {
                let shake = (bird.damage_timer * 40.0).sin() * 0.3;
                target_rotation = target_rotation * Quat::from_rotation_z(shake);
            }

            transform.rotation = transform.rotation.slerp(target_rotation, 5.0 * dt);
        }
    }
}

/// Handle obstacle collision
pub fn obstacle_collision(
    mut bird_query: Query<(&mut Bird, &mut Transform, &BirdStats), With<Player>>,
    obstacle_query: Query<(&Transform, &Obstacle), Without<Player>>,
    time: Res<Time>,
) {
    let Ok((mut bird, mut bird_transform, bird_stats)) = bird_query.get_single_mut() else {
        return;
    };
    let dt = time.delta_seconds();

    if bird.damage_timer > 0.0 {
        bird.damage_timer -= dt;
    }

    if bird.grounded {
        return;
    }

    let bird_pos = bird_transform.translation;
    let bird_scale = bird_stats.bird_type.scale();
    let bird_radius = 2.4 * bird_scale;

    for (obs_transform, obstacle) in obstacle_query.iter() {
        let obs_pos = obs_transform.translation;
        let half = obstacle.half_extents;
        let obstacle_top = obs_pos.y + half.y;

        let closest_x = bird_pos.x.clamp(obs_pos.x - half.x, obs_pos.x + half.x);
        let closest_y = bird_pos.y.clamp(obs_pos.y - half.y, obs_pos.y + half.y);
        let closest_z = bird_pos.z.clamp(obs_pos.z - half.z, obs_pos.z + half.z);

        let distance = ((bird_pos.x - closest_x).powi(2)
            + (bird_pos.y - closest_y).powi(2)
            + (bird_pos.z - closest_z).powi(2))
        .sqrt();

        if distance > bird_radius {
            continue;
        }

        let in_xz_bounds = bird_pos.x > obs_pos.x - half.x - 1.0
            && bird_pos.x < obs_pos.x + half.x + 1.0
            && bird_pos.z > obs_pos.z - half.z - 1.0
            && bird_pos.z < obs_pos.z + half.z + 1.0;

        let bird_above_obstacle = bird_pos.y > obs_pos.y;
        let moving_down = bird.velocity.y < 0.0;
        let is_slow = bird.velocity.length() < 5.0;

        if in_xz_bounds && bird_above_obstacle && (moving_down || is_slow) {
            let landing_height = 0.5;
            bird_transform.translation.y = obstacle_top + landing_height;
            bird.velocity = Vec3::ZERO;
            bird.grounded = true;
            bird.pitch = 0.0;
            bird.roll = 0.0;
            return;
        }

        if bird_pos.y > obstacle_top - 5.0 {
            continue;
        }

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

/// Handle wing flapping
pub fn wing_flap(
    mut wing_query: Query<(&Wing, &mut Transform)>,
    mut head_query: Query<(&BodyPart, &mut Transform), Without<Wing>>,
    mut player_query: Query<(&mut Bird, &BirdStats, &Children), With<Player>>,
    mut flap_state: ResMut<FlapState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let flap_duration = 0.3;
    let wing_closed_angle = 1.5;
    let dt = time.delta_seconds();

    let player_data = player_query
        .get_single()
        .map(|(b, stats, children)| {
            (
                b.grounded,
                b.is_walking,
                b.walk_timer,
                stats.bird_type,
                children.iter().copied().collect::<Vec<_>>(),
            )
        })
        .ok();

    let (is_grounded, is_walking, walk_timer, bird_type, player_children) = match player_data {
        Some((g, w, t, bt, c)) => (g, w, t, bt, c),
        None => return,
    };

    if flap_state.cooldown > 0.0 {
        flap_state.cooldown -= dt;
    }

    if keyboard.pressed(KeyCode::Space) {
        flap_state.space_held = true;
        flap_state.wings_closed_time += dt;
    } else {
        flap_state.space_held = false;
        flap_state.wings_closed_time = 0.0;
    }

    if keyboard.just_pressed(KeyCode::Space) && flap_state.cooldown > 0.0 {
        flap_state.timer = 0.0;
    }

    if keyboard.just_pressed(KeyCode::Space) && flap_state.cooldown <= 0.0 {
        if let Ok((mut bird, bird_stats, _)) = player_query.get_single_mut() {
            flap_state.timer = flap_duration;
            flap_state.cooldown = bird_stats.flap_cooldown;

            if bird.grounded {
                bird.grounded = false;
                bird.velocity = Vec3::new(0.0, 15.0, bird_stats.perfect_glide_speed);
                let yaw_rotation = Quat::from_rotation_y(bird.yaw);
                bird.velocity = yaw_rotation * bird.velocity;
            } else {
                let bird_rotation = Quat::from_rotation_y(bird.yaw)
                    * Quat::from_rotation_x(bird.pitch)
                    * Quat::from_rotation_z(bird.roll);

                let local_up = bird_rotation * Vec3::Y;
                let local_forward = bird_rotation * Vec3::Z;

                let thrust = bird_stats.flap_thrust;
                // Sparrow gets double forward speed per flap
                let forward_mult = match bird_stats.bird_type {
                    BirdType::Sparrow => 0.6,
                    _ => 0.3,
                };
                bird.velocity += local_up * thrust + local_forward * (thrust * forward_mult);
            }
        }
    }

    if flap_state.timer > 0.0 {
        flap_state.timer -= dt;
    }

    // Wing animation
    for &child in player_children.iter() {
        if let Ok((wing, mut transform)) = wing_query.get_mut(child) {
            let base_rotation = Quat::IDENTITY;

            if is_grounded {
                // When grounded, body is rotated -PI/2 around X (head up)
                // Wings need to fold along the body using Y rotation (tuck against sides)
                // and X rotation to angle them down against the body
                let fold_y = 1.4; // Tuck wings back along body
                let fold_x = 0.3; // Angle wings slightly down
                if is_walking {
                    let swing = (walk_timer).sin() * 0.3;
                    if wing.is_left {
                        transform.rotation = base_rotation
                            * Quat::from_rotation_y(-fold_y)
                            * Quat::from_rotation_x(fold_x)
                            * Quat::from_rotation_z(swing);
                    } else {
                        transform.rotation = base_rotation
                            * Quat::from_rotation_y(fold_y)
                            * Quat::from_rotation_x(fold_x)
                            * Quat::from_rotation_z(-swing);
                    }
                } else {
                    if wing.is_left {
                        transform.rotation = base_rotation
                            * Quat::from_rotation_y(-fold_y)
                            * Quat::from_rotation_x(fold_x);
                    } else {
                        transform.rotation = base_rotation
                            * Quat::from_rotation_y(fold_y)
                            * Quat::from_rotation_x(fold_x);
                    }
                }
            } else if flap_state.timer > 0.0 {
                let progress = 1.0 - (flap_state.timer / flap_duration);
                let angle = (progress * PI).sin() * 0.8;

                if wing.is_left {
                    transform.rotation = base_rotation * Quat::from_rotation_z(angle);
                } else {
                    transform.rotation = base_rotation * Quat::from_rotation_z(-angle);
                }
            } else if flap_state.space_held && flap_state.wings_closed_time >= 0.5 {
                // Different wing poses for each bird's ability
                match bird_type {
                    BirdType::Sparrow => {
                        // QUICK DASH: Wings swept back, streamlined
                        let sweep_back = 1.2;
                        let tuck_angle = 0.8;
                        if wing.is_left {
                            transform.rotation = base_rotation
                                * Quat::from_rotation_y(-sweep_back)
                                * Quat::from_rotation_z(tuck_angle);
                        } else {
                            transform.rotation = base_rotation
                                * Quat::from_rotation_y(sweep_back)
                                * Quat::from_rotation_z(-tuck_angle);
                        }
                    }
                    BirdType::Hawk => {
                        // POWER DIVE: Wings fully closed against body
                        if wing.is_left {
                            transform.rotation = base_rotation * Quat::from_rotation_z(wing_closed_angle);
                        } else {
                            transform.rotation = base_rotation * Quat::from_rotation_z(-wing_closed_angle);
                        }
                    }
                    BirdType::Eagle => {
                        // THERMAL SOAR: Wings spread wide and slightly raised
                        let spread_angle = -0.3; // Slight upward angle
                        let forward_tilt = 0.2;
                        if wing.is_left {
                            transform.rotation = base_rotation
                                * Quat::from_rotation_z(spread_angle)
                                * Quat::from_rotation_y(-forward_tilt);
                        } else {
                            transform.rotation = base_rotation
                                * Quat::from_rotation_z(-spread_angle)
                                * Quat::from_rotation_y(forward_tilt);
                        }
                    }
                    BirdType::Albatross => {
                        // DYNAMIC SOARING: Wings fully extended, slight wave motion
                        let wave = (flap_state.wings_closed_time * 3.0).sin() * 0.1;
                        let spread_angle = -0.2 + wave;
                        if wing.is_left {
                            transform.rotation = base_rotation * Quat::from_rotation_z(spread_angle);
                        } else {
                            transform.rotation = base_rotation * Quat::from_rotation_z(-spread_angle);
                        }
                    }
                }
            } else {
                transform.rotation = base_rotation;
            }
        }
    }

    // Head animation
    for &child in player_children.iter() {
        if let Ok((body_part, mut transform)) = head_query.get_mut(child) {
            if matches!(body_part, BodyPart::Head) {
                if is_grounded {
                    let head_forward = Quat::from_rotation_x(PI / 2.0);
                    transform.rotation = transform.rotation.slerp(head_forward, 10.0 * dt);
                } else {
                    transform.rotation = transform.rotation.slerp(Quat::IDENTITY, 10.0 * dt);
                }
            }
        }
    }
}

/// Camera follow system
pub fn camera_follow(
    player_query: Query<(&Bird, &Transform, &BirdStats), With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
    time: Res<Time>,
    mut frame_count: Local<u32>,
) {
    let Ok((bird, player_transform, stats)) = player_query.get_single() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.get_single_mut() else {
        return;
    };
    let dt = time.delta_seconds();

    let flight_rotation = if bird.grounded {
        Quat::from_rotation_y(bird.yaw) * Quat::from_rotation_x(bird.pitch)
    } else {
        Quat::from_rotation_y(bird.yaw)
            * Quat::from_rotation_x(bird.pitch)
            * Quat::from_rotation_z(bird.roll)
    };

    let flight_forward = flight_rotation * Vec3::Z;
    let bird_up = flight_rotation * Vec3::Y;

    // Adjust camera distance based on bird size
    let camera_distance_behind = match stats.bird_type {
        BirdType::Sparrow => 25.0,  // Closer for small bird
        BirdType::Hawk => 45.0,
        BirdType::Eagle => 50.0,
        BirdType::Albatross => 55.0,
    };
    let camera_height_above = match stats.bird_type {
        BirdType::Sparrow => 0.0,
        _ => 1.0,
    };

    let target_pos =
        player_transform.translation - flight_forward * camera_distance_behind + bird_up * camera_height_above;

    let target_rotation = flight_rotation * Quat::from_rotation_y(PI);

    let smooth_factor = (10.0 * dt).min(1.0);
    camera_transform.translation = camera_transform.translation.lerp(target_pos, smooth_factor);
    camera_transform.rotation = camera_transform.rotation.slerp(target_rotation, smooth_factor);

    *frame_count += 1;
    if *frame_count <= 5 || *frame_count % 120 == 0 {
        let cam_look_direction = target_rotation * Vec3::NEG_Z;
        let bird_to_camera = camera_transform.translation - player_transform.translation;
        let camera_relative = bird_to_camera.normalize_or_zero();
        println!(
            "Camera[{}]: bird_pos={:?}, cam_pos={:?}, target_pos={:?}, bird(ypr)=({:.1},{:.1},{:.1})",
            *frame_count,
            player_transform.translation,
            camera_transform.translation,
            target_pos,
            bird.yaw.to_degrees(),
            bird.pitch.to_degrees(),
            bird.roll.to_degrees()
        );
        println!(
            "  flight_forward={:?}, cam_look_dir={:?}, bird_up={:?}",
            flight_forward, cam_look_direction, bird_up
        );
        println!(
            "  bird_to_camera={:?}, dist={:.1}, relative={:?}",
            bird_to_camera,
            bird_to_camera.length(),
            camera_relative
        );
    }
}

/// Drafting detection system
pub fn drafting_system(
    mut player_query: Query<(&Transform, &mut Drafting), With<Player>>,
    ai_query: Query<&Transform, (With<AiBird>, Without<Player>)>,
) {
    let Ok((player_transform, mut drafting)) = player_query.get_single_mut() else {
        return;
    };
    let player_pos = player_transform.translation;

    drafting.is_drafting = false;

    for ai_transform in ai_query.iter() {
        let ai_pos = ai_transform.translation;
        let to_ai = ai_pos - player_pos;
        let distance = to_ai.length();

        if distance < 8.0 && distance > 1.0 {
            if to_ai.z > 0.0 && to_ai.z.abs() > to_ai.x.abs() * 2.0 {
                if (to_ai.y).abs() < 3.0 {
                    drafting.is_drafting = true;
                    break;
                }
            }
        }
    }
}

/// Spawn wind particles for Albatross ability
pub fn wind_particle_spawner(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_query: Query<(&Transform, &BirdStats), With<Player>>,
    flap_state: Res<FlapState>,
    time: Res<Time>,
    mut spawn_timer: Local<f32>,
) {
    let Ok((player_transform, stats)) = player_query.get_single() else {
        return;
    };

    // Only spawn for Albatross during ability
    if stats.bird_type != BirdType::Albatross || !flap_state.space_held || flap_state.wings_closed_time < 0.5 {
        *spawn_timer = 0.0;
        return;
    }

    *spawn_timer += time.delta_seconds();

    // Spawn particles every 0.05 seconds
    if *spawn_timer >= 0.05 {
        *spawn_timer = 0.0;

        let mut rng = rand::thread_rng();
        let player_pos = player_transform.translation;

        // Spawn 2-3 particles
        for _ in 0..rng.gen_range(2..4) {
            let offset = Vec3::new(
                rng.gen_range(-15.0..15.0),
                rng.gen_range(-5.0..10.0),
                rng.gen_range(-10.0..20.0),
            );

            let particle_velocity = Vec3::new(
                rng.gen_range(-5.0..5.0),
                rng.gen_range(8.0..15.0),  // Upward to show lift
                rng.gen_range(-30.0..-15.0),  // Moving past the bird
            );

            let size = rng.gen_range(0.3..0.8);

            commands.spawn((
                PbrBundle {
                    mesh: meshes.add(Cuboid::new(size, size * 0.3, size * 2.0)),
                    material: materials.add(StandardMaterial {
                        base_color: Color::srgba(0.9, 0.95, 1.0, 0.4),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    }),
                    transform: Transform::from_translation(player_pos + offset)
                        .with_rotation(Quat::from_rotation_y(rng.gen_range(0.0..PI))),
                    ..default()
                },
                WindParticle {
                    lifetime: rng.gen_range(0.4..0.8),
                    velocity: particle_velocity,
                },
            ));
        }
    }
}

/// Update and despawn wind particles
pub fn wind_particle_update(
    mut commands: Commands,
    mut particle_query: Query<(Entity, &mut Transform, &mut WindParticle)>,
    time: Res<Time>,
) {
    let dt = time.delta_seconds();

    for (entity, mut transform, mut particle) in particle_query.iter_mut() {
        particle.lifetime -= dt;

        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        // Move particle
        transform.translation += particle.velocity * dt;

        // Fade effect by scaling down
        let scale_factor = particle.lifetime * 2.0;
        transform.scale = Vec3::splat(scale_factor.min(1.0));
    }
}

/// Handle reset to bird selection when R is pressed
pub fn reset_to_selection(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut windows: Query<&mut Window>,
) {
    if keyboard.just_pressed(KeyCode::KeyR) {
        // Release cursor before going back to selection
        if let Ok(mut window) = windows.get_single_mut() {
            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
        }
        next_state.set(AppState::BirdSelection);
    }
}

/// Cleanup player when exiting Playing state
pub fn cleanup_player(
    mut commands: Commands,
    player_query: Query<Entity, With<Player>>,
    mut game_state: ResMut<GameState>,
    mut flap_state: ResMut<FlapState>,
) {
    // Despawn player entity
    for entity in player_query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    // Reset game state
    game_state.game_over = false;
    game_state.won = false;

    // Reset flap state
    flap_state.timer = 0.0;
    flap_state.cooldown = 0.0;
    flap_state.space_held = false;
    flap_state.wings_closed_time = 0.0;
}
