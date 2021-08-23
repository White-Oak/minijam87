use bevy::{log, prelude::*, utils::HashMap};
use bevy_prototype_lyon::prelude::*;
use hex2d::{Coordinate, Direction, Spacing, Spin};
use hex2d_dpcext::algo::bfs::Traverser;
use rand::{
    distributions::WeightedIndex,
    prelude::{Distribution, SliceRandom},
    thread_rng, Rng,
};

use crate::{MainCamera, daytime::TickEvent, ui::{ChangeMoneyEvent, GeneratedNextRing, UpgradeTileEvent}, workers::{ReturningWorker, SpawnWorkerEvent, WaitingWorker, Worker}};

const NEIGHBOURS_WEIGHTS: [[(State, u8); 3]; 3] = [
    [
        (State::Inactive, 45),
        (State::Active, 35),
        (State::Obstacle, 20),
    ],
    [
        (State::Inactive, 50),
        (State::Active, 10),
        (State::Obstacle, 40),
    ],
    [
        (State::Inactive, 25),
        (State::Active, 0),
        (State::Obstacle, 75),
    ],
];
pub const SIZE: f32 = 100.;

pub const START_RING_TIMER_SECS: f32 = 10.;
pub const NEXT_RING_TIMER_SECS: f32 = 60.;

pub const DEBUG_MODE: bool = false;

const BASE_CHANCE_TO_SPAWN_WORKER: u32 = 1;
const CHANCE_INCREASE_PER_TICK: u32 = 1;
const HUNDRED_PERCENT_CHANCE: u32 = 200;

const REWARD_FOR_COFFEE: i32 = 2;
const WAIT_TICKS_AFTER_SERVING: u32 = 6;

#[derive(Debug, Clone, Copy)]
pub enum State {
    Inactive,
    Active,
    BreakShop,
    Obstacle,
}

impl State {
    fn color(&self) -> Color {
        match self {
            State::Inactive => Color::DARK_GRAY,
            State::Active => Color::rgb_u8(82, 151, 255),
            State::BreakShop => Color::rgb_u8(0, 112, 74),
            State::Obstacle => Color::BLACK,
        }
    }

    fn is_obstacle(&self) -> bool {
        // matches!(self, State::Obstacle | State::Active)
        matches!(self, State::Obstacle)
    }

    fn is_passable(&self) -> bool {
        !self.is_obstacle()
    }

    fn is_coffee(&self) -> bool {
        matches!(self, State::BreakShop)
    }

    fn is_upgradeable(&self) -> bool {
        matches!(self, State::Inactive)
    }
}

pub struct NextRingTimer(pub Timer);

impl Default for NextRingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(START_RING_TIMER_SECS as f32, false))
    }
}

struct SelectableTile;
struct OfficeTile {
    ticks_wo_worker: u32,
}
struct CoffeeTile {
    waiting_ticks: u32,
}

struct GeneratedRings(i32);

impl Default for GeneratedRings {
    fn default() -> Self {
        Self(1)
    }
}

pub struct Map {
    tiles: HashMap<Coordinate, State>,
    pub generated_rings: u32,
}

fn build_hex_shape() -> shapes::RegularPolygon {
    shapes::RegularPolygon {
        sides: 6,
        feature: shapes::RegularPolygonFeature::Radius(100.0),
        ..shapes::RegularPolygon::default()
    }
}

fn spawn_tile(
    commands: &mut Commands,
    font_handle: &Handle<Font>,
    c: Coordinate,
    tile: State,
) -> Entity {
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
    let mut builder = commands.spawn();
    let builder = builder
        .insert_bundle(GeometryBuilder::build_as(
            &build_hex_shape(),
            ShapeColors::outlined(tile.color(), Color::BLACK),
            DrawMode::Outlined {
                fill_options: FillOptions::default(),
                outline_options: StrokeOptions::default().with_line_width(10.0),
            },
            Transform::from_xyz(x, y, -0.1),
        ))
        .insert(c)
        .insert(SelectableTile);
    match tile {
        State::Active => {
            builder.insert(OfficeTile { ticks_wo_worker: 0 });
        }
        State::BreakShop => {
            builder.insert(CoffeeTile { waiting_ticks: 0 });
        }
        _ => {}
    }
    if DEBUG_MODE {
        builder.with_children(|ec| {
            ec.spawn_bundle(Text2dBundle {
                text,
                transform: Transform::from_xyz(0., 0., 0.1),
                ..Text2dBundle::default()
            });
        });
    }
    builder.id()
}

fn office_system(
    mut query: Query<(&Coordinate, &mut OfficeTile)>,
    mut events: EventReader<TickEvent>,
    mut spawn_events: EventWriter<SpawnWorkerEvent>,
    map: Res<Map>,
) {
    let mut rng = thread_rng();
    for _ in events.iter() {
        for (coord, mut office) in query.iter_mut() {
            let chance =
                BASE_CHANCE_TO_SPAWN_WORKER + CHANCE_INCREASE_PER_TICK * office.ticks_wo_worker;
            let next = rng.gen_range(0..HUNDRED_PERCENT_CHANCE);
            if next < chance {
                office.ticks_wo_worker = 0;
                // spawn worker
                let is_passable = |c| map.tiles.get(&c).map(State::is_passable).unwrap_or(false);
                let is_dest = |c| map.tiles.get(&c).map(State::is_coffee).unwrap_or(false);
                let mut traverser = Traverser::new(is_passable, is_dest, *coord);
                let coffee = if let Some(x) = traverser.find() {
                    x
                } else {
                    log::error!("Cannot find nearest coffee shop");
                    continue;
                };
                let mut path = vec![coffee];
                let mut end = coffee;
                loop {
                    let next = traverser.backtrace(end).unwrap();
                    if next == *coord {
                        break;
                    }
                    path.push(next);
                    end = next;
                }
                log::debug!("Spawn worker from {:?} to {:?}", coord, coffee);
                let event = SpawnWorkerEvent(*coord, coffee, path);
                spawn_events.send(event);
            } else {
                office.ticks_wo_worker += 1;
            }
        }
    }
}

