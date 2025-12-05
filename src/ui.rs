use bevy::prelude::*;

use crate::bird::{BirdStats, BirdType};
use crate::components::{
    BirdButton, DistanceText, DraftIndicator, Drafting, Goal, Player, PlayerScore, ScoreText,
    SelectionUI, ServerStatusIndicator,
};
use crate::network::NetworkState;
use crate::state::{AppState, GameState, SelectedBirdType};

/// Helper to create a stat bar
fn spawn_stat_bar(parent: &mut ChildBuilder, label: &str, value: f32, max_value: f32, color: Color) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            // Label
            row.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font_size: 11.0,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                    ..default()
                },
            ));
            // Bar background
            row.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(6.0),
                    margin: UiRect::top(Val::Px(2.0)),
                    ..default()
                },
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.4).into(),
                ..default()
            })
            .with_children(|bar_bg| {
                // Bar fill
                let fill_percent = (value / max_value * 100.0).min(100.0);
                bar_bg.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(fill_percent),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    background_color: color.into(),
                    ..default()
                });
            });
        });
}

/// Setup bird selection UI
pub fn setup_selection_ui(mut commands: Commands) {
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
                background_color: Color::srgba(0.05, 0.08, 0.15, 0.92).into(),
                ..default()
            },
            SelectionUI,
        ))
        .with_children(|parent| {
            // Title with shadow effect
            parent
                .spawn(NodeBundle {
                    style: Style {
                        margin: UiRect::bottom(Val::Px(10.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|title_container| {
                    title_container.spawn(
                        TextBundle::from_section(
                            "FLIGHT QUEST",
                            TextStyle {
                                font_size: 56.0,
                                color: Color::srgb(0.95, 0.85, 0.6),
                                ..default()
                            },
                        ),
                    );
                });

            // Subtitle
            parent.spawn(
                TextBundle::from_section(
                    "Choose Your Bird",
                    TextStyle {
                        font_size: 28.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.8),
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::bottom(Val::Px(35.0)),
                    ..default()
                }),
            );

            // Bird cards container
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(25.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    for bird_type in [
                        BirdType::Sparrow,
                        BirdType::Hawk,
                        BirdType::Eagle,
                        BirdType::Albatross,
                    ] {
                        let stats = BirdStats::for_type(bird_type);
                        let base_color = bird_type.color();
                        let rgba = base_color.to_srgba();

                        // Card container with border effect
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    padding: UiRect::all(Val::Px(3.0)),
                                    ..default()
                                },
                                background_color: Color::srgba(
                                    rgba.red * 0.5,
                                    rgba.green * 0.5,
                                    rgba.blue * 0.5,
                                    0.8,
                                )
                                .into(),
                                ..default()
                            })
                            .with_children(|border| {
                                // Main card button
                                border
                                    .spawn((
                                        ButtonBundle {
                                            style: Style {
                                                width: Val::Px(180.0),
                                                height: Val::Px(260.0),
                                                flex_direction: FlexDirection::Column,
                                                align_items: AlignItems::Center,
                                                padding: UiRect::all(Val::Px(12.0)),
                                                ..default()
                                            },
                                            background_color: Color::srgba(0.12, 0.15, 0.22, 0.95)
                                                .into(),
                                            ..default()
                                        },
                                        BirdButton(bird_type),
                                    ))
                                    .with_children(|card| {
                                        // Bird icon/preview area
                                        card.spawn(NodeBundle {
                                            style: Style {
                                                width: Val::Px(80.0),
                                                height: Val::Px(60.0),
                                                margin: UiRect::bottom(Val::Px(8.0)),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            background_color: Color::srgba(
                                                rgba.red * 0.3,
                                                rgba.green * 0.3,
                                                rgba.blue * 0.3,
                                                0.5,
                                            )
                                            .into(),
                                            ..default()
                                        })
                                        .with_children(|preview| {
                                            // Simple bird silhouette using nested boxes
                                            preview
                                                .spawn(NodeBundle {
                                                    style: Style {
                                                        width: Val::Px(40.0 * bird_type.scale()),
                                                        height: Val::Px(20.0 * bird_type.scale()),
                                                        justify_content: JustifyContent::Center,
                                                        align_items: AlignItems::Center,
                                                        ..default()
                                                    },
                                                    background_color: base_color.into(),
                                                    ..default()
                                                })
                                                .with_children(|body| {
                                                    // Wings
                                                    body.spawn(NodeBundle {
                                                        style: Style {
                                                            width: Val::Px(60.0 * bird_type.scale()),
                                                            height: Val::Px(8.0 * bird_type.scale()),
                                                            position_type: PositionType::Absolute,
                                                            ..default()
                                                        },
                                                        background_color: Color::srgb(
                                                            rgba.red * 0.7,
                                                            rgba.green * 0.7,
                                                            rgba.blue * 0.7,
                                                        )
                                                        .into(),
                                                        ..default()
                                                    });
                                                });
                                        });

                                        // Bird name
                                        card.spawn(TextBundle::from_section(
                                            bird_type.name(),
                                            TextStyle {
                                                font_size: 22.0,
                                                color: base_color,
                                                ..default()
                                            },
                                        ));

                                        // Size indicator
                                        card.spawn(
                                            TextBundle::from_section(
                                                format!("Size: {:.1}x", bird_type.scale()),
                                                TextStyle {
                                                    font_size: 12.0,
                                                    color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                                                    ..default()
                                                },
                                            )
                                            .with_style(Style {
                                                margin: UiRect::bottom(Val::Px(10.0)),
                                                ..default()
                                            }),
                                        );

                                        // Stats container
                                        card.spawn(NodeBundle {
                                            style: Style {
                                                width: Val::Percent(100.0),
                                                flex_direction: FlexDirection::Column,
                                                ..default()
                                            },
                                            ..default()
                                        })
                                        .with_children(|stats_container| {
                                            // Speed stat
                                            spawn_stat_bar(
                                                stats_container,
                                                "Speed",
                                                stats.perfect_glide_speed,
                                                35.0,
                                                Color::srgb(0.3, 0.8, 0.4),
                                            );
                                            // Agility stat
                                            spawn_stat_bar(
                                                stats_container,
                                                "Agility",
                                                stats.turn_rate,
                                                1.5,
                                                Color::srgb(0.4, 0.6, 0.9),
                                            );
                                            // Glide stat
                                            spawn_stat_bar(
                                                stats_container,
                                                "Glide",
                                                2.0 - stats.glide_efficiency,
                                                2.0,
                                                Color::srgb(0.9, 0.7, 0.3),
                                            );
                                            // Power stat
                                            spawn_stat_bar(
                                                stats_container,
                                                "Power",
                                                stats.flap_thrust,
                                                25.0,
                                                Color::srgb(0.9, 0.4, 0.4),
                                            );
                                        });
                                    });
                            });
                    }
                });

            // Instructions
            parent.spawn(
                TextBundle::from_section(
                    "Click a bird to begin your flight",
                    TextStyle {
                        font_size: 18.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.5),
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::top(Val::Px(35.0)),
                    ..default()
                }),
            );

            // Controls hint
            parent.spawn(
                TextBundle::from_section(
                    "Controls: Mouse to steer • Space to flap/dive • WASD to walk when landed",
                    TextStyle {
                        font_size: 14.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.35),
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::top(Val::Px(15.0)),
                    ..default()
                }),
            );
        });
}

