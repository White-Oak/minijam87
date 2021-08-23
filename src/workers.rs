use std::f32::consts::PI;

use bevy::{log, prelude::*, sprite::TextureAtlas};
use hex2d::{Coordinate, Spacing};
use rand::{thread_rng, Rng};

use crate::{
    daytime::TickEvent, field::SIZE, overwait_particles::StartOverwaitEmitter, ui::ChangeMoneyEvent,
};

pub struct Worker {
    pub home: Coordinate,
    pub coffee: Coordinate,
    pub path: Vec<Coordinate>,
    pub waited_for_coffee: bool,
    pub will_bring_money: u8,
}

const FRAMES_PER_ONE_TILE: u32 = 64;
pub struct MovingWorker(u32, Vec3);

const MAX_WAITING_TICKS: u32 = 50;
const FEE_FOR_OVERWAIT: i32 = -5;

fn money_for_path(path_len: usize) -> u8 {
    let y = (path_len as f32) * -0.5 + 2.5;
    y.ceil().max(0.) as u8
}

pub struct WaitingWorker(u32);
impl WaitingWorker {
    pub fn is_dead(&self) -> bool {
        self.0 >= MAX_WAITING_TICKS
    }
}

pub struct ReturningWorker;

struct WorkerAtlasResource {
    atlas: Handle<TextureAtlas>,
}

fn random_pos(c: &Coordinate) -> (f32, f32) {
    let mut rng = thread_rng();
    let (x, y) = c.to_pixel(Spacing::FlatTop(SIZE));
    let radius = SIZE / 2. * 3_f32.sqrt() - 15.;
    let radius = rng.gen_range(0_f32..radius);
    let angle = rng.gen_range(0_f32..2_f32 * PI);
    // rotating (0, radius)
    let x2 = x - angle.sin() * radius;
    let y2 = y - angle.cos() * radius;
    (x2, y2)
}

fn start_moving_worker(
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut Worker, &Transform),
        (Without<MovingWorker>, Without<WaitingWorker>),
    >,
) {
    for (entity, mut worker, transform) in query.iter_mut() {
        let mut ec = commands.entity(entity);
        if worker.path.is_empty() {
            if worker.waited_for_coffee {
                ec.despawn_recursive();
            } else {
                ec.insert(WaitingWorker(0));
                log::debug!("started waiting");
            }
            continue;
        }
        let next_c = worker.path.pop().unwrap();
        let (x, y) = random_pos(&next_c);
        let next = Vec3::new(x, y, 0.);
        let speed = (next - transform.translation) / FRAMES_PER_ONE_TILE as f32;
        let moving = MovingWorker(0, speed);
        ec.insert(moving);
    }
}

fn move_worker(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut MovingWorker)>,
) {
    for (entity, mut tr, mut mw) in query.iter_mut() {
        tr.translation += mw.1;
        mw.0 += 1;
        if mw.0 == FRAMES_PER_ONE_TILE {
            let mut ec = commands.entity(entity);
            ec.remove::<MovingWorker>();
            log::debug!("stopped moving");
        }
    }
}

fn wait_worker(
    mut commands: Commands,
    mut ticks: EventReader<TickEvent>,
    mut money: EventWriter<ChangeMoneyEvent>,
    mut query: Query<(Entity, &mut WaitingWorker, &Transform)>,
    mut overwait_events: EventWriter<StartOverwaitEmitter>,
) {
    for _ in ticks.iter() {
        for (entity, mut w, trns) in query.iter_mut() {
            w.0 += 1;
            if w.is_dead() {
                commands.entity(entity).despawn_recursive();
                money.send(ChangeMoneyEvent(FEE_FOR_OVERWAIT));
                overwait_events.send(StartOverwaitEmitter(trns.translation))
            }
        }
    }
}

pub struct SpawnWorkerEvent(pub Coordinate, pub Coordinate, pub Vec<Coordinate>);

fn spawn_worker(
    mut commands: Commands,
    atlas: Res<WorkerAtlasResource>,
    mut events: EventReader<SpawnWorkerEvent>,
) {
    for SpawnWorkerEvent(home, coffee, path) in events.iter() {
        let (x, y) = random_pos(home);
        let mut rng = thread_rng();
        let r = rng.gen_range(0..150);
        let g = rng.gen_range(0..150);
        let b = rng.gen_range(0..150);
        let head = TextureAtlasSprite {
            color: Color::rgb_u8(r, g, b),
            index: 0,
            ..Default::default()
        };
        let head = SpriteSheetBundle {
            sprite: head,
            texture_atlas: atlas.atlas.clone(),
            transform: Transform::from_translation(Vec3::new(0., 13., 0.1)),
            ..Default::default()
        };
        let r = rng.gen_range(50..200);
        let g = rng.gen_range(50..200);
        let b = rng.gen_range(50..200);
        let body = TextureAtlasSprite {
            color: Color::rgb_u8(r, g, b),
            index: 1,
            ..Default::default()
        };
        let body = SpriteSheetBundle {
            sprite: body,
            texture_atlas: atlas.atlas.clone(),
            ..Default::default()
        };
        let main_transform = Transform::from_xyz(x, y, 0.9);
        let will_bring_money = money_for_path(path.len());
        commands
            .spawn()
            .insert(Worker {
                home: *home,
                coffee: *coffee,
                path: path.clone(),
                waited_for_coffee: false,
                will_bring_money,
            })
            .insert(main_transform)
            .insert(GlobalTransform::default())
            .with_children(|ec| {
                ec.spawn_bundle(head);
                ec.spawn_bundle(body);
            });
    }
}

impl FromWorld for WorkerAtlasResource {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource_mut::<AssetServer>().unwrap();
        let tex_handle = asset_server.load("worker.png");
        let atlas = TextureAtlas::from_grid(tex_handle, Vec2::new(200. / 8., 198. / 8.), 2, 1);
        let mut atlases = world.get_resource_mut::<Assets<TextureAtlas>>().unwrap();
        let handle = atlases.add(atlas);
        WorkerAtlasResource { atlas: handle }
    }
}

pub struct WorkerPlugin;
impl Plugin for WorkerPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<WorkerAtlasResource>()
            .add_system(spawn_worker.system())
            .add_system(start_moving_worker.system())
            .add_system(move_worker.system())
            .add_system(wait_worker.system().before("coffee"))
            .add_event::<SpawnWorkerEvent>();
    }
}
