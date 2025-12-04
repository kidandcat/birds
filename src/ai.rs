use bevy::prelude::*;
use rand::Rng;
use std::f32::consts::PI;

use crate::components::{AiBird, Bird};

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
