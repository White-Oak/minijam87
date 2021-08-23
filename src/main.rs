mod daytime;
mod field;
mod overwait_particles;
mod ui;
mod upgrade_particles;
mod utils;
mod workers;

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, text::TextPlugin};
use bevy_prototype_lyon::prelude::*;
use daytime::DaytimePlugin;
use field::FieldPlugin;
use overwait_particles::OverwaitParticlesPlugin;
use ui::UiPlugin;
use upgrade_particles::UpgradeParticlesPlugin;
use workers::WorkerPlugin;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 8 })
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .add_plugin(TextPlugin)
        .add_startup_system(setup.system())
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_plugin(UiPlugin)
        .add_plugin(FieldPlugin)
        .add_plugin(DaytimePlugin)
        .add_plugin(WorkerPlugin)
        .add_plugin(UpgradeParticlesPlugin)
        .add_plugin(OverwaitParticlesPlugin)
        .run();
}

pub struct MainCamera;

fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);
}
