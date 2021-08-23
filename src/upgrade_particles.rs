use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::{core::Timer, math::Vec3};
use rand::{thread_rng, Rng};

use crate::field::SIZE;

struct Particle;
struct Lifetime(i32);
struct Velocity(Vec3);
struct Acceleration(Vec3);
struct Alive(bool);

struct UpgradeEmitter {
    duration: Timer,
}

pub struct StartUpgradeEmitter(pub Vec3);

const EMIT_DURATION: f32 = 10.;

const VARIETY: usize = 50;
const MAX_GREEN: f32 = 0.9;
const STEP_GREEN: f32 = MAX_GREEN / VARIETY as f32;
const INITIAL_SIZE: f32 = 15.;
const MAX_LIFETIME: i32 = 100;
const AMOUNT: u32 = 50;
const AMOUNT_VARIANCE: f32 = 0.2;

fn create_emitter(
    mut event_reader: EventReader<StartUpgradeEmitter>,
    mut commands: Commands,
    particle_materials: Res<ParticleMaterials>,
) {
    for StartUpgradeEmitter(translation) in event_reader.iter() {
        commands
            .spawn()
            .insert(Transform::from_translation(*translation))
            .insert(GlobalTransform::default())
            .insert(UpgradeEmitter {
                duration: Timer::from_seconds(EMIT_DURATION, false),
            })
            .with_children(|ec| {
                let mut rng = thread_rng();
                let amount =
                    AMOUNT as f32 * rng.gen_range((1. - AMOUNT_VARIANCE)..(1. + AMOUNT_VARIANCE));
                let amount = amount as usize;
                for _ in 0..amount {
                    let tile_size = Vec2::splat(INITIAL_SIZE);
                    let variety_idx = rng.gen_range(0..VARIETY);
                    let material = particle_materials.0[variety_idx].clone();
                    let trnsl = start_location(&mut rng);
                    let transform = Transform::from_translation(trnsl);
                    let velocity = trnsl / (SIZE) * rng.gen_range(0.5..2.0);
                    ec.spawn_bundle(SpriteBundle {
                        sprite: Sprite::new(tile_size),
                        material,
                        transform,
                        ..Default::default()
                    })
                    .insert(Particle)
                    .insert(Acceleration(Vec3::new(0.0, 0.0, 0.0)))
                    .insert(Velocity(velocity))
                    .insert(Alive(true))
                    .insert(Lifetime(MAX_LIFETIME));
                }
            });
    }
}

struct ParticleMaterials(Vec<Handle<ColorMaterial>>);
impl FromWorld for ParticleMaterials {
    fn from_world(world: &mut bevy::prelude::World) -> Self {
        let world = world.cell();
        let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        let mut vec = vec![];
        let asset_server = world.get_resource_mut::<AssetServer>().unwrap();
        let texture = asset_server.load("particle.png");
        for i in 0..VARIETY {
            let mut material: ColorMaterial = texture.clone().into();
            let green = i as f32 * STEP_GREEN;
            material.color = Color::rgba(green, 1., green, 1.);
            let material = materials.add(material);
            vec.push(material);
        }
        ParticleMaterials(vec)
    }
}

fn start_location<R: Rng>(rng: &mut R) -> Vec3 {
    let radius = SIZE / 2. * 3_f32.sqrt();
    let radius = rng.gen_range(1.0..1.1) * radius;
    let angle = rng.gen_range(0_f32..2_f32 * PI);
    // rotating (0, radius)
    let x = -angle.sin() * radius;
    let y = -angle.cos() * radius;
    Vec3::new(x, y, 0.)
}

fn kill_particles(mut commands: Commands, mut query: Query<(Entity, &mut Lifetime, &mut Sprite)>) {
    for (entity, mut lifetime, mut sprite) in query.iter_mut() {
        lifetime.0 -= 3;
        let ratio = (lifetime.0 as f32) / MAX_LIFETIME as f32;
        sprite.size = Vec2::splat(INITIAL_SIZE * ratio);
        if lifetime.0 <= 0 {
            commands.entity(entity).despawn();
        }
    }
}

fn kill_emitter(
    mut commands: Commands,
    mut query: Query<(Entity, &mut UpgradeEmitter)>,
    time: Res<Time>,
) {
    for (entity, mut emitter) in query.iter_mut() {
        if emitter.duration.tick(time.delta()).finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn update_pos(
    mut query: Query<(&mut Transform, &mut Velocity, &Acceleration, &Alive), With<Particle>>,
) {
    for (mut pos, mut vel, accel, is_alive) in query.iter_mut() {
        if is_alive.0 {
            vel.0 += accel.0;
            pos.translation += vel.0;
        }
    }
}

pub struct UpgradeParticlesPlugin;

impl Plugin for UpgradeParticlesPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(create_emitter.system())
            .add_system(kill_emitter.system())
            .add_system(kill_particles.system())
            .add_system(update_pos.system())
            .add_event::<StartUpgradeEmitter>()
            .init_resource::<ParticleMaterials>();
    }
}
