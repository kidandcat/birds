use bevy::prelude::*;
use crate::bird::BirdType;

/// Audio events for game sounds
#[derive(Event)]
pub struct PlaySound {
    pub sound: SoundEffect,
}

#[derive(Clone, Copy, Debug)]
pub enum SoundEffect {
    WingFlap(BirdType),
    Collision,
    HuntingStrike,
    CaptureSuccess,
    ButtonClick,
    ButtonHover,
    ScorePoint,
    Takeoff,
    Landing,
    WindSoar,
}

/// Resource holding all audio handles
#[derive(Resource)]
pub struct GameAudio {
    // Wing flap sounds per bird type
    pub flap_sparrow: Handle<AudioSource>,
    pub flap_hawk: Handle<AudioSource>,
    pub flap_eagle: Handle<AudioSource>,
    pub flap_albatross: Handle<AudioSource>,
    // Game events
    pub collision: Handle<AudioSource>,
    pub hunting_strike: Handle<AudioSource>,
    pub capture_success: Handle<AudioSource>,
    pub takeoff: Handle<AudioSource>,
    pub landing: Handle<AudioSource>,
    pub wind_soar: Handle<AudioSource>,
    // UI sounds
    pub button_click: Handle<AudioSource>,
    pub button_hover: Handle<AudioSource>,
    pub score_point: Handle<AudioSource>,
    // Music
    pub background_music: Handle<AudioSource>,
}

/// Marker for background music entity
#[derive(Component)]
pub struct BackgroundMusic;

/// Marker for the wind soaring sound (looping)
#[derive(Component)]
pub struct WindSoarSound;

/// Load all audio assets
pub fn load_audio(mut commands: Commands, asset_server: Res<AssetServer>) {
    let audio = GameAudio {
        flap_sparrow: asset_server.load("sounds/flap_sparrow.ogg"),
        flap_hawk: asset_server.load("sounds/flap_hawk.ogg"),
        flap_eagle: asset_server.load("sounds/flap_eagle.ogg"),
        flap_albatross: asset_server.load("sounds/flap_albatross.ogg"),
        collision: asset_server.load("sounds/collision.ogg"),
        hunting_strike: asset_server.load("sounds/hunting_strike.ogg"),
        capture_success: asset_server.load("sounds/capture_success.ogg"),
        takeoff: asset_server.load("sounds/takeoff.ogg"),
        landing: asset_server.load("sounds/landing.ogg"),
        wind_soar: asset_server.load("sounds/wind_soar.ogg"),
        button_click: asset_server.load("sounds/button_click.ogg"),
        button_hover: asset_server.load("sounds/button_hover.ogg"),
        score_point: asset_server.load("sounds/score_point.ogg"),
        background_music: asset_server.load("music/background.ogg"),
    };
    commands.insert_resource(audio);
}

/// Start background music when entering playing state
pub fn start_background_music(
    mut commands: Commands,
    audio: Res<GameAudio>,
) {
    commands.spawn((
        AudioBundle {
            source: audio.background_music.clone(),
            settings: PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Loop,
                volume: bevy::audio::Volume::new(0.3),
                ..default()
            },
        },
        BackgroundMusic,
    ));
}

/// Stop background music when exiting playing state
pub fn stop_background_music(
    mut commands: Commands,
    music_query: Query<Entity, With<BackgroundMusic>>,
) {
    for entity in music_query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Play sound effects based on events
pub fn play_sound_effects(
    mut commands: Commands,
    mut events: EventReader<PlaySound>,
    audio: Option<Res<GameAudio>>,
) {
    let Some(audio) = audio else { return };

    for event in events.read() {
        let (source, volume) = match event.sound {
            SoundEffect::WingFlap(bird_type) => {
                let handle = match bird_type {
                    BirdType::Sparrow => audio.flap_sparrow.clone(),
                    BirdType::Hawk => audio.flap_hawk.clone(),
                    BirdType::Eagle => audio.flap_eagle.clone(),
                    BirdType::Albatross => audio.flap_albatross.clone(),
                };
                (handle, 0.5)
            }
            SoundEffect::Collision => (audio.collision.clone(), 0.8),
            SoundEffect::HuntingStrike => (audio.hunting_strike.clone(), 0.7),
            SoundEffect::CaptureSuccess => (audio.capture_success.clone(), 0.6),
            SoundEffect::ButtonClick => (audio.button_click.clone(), 0.4),
            SoundEffect::ButtonHover => (audio.button_hover.clone(), 0.2),
            SoundEffect::ScorePoint => (audio.score_point.clone(), 0.3),
            SoundEffect::Takeoff => (audio.takeoff.clone(), 0.5),
            SoundEffect::Landing => (audio.landing.clone(), 0.4),
            SoundEffect::WindSoar => (audio.wind_soar.clone(), 0.3),
        };

        commands.spawn(AudioBundle {
            source,
            settings: PlaybackSettings {
                mode: bevy::audio::PlaybackMode::Despawn,
                volume: bevy::audio::Volume::new(volume),
                ..default()
            },
        });
    }
}

/// Manage wind soaring sound for Albatross ability
pub fn wind_soar_sound(
    mut commands: Commands,
    audio: Option<Res<GameAudio>>,
    player_query: Query<&crate::bird::BirdStats, With<crate::components::Player>>,
    flap_state: Res<crate::components::FlapState>,
    wind_sound_query: Query<Entity, With<WindSoarSound>>,
    mut is_soaring: Local<bool>,
) {
    let Some(audio) = audio else { return };
    let Ok(stats) = player_query.get_single() else { return };

    let should_soar = stats.bird_type == BirdType::Albatross
        && flap_state.space_held
        && flap_state.wings_closed_time >= 0.5;

    if should_soar && !*is_soaring {
        // Start wind soar sound
        commands.spawn((
            AudioBundle {
                source: audio.wind_soar.clone(),
                settings: PlaybackSettings {
                    mode: bevy::audio::PlaybackMode::Loop,
                    volume: bevy::audio::Volume::new(0.4),
                    ..default()
                },
            },
            WindSoarSound,
        ));
        *is_soaring = true;
    } else if !should_soar && *is_soaring {
        // Stop wind soar sound
        for entity in wind_sound_query.iter() {
            commands.entity(entity).despawn();
        }
        *is_soaring = false;
    }
}

/// Cleanup wind soar sound when exiting playing state
pub fn cleanup_wind_soar(
    mut commands: Commands,
    wind_sound_query: Query<Entity, With<WindSoarSound>>,
) {
    for entity in wind_sound_query.iter() {
        commands.entity(entity).despawn();
    }
}
