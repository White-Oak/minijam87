use std::process;

use bevy::{
    core::FixedTimestep,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    log,
    prelude::*,
    window::WindowResized,
};

use crate::{MainCamera, daytime::Daytime, field::{CoffeeShops, Map, NextRingTimer, SIZE}};

struct FpsCounter;
struct NextRingCounter;
struct MoneyTextCounter;
struct TimeTextCounter;
struct CoffeeShopsCounter;

pub struct Money(u32);
pub struct ChangeMoneyEvent(pub i32);

impl Default for Money {
    fn default() -> Self {
        Self(0)
    }
}

pub struct UpgradeTileEvent;

pub struct GeneratedNextRing(pub u32);

pub struct UiPlugin;

fn fps_change_text(diagnostics: Res<Diagnostics>, mut query: Query<&mut Text, With<FpsCounter>>) {
    let fps = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS).unwrap();
    if let Some(v) = fps.average() {
        for mut text in query.iter_mut() {
            text.sections[0].value = format!("FPS: {}", v as i64);
        }
    }
}

fn money_change_text(money: Res<Money>, mut query: Query<&mut Text, With<MoneyTextCounter>>) {
    if money.is_changed() {
        for mut text in query.iter_mut() {
            text.sections[0].value = format!("Money: {}", money.0 as i64);
        }
    }
}

fn shops_change_text(shops: Res<CoffeeShops>, mut query: Query<&mut Text, With<CoffeeShopsCounter>>) {
    if shops.is_changed() {
        for mut text in query.iter_mut() {
            text.sections[0].value = format!("Cofee shops: {}/{}", shops.0, shops.1);
        }
    }
}


fn next_ring_change_text(
    timer: Res<NextRingTimer>,
    mut query: Query<&mut Text, With<NextRingCounter>>,
) {
    let left = timer.0.percent_left() * 100.;
    for mut text in query.iter_mut() {
        text.sections[0].value = format!("Until next ring: {}%", left as i64);
    }
}

fn daytime_change_text(daytime: Res<Daytime>, mut query: Query<&mut Text, With<TimeTextCounter>>) {
    for mut text in query.iter_mut() {
        text.sections[0].value = daytime.to_string();
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
    let text = Text::with_section(
        "Until next ring: ".to_string(),
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
    let shops_text = Text::with_section(
        "Coffee shops: 1/1".to_string(),
        TextStyle {
            font: font_handle,
            font_size: 30.0,
            color: Color::BLACK,
        },
        TextAlignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Left,
        },
    );
    let card_material = color_materials.add(Color::rgb_u8(230, 245, 255).into());
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
                ec.spawn_bundle(TextBundle {
                    text: shops_text,
                    ..Default::default()
                })
                .insert(CoffeeShopsCounter);
            });

        });
}

fn change_money(mut money: ResMut<Money>, mut events: EventReader<ChangeMoneyEvent>) {
    for &ChangeMoneyEvent(delta) in events.iter() {
        if delta.is_negative() {
            let delta = delta.abs() as u32;
            let res = money.0.checked_sub(delta);
            if let Some(res) = res {
                money.0 = res;
            } else {
                log::info!("Game over!");
                process::exit(0);
            }
        } else {
            money.0 += delta as u32;
        }
    }
}

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut events: EventWriter<UpgradeTileEvent>,
) {
    if keys.just_pressed(KeyCode::U) || mouse.just_pressed(MouseButton::Left) {
        events.send(UpgradeTileEvent);
    }
}

fn calc_scale_vec(rings: u32, wnd_height: f32) -> Vec3 {
    let total_height = SIZE * 3_f32.sqrt() * (1 + rings * 2) as f32;
    let scale = total_height / wnd_height;
    let scale = scale.max(1.);
    Vec3::new(scale, scale, 1.)
}

fn set_scale(query: &mut Query<&mut Transform, With<MainCamera>>, windows: &Windows, rings: u32) {
    let wnd = windows.get_primary().unwrap();
    for mut proj in query.iter_mut() {
        // println!("{:?}", proj.scale);
        proj.scale = calc_scale_vec(rings, wnd.height());
    }
}

fn change_camera_scale(
    mut query: Query<&mut Transform, With<MainCamera>>,
    mut events: EventReader<GeneratedNextRing>,
    windows: Res<Windows>,
) {
    for &GeneratedNextRing(rings) in events.iter() {
        set_scale(&mut query, &windows, rings);
    }
}

fn change_camera_scale_from_resize(
    mut query: Query<&mut Transform, With<MainCamera>>,
    mut events: EventReader<WindowResized>,
    windows: Res<Windows>,
    map: Res<Map>,
) {
    for _ in events.iter() {
        let rings = map.generated_rings;
        set_scale(&mut query, &windows, rings);
    }
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::steps_per_second(16.))
                    .with_system(fps_change_text.system())
                    .with_system(next_ring_change_text.system())
                    .with_system(money_change_text.system())
                    .with_system(daytime_change_text.system())
                    .with_system(shops_change_text.system()),
            )
            .add_system(change_camera_scale.system())
            .add_system(change_camera_scale_from_resize.system())
            .add_system(change_money.system())
            .add_system(keyboard_input.system())
            .add_event::<ChangeMoneyEvent>()
            .add_event::<UpgradeTileEvent>()
            .add_event::<GeneratedNextRing>()
            .init_resource::<Money>();
    }
}
