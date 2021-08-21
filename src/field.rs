use bevy::{prelude::*, utils::HashMap};
use bevy_prototype_lyon::prelude::*;
use hex2d::{Coordinate, Direction, Spacing, Spin};
use rand::{
    distributions::WeightedIndex,
    prelude::{Distribution, SliceRandom},
};

use crate::MainCamera;

const MAX_ACTIVE_COUNTER: f32 = 120.0;

const NEIGHBOURS_WEIGHTS: [[(State, u8); 2]; 3] = [
    [(State::Inactive, 80), (State::Obstacle, 20)],
    [(State::Inactive, 60), (State::Obstacle, 40)],
    [(State::Inactive, 25), (State::Obstacle, 75)],
];

const SIZE: f32 = 100.;

pub const NEXT_RING_TIMER_SECS: f32 = 120.;

#[derive(Debug, Clone, Copy)]
enum State {
    Inactive,
    Active(i32),
    BreakShop,
    Obstacle,
}

impl State {
    fn color(&self) -> Color {
        match self {
            State::Inactive => Color::DARK_GRAY,
            State::Active(x) if *x > 0 => Color::rgb_u8(82, 151, 255),
            State::Active(x) if *x <= 0 => Color::RED,
            State::BreakShop => Color::rgb_u8(0, 112, 74),
            State::Obstacle => Color::BLACK,
            State::Active(_) => unreachable!(""),
        }
    }

    fn absent_color() -> Color {
        State::Inactive.color()
    }

    fn is_obstacle(&self) -> bool {
        matches!(self, State::Obstacle)
    }
}

pub struct NextRingTimer(pub Timer);

impl Default for NextRingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(NEXT_RING_TIMER_SECS as f32, true))
    }
}

struct SelectableTile;

struct GeneratedRings(i32);

impl Default for GeneratedRings {
    fn default() -> Self {
        Self(1)
    }
}

struct Map {
    tiles: HashMap<Coordinate, State>,
    generated_rings: i32,
}

fn build_hex_shape() -> shapes::RegularPolygon {
    shapes::RegularPolygon {
        sides: 6,
        feature: shapes::RegularPolygonFeature::Radius(100.0),
        ..shapes::RegularPolygon::default()
    }
}

fn setup(mut commands: Commands, asset_server: ResMut<AssetServer>, map: Res<Map>) {
    let font_handle = asset_server.load("FiraSans-Bold.ttf");
    for (c, tile) in map.tiles.iter() {
        let (x, y) = c.to_pixel(Spacing::FlatTop(SIZE));
        let (x_c, y_c) = (c.x, c.y);
        let text = Text::with_section(
            format!("{}, {}", x_c, y_c),
            TextStyle {
                font: font_handle.clone(),
                font_size: 60.0,
                color: Color::BLACK,
            },
            TextAlignment {
                vertical: VerticalAlign::Center,
                horizontal: HorizontalAlign::Center,
            },
        );
        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &build_hex_shape(),
                ShapeColors::outlined(tile.color(), Color::BLACK),
                DrawMode::Outlined {
                    fill_options: FillOptions::default(),
                    outline_options: StrokeOptions::default().with_line_width(10.0),
                },
                Transform::from_xyz(x, y, 0.),
            ))
            .with_children(|ec| {
                ec.spawn_bundle(Text2dBundle {
                    text,
                    transform: Transform::from_xyz(0., 0., 0.1),
                    ..Text2dBundle::default()
                });
            })
            .insert(*c);
    }
}

