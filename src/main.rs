mod ui;
mod workers;
mod field;

use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, text::TextPlugin};
use bevy_prototype_lyon::prelude::*;
use field::FieldPlugin;
use ui::UiPlugin;


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
        .run();
}

pub struct MainCamera;

fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);
}
