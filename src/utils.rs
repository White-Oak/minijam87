use bevy::core::Time;

const DEV_FRAME_TIME: f32 = 1. / 60.;

pub fn time_k(time: &Time) -> f32 {
    time.delta_seconds() / DEV_FRAME_TIME
}
