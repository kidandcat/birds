use bevy::prelude::*;

use crate::bird::{BirdStats, BirdType};
use crate::components::{
    BirdButton, DistanceText, DraftIndicator, Drafting, Goal, Player, SelectionUI,
};
use crate::state::{AppState, GameState, SelectedBirdType};

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
                background_color: Color::srgba(0.0, 0.0, 0.0, 0.7).into(),
                ..default()
            },
            SelectionUI,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn(
                TextBundle::from_section(
                    "Choose Your Bird",
                    TextStyle {
                        font_size: 48.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::bottom(Val::Px(40.0)),
                    ..default()
                }),
            );

            // Bird buttons container
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(20.0),
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
                        parent
                            .spawn((
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
                            ))
                            .with_children(|parent| {
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
            parent.spawn(
                TextBundle::from_section(
                    "Click to select",
                    TextStyle {
                        font_size: 20.0,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.6),
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::top(Val::Px(30.0)),
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
