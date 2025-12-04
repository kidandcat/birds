use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use std::f32::consts::PI;

use crate::bird::{BirdStats, BirdType};
use crate::components::{
    AiBird, Bird, BodyPart, Drafting, FlapState, Obstacle, Player, Wing,
};
use crate::state::SelectedBirdType;

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

    commands
        .spawn((
            PbrBundle {
                mesh: voxel.clone(),
                material: body_material,
                transform: Transform::from_xyz(0.0, 125.0, 0.0).with_scale(Vec3::new(
                    0.8 * player_scale * 3.0,
                    0.8 * player_scale * 3.0,
                    1.2 * player_scale * 3.0,
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
            // Head
            parent
                .spawn((
                    PbrBundle {
                        mesh: voxel.clone(),
                        material: head_material,
                        transform: Transform::from_xyz(0.0, 0.0, 0.7)
                            .with_scale(Vec3::new(0.75, 0.75, 0.75)),
                        ..default()
                    },
                    BodyPart::Head,
                ))
                .with_children(|head| {
                    // Beak
                    head.spawn(PbrBundle {
                        mesh: voxel.clone(),
                        material: beak_material,
                        transform: Transform::from_xyz(0.0, 0.0, 0.55)
                            .with_scale(Vec3::new(0.4, 0.27, 0.55)),
                        ..default()
                    });
                    // Eyes
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
                        transform: Transform::from_xyz(0.4, 0.2, 0.2).with_scale(Vec3::splat(0.27)),
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
                Wing {
                    is_left: true,
                    _base_x: -0.8,
                },
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
                Wing {
                    is_left: false,
                    _base_x: 0.8,
                },
                BodyPart::Wing,
            ));
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

        // Speed-based gravity & lift
        if flap_state.space_held {
            let (max_gravity, ramp_time) = match stats.bird_type {
                BirdType::Sparrow => (1000.0, 3.0),
                BirdType::Hawk => (1000.0, 4.5),
                BirdType::Eagle => (1000.0, 5.5),
                BirdType::Albatross => (1000.0, 7.0),
            };
            let gravity_factor = (flap_state.wings_closed_time / ramp_time).min(1.0);
            bird.velocity.y -= max_gravity * gravity_factor * dt;

            let fall_speed = (-bird.velocity.y).max(0.0);
            if fall_speed > 1.0 {
                let horizontal_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
                let size_multiplier = match stats.bird_type {
                    BirdType::Sparrow => 10.0,
                    BirdType::Hawk => 6.0,
                    BirdType::Eagle => 4.0,
                    BirdType::Albatross => 3.0,
                };
                let acceleration = fall_speed * size_multiplier;
                bird.velocity += horizontal_forward * acceleration * dt;

                if bird.velocity.length() > stats.max_dive_speed {
                    bird.velocity = bird.velocity.normalize_or_zero() * stats.max_dive_speed;
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
        .map(|(b, _, children)| {
            (
                b.grounded,
                b.is_walking,
                b.walk_timer,
                children.iter().copied().collect::<Vec<_>>(),
            )
        })
        .ok();

    let (is_grounded, is_walking, walk_timer, player_children) = match player_data {
        Some((g, w, t, c)) => (g, w, t, c),
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
                bird.velocity += local_up * thrust + local_forward * (thrust * 0.3);
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
                let fold_angle = 1.2;
                if is_walking {
                    let swing = (walk_timer).sin() * 0.4;
                    if wing.is_left {
                        transform.rotation = base_rotation
                            * Quat::from_rotation_z(fold_angle)
                            * Quat::from_rotation_y(swing);
                    } else {
                        transform.rotation = base_rotation
                            * Quat::from_rotation_z(-fold_angle)
                            * Quat::from_rotation_y(-swing);
                    }
                } else {
                    if wing.is_left {
                        transform.rotation = base_rotation * Quat::from_rotation_z(fold_angle);
                    } else {
                        transform.rotation = base_rotation * Quat::from_rotation_z(-fold_angle);
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
            } else if flap_state.space_held {
                if wing.is_left {
                    transform.rotation = base_rotation * Quat::from_rotation_z(wing_closed_angle);
                } else {
                    transform.rotation = base_rotation * Quat::from_rotation_z(-wing_closed_angle);
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
    player_query: Query<(&Bird, &Transform), With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera3d>, Without<Player>)>,
    time: Res<Time>,
    mut frame_count: Local<u32>,
) {
    let Ok((bird, player_transform)) = player_query.get_single() else {
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

    let camera_distance_behind = 45.0;
    let camera_height_above = 5.0;

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
