use bevy::{
    log,
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_prototype_lyon::prelude::*;
use hex2d::{Coordinate, Direction, Spacing, Spin};
use hex2d_dpcext::algo::bfs::Traverser;
use rand::{
    distributions::WeightedIndex,
    prelude::{Distribution, SliceRandom},
    thread_rng, Rng,
};

use crate::{
    daytime::TickEvent,
    ui::{ChangeMoneyEvent, GeneratedNextRing, UpgradeTileEvent},
    upgrade_particles::StartUpgradeEmitter,
    workers::{ReturningWorker, SpawnWorkerEvent, WaitingWorker, Worker},
    MainCamera,
};

const NEIGHBOURS_WEIGHTS: [[(State, u8); 3]; 3] = [
    [
        (State::Inactive, 40),
        (State::Active, 40),
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
pub const TIMER_MULTIPLER: f32 = 1.2;

const BASE_CHANCE_TO_SPAWN_WORKER: u32 = 9;
const CHANCE_INCREASE_PER_TICK: u32 = 1;
const HUNDRED_PERCENT_CHANCE: u32 = 200;

const WAIT_TICKS_AFTER_SERVING: u32 = 3;
const MAX_SHOPS_INCREASE: u32 = 3;
const STARTING_MAX_SHOPS: u32 = 1;

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

pub struct CoffeeShops(pub u32, pub u32);
impl Default for CoffeeShops {
    fn default() -> Self {
        Self(1, STARTING_MAX_SHOPS)
    }
}

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

fn spawn_tile(commands: &mut Commands, c: Coordinate, tile: State) -> Entity {
    let (x, y) = c.to_pixel(Spacing::FlatTop(SIZE));
    let mut builder = commands.spawn();
    let builder = builder
        .insert_bundle(GeometryBuilder::build_as(
            &build_hex_shape(),
            ShapeColors::outlined(tile.color(), Color::BLACK),
            DrawMode::Outlined {
                fill_options: FillOptions::default(),
                outline_options: StrokeOptions::default().with_line_width(10.0),
            },
            Transform::from_xyz(x, y, 0.),
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
                    log::debug!("Cannot find nearest coffee shop");
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
    w_workers: Query<(Entity, &Worker, &WaitingWorker), Without<ReturningWorker>>,
    mut shops: Query<(&Coordinate, &mut CoffeeTile)>,
    mut ticks: EventReader<TickEvent>,
    mut money: EventWriter<ChangeMoneyEvent>,
) {
    let mut set = HashSet::with_capacity_and_hasher(2, Default::default());
    for _ in ticks.iter() {
        for (coord, mut shop) in shops.iter_mut() {
            if shop.waiting_ticks != 0 {
                shop.waiting_ticks -= 1;
                continue;
            }
            log::debug!("looking for workers");
            let res = w_workers.iter().find(|(entity, w, ww)| {
                !ww.is_dead() && w.coffee == *coord && !set.contains(entity)
            });
            let (w_entity, worker, _) = if let Some(x) = res {
                x
            } else {
                log::debug!("no workers");
                continue;
            };
            set.insert(w_entity);
            shop.waiting_ticks = WAIT_TICKS_AFTER_SERVING;
            let mut ec = commands.entity(w_entity);
            ec.insert(ReturningWorker);
            money.send(ChangeMoneyEvent(worker.will_bring_money as i32));
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
            log::debug!("Cannot find nearest coffee shop");
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

fn setup(mut commands: Commands, map: Res<Map>) {
    for (c, tile) in map.tiles.iter() {
        spawn_tile(&mut commands, *c, *tile);
    }
}

fn generate_next_ring(
    mut commands: Commands,
    mut map: ResMut<Map>,
    mut timer: ResMut<NextRingTimer>,
    time: Res<Time>,
    mut next_ring_event: EventWriter<GeneratedNextRing>,
    mut shops: ResMut<CoffeeShops>,
) {
    if !timer.0.tick(time.delta()).finished() {
        return;
    }
    let duration = timer.0.duration().mul_f32(TIMER_MULTIPLER);
    timer.0 = Timer::new(duration, false);
    map.generated_rings += 1;
    let next_ring =
        Coordinate::new(0, 0).ring_iter(map.generated_rings as i32, Spin::CW(Direction::XY));
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
        spawn_tile(&mut commands, c, tile);
    }
    next_ring_event.send(GeneratedNextRing(map.generated_rings));
    let delta = (map.generated_rings - 1).min(MAX_SHOPS_INCREASE);
    shops.1 += delta;
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
    tiles: Query<(Entity, &Coordinate), With<SelectableTile>>,
    mut shops: ResMut<CoffeeShops>,
    mut emitter_events: EventWriter<StartUpgradeEmitter>,
) {
    for _ in events.iter() {
        if shops.0 >= shops.1 {
            continue;
        }
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
        shops.0 += 1;
        map.tiles.insert(selected.coordinate, State::BreakShop);
        commands.entity(entity).despawn_recursive();
        spawn_tile(&mut commands, selected.coordinate, State::BreakShop);
        let (x, y) = selected.coordinate.to_pixel(Spacing::FlatTop(SIZE));
        emitter_events.send(StartUpgradeEmitter(Vec3::new(x, y, 0.2)));
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
            .init_resource::<CoffeeShops>()
            .add_system(generate_next_ring.system())
            .add_system(office_system.system())
            .add_system(return_worker.system())
            .add_system(process_coffees.system().label("coffee"))
            .add_system(upgrade_hex.system())
            .add_system(select_hex.system());
    }
}
