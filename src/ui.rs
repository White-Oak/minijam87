use std::time::Duration;

use bevy::{
    core::FixedTimestep,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

use crate::field::{NextRingTimer, NEXT_RING_TIMER_SECS};

struct FpsCounter;
struct NextRingCounter;
struct MoneyTextCounter;
struct TimeTextCounter;
pub struct UiPlugin;

fn fps_change_text(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsCounter>>) {
    let fps = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS).unwrap();
    if let Some(v) = fps.average() {
        for mut text in query.iter_mut() {
            text.sections[0].value = format!("FPS: {}", v as i64);
        }
    }
}

fn next_ring_change_text(
    timer: Res<NextRingTimer>,
    mut query: Query<&mut Text, With<NextRingCounter>>,
) {
    let left = timer.0.percent_left() * NEXT_RING_TIMER_SECS;
    for mut text in query.iter_mut() {
        text.sections[0].value = format!("Until next ring: {}", left as i64);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
) {
    let font_handle = asset_server.load("FiraSans-Bold.ttf");
    let text = Text::with_section(
        "FPS: ".to_string(),
        TextStyle {
            font: font_handle.clone(),
            font_size: 30.0,
            color: Color::BLACK,
        },
        TextAlignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Left,
        },
    );

    let material = color_materials.add(Color::NONE.into());
    let mut ui_bundle = commands.spawn_bundle(UiCameraBundle::default());
    let ui_cmds = ui_bundle // root node
        .commands();
    ui_cmds
        .spawn_bundle(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    right: Val::Px(10.),
                    top: Val::Px(10.),
                    ..Default::default()
                },
                ..Default::default()
            },
            material: material.clone(),
            ..Default::default()
        })
        .with_children(|ec| {
            ec.spawn_bundle(TextBundle {
                text,
                ..Default::default()
            })
            .insert(FpsCounter);
        });

    // General info section
    let money_text = Text::with_section(
        "Money: 0".to_string(),
        TextStyle {
            font: font_handle.clone(),
            font_size: 30.0,
            color: Color::BLACK,
        },
        TextAlignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Left,
        },
    );
    let time_text = Text::with_section(
        "Time: 00:00".to_string(),
        TextStyle {
            font: font_handle.clone(),
            font_size: 30.0,
            color: Color::BLACK,
        },
        TextAlignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Left,
        },
    );
    let card_material = color_materials.add(Color::rgb_u8(230, 245, 255).into());
    let text = Text::with_section(
        "Until next ring: ".to_string(),
        TextStyle {
            font: font_handle,
            font_size: 40.0,
            color: Color::BLACK,
        },
        TextAlignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Left,
        },
    );
    ui_cmds
        .spawn_bundle(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                position: Rect {
                    left: Val::Px(10.),
                    top: Val::Px(10.),
                    ..Default::default()
                },
                flex_direction: FlexDirection::ColumnReverse,
                size: Size {
                    width: Val::Px(310.),
                    height: Val::Undefined,
                },
                max_size: Size {
                    width: Val::Px(310.),
                    height: Val::Undefined,
                },
                // border: Rect {
                //     left: Val::Px(10.),
                //     top: Val::Px(10.),
                //     bottom: Val::Px(10.),
                //     right: Val::Px(10.),
                // },
                align_items: AlignItems::FlexStart,
                ..Default::default()
            },
            material,
            ..Default::default()
        })
        .with_children(|ec| {
            ec.spawn_bundle(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::ColumnReverse,
                    size: Size {
                        width: Val::Px(310.),
                        height: Val::Undefined,
                    },
                    max_size: Size {
                        width: Val::Px(310.),
                        height: Val::Undefined,
                    },
                    padding: Rect {
                        left: Val::Px(10.),
                        top: Val::Px(10.),
                        bottom: Val::Px(10.),
                        right: Val::Px(10.),
                    },
                    align_items: AlignItems::FlexStart,
                    ..Default::default()
                },
                material: card_material.clone(),
                ..Default::default()
            })
            .with_children(|ec| {
                ec.spawn_bundle(TextBundle {
                    text: text.clone(),
                    ..Default::default()
                })
                .insert(NextRingCounter);
                ec.spawn_bundle(TextBundle {
                    text: money_text,
                    ..Default::default()
                })
                .insert(MoneyTextCounter);
                ec.spawn_bundle(TextBundle {
                    text: time_text,
                    ..Default::default()
                })
                .insert(TimeTextCounter);
            });

            ec.spawn_bundle(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::ColumnReverse,
                    size: Size {
                        width: Val::Px(310.),
                        height: Val::Undefined,
                    },
                    max_size: Size {
                        width: Val::Px(310.),
                        height: Val::Undefined,
                    },
                    padding: Rect {
                        left: Val::Px(10.),
                        top: Val::Px(10.),
                        bottom: Val::Px(10.),
                        right: Val::Px(10.),
                    },
                    margin: Rect {
                        top: Val::Px(20.),
                        ..Default::default()
                    },
                    align_items: AlignItems::FlexStart,
                    ..Default::default()
                },
                material: card_material,
                ..Default::default()
            })
            .with_children(|ec| {
                ec.spawn_bundle(TextBundle {
                    text,
                    ..Default::default()
                })
                .insert(NextRingCounter);
            });
        });
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system()).add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(1. / 2.))
                .with_system(fps_change_text.system())
                .with_system(next_ring_change_text.system()),
        );
    }
}