/// Handle selection button interactions
pub fn selection_button_system(
    mut commands: Commands,
    mut interaction_query: Query<
        (&Interaction, &BirdButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
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
                )
                .into();
            }
            Interaction::None => {
                *bg_color = base_color.into();
            }
        }
    }
}

/// Cleanup selection UI on state exit
pub fn cleanup_selection_ui(mut commands: Commands, query: Query<Entity, With<SelectionUI>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

/// Update UI elements
pub fn update_ui(
    drafting_query: Query<&Drafting, With<Player>>,
    mut draft_text_query: Query<&mut Text, With<DraftIndicator>>,
    mut distance_text_query: Query<&mut Text, (With<DistanceText>, Without<DraftIndicator>)>,
    player_query: Query<&Transform, With<Player>>,
    goal_query: Query<&Transform, With<Goal>>,
) {
    let Ok(drafting) = drafting_query.get_single() else {
        return;
    };
    let Ok(mut draft_text) = draft_text_query.get_single_mut() else {
        return;
    };
    let Ok(mut distance_text) = distance_text_query.get_single_mut() else {
        return;
    };

    if drafting.is_drafting {
        draft_text.sections[0].value = "DRAFTING!".to_string();
    } else {
        draft_text.sections[0].value = "".to_string();
    }

    if let Ok(player_transform) = player_query.get_single() {
        if let Ok(goal_transform) = goal_query.get_single() {
            let distance = (goal_transform.translation - player_transform.translation).length();
            distance_text.sections[0].value = format!("Distance to home: {:.0}m", distance);
        }
    }
}

/// Check if goal reached
pub fn check_goal(
    player_query: Query<&Transform, With<Player>>,
    goal_query: Query<&Transform, With<Goal>>,
    mut game_state: ResMut<GameState>,
) {
    if game_state.game_over {
        return;
    }

    let Ok(player_transform) = player_query.get_single() else {
        return;
    };
    let Ok(goal_transform) = goal_query.get_single() else {
        return;
    };

    let distance = (goal_transform.translation - player_transform.translation).length();

    if distance < 5.0 {
        println!("You made it home!");
        game_state.game_over = true;
        game_state.won = true;
    }
}

/// Update server status indicator
pub fn update_server_status(
    network: Option<Res<NetworkState>>,
    mut status_query: Query<&mut Text, With<ServerStatusIndicator>>,
) {
    let Ok(mut text) = status_query.get_single_mut() else {
        return;
    };

    match network {
        Some(net) => {
            if net.connected {
                text.sections[0].value = format!("Online: {}", net.server_addr);
                text.sections[0].style.color = Color::srgba(0.3, 0.9, 0.4, 0.9);
            } else {
                text.sections[0].value = format!("Connecting to {}...", net.server_addr);
                text.sections[0].style.color = Color::srgba(0.9, 0.8, 0.3, 0.9);
            }
        }
        None => {
            text.sections[0].value = "Offline".to_string();
            text.sections[0].style.color = Color::srgba(0.5, 0.5, 0.5, 0.7);
        }
    }
}

/// Update score display
pub fn update_score(
    score: Res<PlayerScore>,
    mut score_query: Query<&mut Text, With<ScoreText>>,
    player_query: Query<&BirdStats, With<Player>>,
) {
    let Ok(mut text) = score_query.get_single_mut() else {
        return;
    };

    let bird_type = player_query
        .get_single()
        .map(|s| s.bird_type)
        .unwrap_or(BirdType::Hawk);

    let role = if bird_type == BirdType::Sparrow {
        "Survivor"
    } else {
        "Hunter"
    };

    text.sections[0].value = format!("Score: {} ({})", score.points, role);
}

/// Setup score UI (called on enter Playing)
pub fn setup_score_ui(mut commands: Commands) {
    commands.spawn((
        TextBundle::from_section(
            "Score: 0",
            TextStyle {
                font_size: 28.0,
                color: Color::srgb(1.0, 0.9, 0.3),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(20.0),
            ..default()
        }),
        ScoreText,
    ));
}

/// Cleanup score UI and reset score
pub fn cleanup_score_ui(
    mut commands: Commands,
    score_query: Query<Entity, With<ScoreText>>,
    mut score: ResMut<PlayerScore>,
) {
    for entity in score_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
    // Reset score when leaving
    score.points = 0;
    score.passive_timer = 0.0;
}