fn generate_next_ring(
    mut commands: Commands,
    mut map: ResMut<Map>,
    mut timer: ResMut<NextRingTimer>,
    time: Res<Time>,
    asset_server: ResMut<AssetServer>,
) {
    if !timer.0.tick(time.delta()).finished() {
        return;
    }
    let font_handle = asset_server.load("FiraSans-Bold.ttf");
    map.generated_rings += 1;
    let next_ring = Coordinate::new(0, 0).ring_iter(map.generated_rings, Spin::CW(Direction::XY));
    let mut next_tiles = vec![];
    for c in next_ring {
        let obstacles = c
            .neighbors()
            .iter()
            .filter_map(|c| map.tiles.get(c))
            .filter(|tile| tile.is_obstacle())
            .count();
        let weights = NEIGHBOURS_WEIGHTS[obstacles];
        let distr = WeightedIndex::new(weights.iter().map(|item| item.1)).unwrap();
        let mut rng = rand::thread_rng();
        let tile = weights[distr.sample(&mut rng)].0;
        next_tiles.push((c, tile));
    }
    for (c, tile) in next_tiles {
        map.tiles.insert(c, tile);
        let (x, y) = c.to_pixel(Spacing::FlatTop(SIZE));
        let (x_c, y_c) = (c.x, c.y);
        let text = Text::with_section(
            format!("{}, {}", x_c, y_c),
            TextStyle {
                font: font_handle.clone(),
                font_size: 60.0,
                color: Color::BLACK,
            },
            TextAlignment {
                vertical: VerticalAlign::Center,
                horizontal: HorizontalAlign::Center,
            },
        );
        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &build_hex_shape(),
                ShapeColors::outlined(tile.color(), Color::BLACK),
                DrawMode::Outlined {
                    fill_options: FillOptions::default(),
                    outline_options: StrokeOptions::default().with_line_width(10.0),
                },
                Transform::from_xyz(x, y, 0.),
            ))
            .with_children(|ec| {
                ec.spawn_bundle(Text2dBundle {
                    text,
                    transform: Transform::from_xyz(0., 0., 0.1),
                    ..Text2dBundle::default()
                });
            })
            .insert(c);
    }
}

impl Default for Map {
    fn default() -> Self {
        let mut tiles = HashMap::default();
        let start = Coordinate::new(0, 0);
        tiles.insert(start, State::Active(120));
        let neigh = start.neighbors();
        for n in neigh {
            tiles.insert(n, State::Inactive);
        }

        let mut rng = rand::thread_rng();
        let mut chosen = neigh.choose_multiple(&mut rng, 3);
        tiles.insert(*chosen.next().unwrap(), State::BreakShop);
        for n in chosen {
            tiles.insert(*n, State::Obstacle);
        }
        Self {
            tiles,
            generated_rings: 1,
        }
    }
}

struct SelectedHex {
    entity: Entity,
    coordinate: Coordinate,
}

fn select_hex(
    mut commands: Commands,
    windows: Res<Windows>,
    tiles: Query<(Entity, &Coordinate)>,
    mut selected_hex: ResMut<Option<SelectedHex>>,
    q_camera: Query<&Transform, With<MainCamera>>,
) {
    let wnd = windows.get_primary().unwrap();
    let pos = if let Some(position) = wnd.cursor_position() {
        position
    } else {
        return;
    };

    // get the size of the window
    let size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
    // the default orthographic projection is in pixels from the center;
    // just undo the translation
    let p = pos - size / 2.0;
    // assuming there is exactly one main camera entity, so this is OK
    let camera_transform = q_camera.single().unwrap();

    // apply the camera transform
    let pos_wld = camera_transform.compute_matrix() * p.extend(0.0).extend(1.0);
    let coordinate = Coordinate::<i32>::from_pixel(pos_wld.x, pos_wld.y, Spacing::FlatTop(SIZE));
    if let Some(selected_hex) = selected_hex.as_ref() {
        if selected_hex.coordinate == coordinate {
            // don't deselect or select
            return;
        }
        commands.entity(selected_hex.entity).despawn();
    }
    let any_selected = tiles.iter().any(|(_, coord)| *coord == coordinate);
    if any_selected {
        let (x, y) = coordinate.to_pixel(Spacing::FlatTop(100.0));
        let new_entity = commands
            .spawn_bundle(GeometryBuilder::build_as(
                &build_hex_shape(),
                ShapeColors::new(Color::RED),
                DrawMode::Stroke(StrokeOptions::default().with_line_width(5.0)),
                Transform::from_xyz(x, y, 0.5),
            ))
            .insert(coordinate)
            .id();
        let _ = selected_hex.insert(SelectedHex {
            entity: new_entity,
            coordinate,
        });
    }
}

pub struct FieldPlugin;
impl Plugin for FieldPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .init_resource::<Option<SelectedHex>>()
            .init_resource::<Map>()
            .init_resource::<NextRingTimer>()
            .add_system(generate_next_ring.system())
            .add_system(select_hex.system());
    }
}