fn process_coffees(
    mut commands: Commands,
    w_workers: Query<(Entity, &Worker), (With<WaitingWorker>, Without<ReturningWorker>)>,
    mut shops: Query<(&Coordinate, &mut CoffeeTile)>,
    mut ticks: EventReader<TickEvent>,
    mut money: EventWriter<ChangeMoneyEvent>,
) {
    for _ in ticks.iter() {
        for (coord, mut shop) in shops.iter_mut() {
            if shop.waiting_ticks != 0 {
                shop.waiting_ticks -= 1;
                continue;
            }
            log::debug!("looking for workers");
            let res = w_workers.iter().find(|(_, w)| w.coffee == *coord);
            let (w_entity, _) = if let Some(x) = res {
                x
            } else {
                log::debug!("no workers");
                continue;
            };
            shop.waiting_ticks = WAIT_TICKS_AFTER_SERVING;
            let mut ec = commands.entity(w_entity);
            ec.insert(ReturningWorker);
            money.send(ChangeMoneyEvent(REWARD_FOR_COFFEE));
        }
    }
}

fn return_worker(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Worker), With<ReturningWorker>>,
    map: Res<Map>,
) {
    for (entity, mut worker) in query.iter_mut() {
        let is_passable = |c| map.tiles.get(&c).map(State::is_passable).unwrap_or(false);
        let is_dest = |c| c == worker.home;
        let mut traverser = Traverser::new(is_passable, is_dest, worker.coffee);
        let coffee = if let Some(x) = traverser.find() {
            x
        } else {
            log::error!("Cannot find nearest coffee shop");
            continue;
        };
        let mut path = vec![coffee];
        let mut end = coffee;
        loop {
            let next = traverser.backtrace(end).unwrap();
            if next == worker.coffee {
                break;
            }
            path.push(next);
            end = next;
        }
        worker.path = path;
        worker.waited_for_coffee = true;
        commands
            .entity(entity)
            .remove::<ReturningWorker>()
            .remove::<WaitingWorker>();
    }
}

fn setup(mut commands: Commands, asset_server: ResMut<AssetServer>, map: Res<Map>) {
    let font_handle = asset_server.load("FiraSans-Bold.ttf");
    for (c, tile) in map.tiles.iter() {
        spawn_tile(&mut commands, &font_handle, *c, *tile);
    }
}

fn generate_next_ring(
    mut commands: Commands,
    mut map: ResMut<Map>,
    mut timer: ResMut<NextRingTimer>,
    time: Res<Time>,
    asset_server: ResMut<AssetServer>,
    mut next_ring_event: EventWriter<GeneratedNextRing>
) {
    if !timer.0.tick(time.delta()).finished() {
        return;
    }
    *timer = NextRingTimer(Timer::from_seconds(NEXT_RING_TIMER_SECS as f32, false));
    let font_handle = asset_server.load("FiraSans-Bold.ttf");
    map.generated_rings += 1;
    let next_ring = Coordinate::new(0, 0).ring_iter(map.generated_rings as i32, Spin::CW(Direction::XY));
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
        spawn_tile(&mut commands, &font_handle, c, tile);
    }
    next_ring_event.send(GeneratedNextRing(map.generated_rings));
}

impl Default for Map {
    fn default() -> Self {
        let mut tiles = HashMap::default();
        let start = Coordinate::new(0, 0);
        tiles.insert(start, State::Active);
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

pub struct SelectedHex {
    entity: Entity,
    coordinate: Coordinate,
}

fn upgrade_hex(
    mut commands: Commands,
    selected: Res<Option<SelectedHex>>,
    mut map: ResMut<Map>,
    mut events: EventReader<UpgradeTileEvent>,
    asset_server: ResMut<AssetServer>,
    tiles: Query<(Entity, &Coordinate), With<SelectableTile>>,
) {
    for _ in events.iter() {
        let selected = if let Some(x) = selected.as_ref() {
            x
        } else {
            continue;
        };
        let upgradable = map
            .tiles
            .get(&selected.coordinate)
            .map(|s| s.is_upgradeable())
            .unwrap_or(false);
        if !upgradable {
            continue;
        }
        let (entity, _) = tiles
            .iter()
            .find(|(_, c)| **c == selected.coordinate)
            .expect("already checked tile for existence");
        map.tiles.insert(selected.coordinate, State::BreakShop);
        commands.entity(entity).despawn_recursive();
        let font_handle = asset_server.load("FiraSans-Bold.ttf");
        spawn_tile(
            &mut commands,
            &font_handle,
            selected.coordinate,
            State::BreakShop,
        );
    }
}

fn select_hex(
    mut commands: Commands,
    windows: Res<Windows>,
    tiles: Query<(Entity, &Coordinate), With<SelectableTile>>,
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
    if let Some(val) = selected_hex.as_ref() {
        if val.coordinate == coordinate {
            // don't deselect or select
            return;
        }
        commands.entity(val.entity).despawn();
        selected_hex.take();
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
            .add_system(office_system.system())
            .add_system(return_worker.system())
            .add_system(process_coffees.system())
            .add_system(upgrade_hex.system())
            .add_system(select_hex.system());
    }
}
