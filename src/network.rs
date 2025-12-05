use bevy::prelude::*;
use flight_shared::{ClientMessage, NetBirdType, PlayerState, ServerMessage};
use std::io::ErrorKind;
use std::net::UdpSocket;
use std::time::Instant;

use crate::bird::{BirdStats, BirdType};
use crate::components::{Bird, FlapState, Player, Wing};

/// Network connection state
#[derive(Resource)]
pub struct NetworkState {
    pub socket: UdpSocket,
    pub player_id: Option<u32>,
    pub connected: bool,
    pub last_send: Instant,
    pub server_addr: String,
}

impl NetworkState {
    pub fn new(server_addr: &str) -> Option<Self> {
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

    pub fn send(&self, msg: &ClientMessage) {
        let data = flight_shared::encode(msg);
        let _ = self.socket.send_to(&data, &self.server_addr);
    }
}

/// Remote player component
#[derive(Component)]
pub struct RemotePlayer {
    pub player_id: u32,
    pub last_update: Instant,
    pub bird_type: NetBirdType,
    pub wings_closed: bool,
    pub flap_timer: f32,
    pub grounded: bool,
    pub is_walking: bool,
}

/// Send join message on connect
pub fn network_connect(network: ResMut<NetworkState>) {
    let join_msg = ClientMessage::Join {
        name: "Player".to_string(),
    };
    network.send(&join_msg);
    println!("Sent join request to server");
}

/// Send player state to server
pub fn network_send_state(
    mut network: ResMut<NetworkState>,
    player_query: Query<(&Transform, &Bird, &BirdStats), With<Player>>,
    flap_state: Res<FlapState>,
    mut retry_timer: Local<f32>,
    time: Res<Time>,
) {
    // If not connected yet, periodically resend join request
    if network.player_id.is_none() {
        *retry_timer += time.delta_seconds();
        if *retry_timer > 2.0 {
            *retry_timer = 0.0;
            let join_msg = ClientMessage::Join {
                name: "Player".to_string(),
            };
            network.send(&join_msg);
        }
        return;
    }

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

/// Receive network messages
pub fn network_receive(
    mut commands: Commands,
    mut network: ResMut<NetworkState>,
    mut remote_players: Query<(Entity, &mut RemotePlayer, &mut Transform, &Children)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut buf = [0u8; 1024];

    loop {
        match network.socket.recv_from(&mut buf) {
            Ok((len, _)) => {
                let data = &buf[..len];
                match flight_shared::decode::<ServerMessage>(data) {
                    Ok(ServerMessage::Welcome { player_id }) => {
                        println!("Connected to server! Player ID: {}", player_id);
                        network.player_id = Some(player_id);
                        network.connected = true;
                    }
                    Ok(ServerMessage::PlayerState(state)) => {
                        if Some(state.player_id) == network.player_id {
                            continue;
                        }

                        let mut found = false;
                        let mut needs_respawn = None;
                        for (entity, mut remote, mut transform, _) in remote_players.iter_mut() {
                            if remote.player_id == state.player_id {
                                if remote.bird_type != state.bird_type {
                                    needs_respawn = Some((entity, state.clone()));
                                    found = true;
                                    break;
                                }

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
                                remote.wings_closed = state.wings_closed;
                                remote.flap_timer = state.flap_timer;
                                remote.grounded = state.grounded;
                                remote.is_walking = state.is_walking;
                                remote.last_update = Instant::now();
                                found = true;
                                break;
                            }
                        }

                        if let Some((entity, new_state)) = needs_respawn {
                            commands.entity(entity).despawn_recursive();
                            spawn_remote_player(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                new_state,
                            );
                        } else if !found {
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
                break;
            }
            Err(e) => {
                eprintln!("Network error: {}", e);
                break;
            }
        }
    }
}

/// Spawn a remote player entity
pub fn spawn_remote_player(
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

    let body_material = materials.add(StandardMaterial {
        base_color: color,
        fog_enabled: false,
        ..default()
    });
    let body_light = materials.add(StandardMaterial {
        base_color: Color::srgb(
            (rgba.red * 1.2).min(1.0),
            (rgba.green * 1.2).min(1.0),
            (rgba.blue * 1.2).min(1.0),
        ),
        fog_enabled: false,
        ..default()
    });
    let wing_material = materials.add(StandardMaterial {
        base_color: Color::srgb(rgba.red * 0.8, rgba.green * 0.8, rgba.blue * 0.8),
        fog_enabled: false,
        ..default()
    });
    let beak_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.95, 0.7, 0.2),
        fog_enabled: false,
        ..default()
    });
    let eye_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.1),
        fog_enabled: false,
        ..default()
    });
    let eye_white = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 1.0, 1.0),
        fog_enabled: false,
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

/// Remove players that haven't been updated
pub fn cleanup_stale_players(
    mut commands: Commands,
    remote_players: Query<(Entity, &RemotePlayer)>,
) {
    for (entity, remote) in remote_players.iter() {
        if remote.last_update.elapsed().as_secs() > 5 {
            println!("Removing stale player {}", remote.player_id);
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Animate remote player wings
pub fn remote_player_wing_animation(
    remote_players: Query<(&RemotePlayer, &Children)>,
    mut wing_query: Query<(&Wing, &mut Transform)>,
) {
    use std::f32::consts::PI;

    let flap_duration = 0.3;
    let wing_closed_angle = 1.2;

    for (remote, children) in remote_players.iter() {
        for &child in children.iter() {
            if let Ok((wing, mut transform)) = wing_query.get_mut(child) {
                let base_rotation = Quat::from_rotation_x(PI / 2.0);

                if remote.grounded {
                    let fold_angle = 1.2;
                    if wing.is_left {
                        transform.rotation = base_rotation * Quat::from_rotation_z(fold_angle);
                    } else {
                        transform.rotation = base_rotation * Quat::from_rotation_z(-fold_angle);
                    }
                } else if remote.flap_timer > 0.0 {
                    let progress = 1.0 - (remote.flap_timer / flap_duration);
                    let angle = (progress * PI).sin() * 0.8;

                    if wing.is_left {
                        transform.rotation = base_rotation * Quat::from_rotation_z(angle);
                    } else {
                        transform.rotation = base_rotation * Quat::from_rotation_z(-angle);
                    }
                } else if remote.wings_closed {
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
    }
}
