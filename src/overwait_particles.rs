
use bevy::prelude::*;
use bevy::{core::Timer, math::Vec3};
use rand::{thread_rng, Rng};


struct Particle;
struct Lifetime(i32);
struct Velocity(Vec3);
struct Acceleration(Vec3);
struct Alive(bool);

const MAX_EMITTERS: u32 = 50;
#[derive(Default)]
struct CurrentEmitters(u32);

struct OverwaitEmitter {
    duration: Timer,
}

pub struct StartOverwaitEmitter(pub Vec3);

const EMIT_DURATION: f32 = 3.;

const VARIETY: usize = 50;
const MAX_RED: f32 = 0.9;
const STEP_RED: f32 = MAX_RED / VARIETY as f32;
const INITIAL_SIZE: f32 = 15.;
const MAX_LIFETIME: i32 = 100;
const AMOUNT: u32 = 30;
const AMOUNT_VARIANCE: f32 = 0.2;

fn create_emitter(
    mut event_reader: EventReader<StartOverwaitEmitter>,
    mut commands: Commands,
    particle_materials: Res<ParticleMaterials>,
    mut current_emitters: ResMut<CurrentEmitters>
) {
    for StartOverwaitEmitter(translation) in event_reader.iter() {
        if current_emitters.0 >= MAX_EMITTERS {
            continue;
        }
        current_emitters.0 += 1;
        let mut translation = *translation;
        translation.y += 30.;
        translation.z = 0.3;
        commands
            .spawn()
            .insert(Transform::from_translation(translation))
            .insert(GlobalTransform::default())
            .insert(OverwaitEmitter {
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
                    let transform = Transform::from_translation(Vec3::new(
                        rng.gen_range(-5.0..5.),
                        rng.gen_range(0.0..2.),
                        0.,
                    ));
                    let velocity =
                        Vec3::new(rng.gen_range(-0.05..0.05), rng.gen_range(2.0..4.), 0.);
                    ec.spawn_bundle(SpriteBundle {
                        sprite: Sprite::new(tile_size),
                        material,
                        transform,
                        ..Default::default()
                    })
                    .insert(Particle)
                    .insert(Acceleration(Vec3::new(0.0, 0.001, 0.0)))
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
            let red = i as f32 * STEP_RED;
            material.color = Color::rgba((0.2 + red).min(1.), 0.4, 0.4, 1.);
            let material = materials.add(material);
            vec.push(material);
        }
        ParticleMaterials(vec)
    }
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
    mut query: Query<(Entity, &mut OverwaitEmitter)>,
    time: Res<Time>,
    mut current_emitters: ResMut<CurrentEmitters>
) {
    for (entity, mut emitter) in query.iter_mut() {
        if emitter.duration.tick(time.delta()).finished() {
            commands.entity(entity).despawn_recursive();
            current_emitters.0 -= 1;
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

pub struct OverwaitParticlesPlugin;

impl Plugin for OverwaitParticlesPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(create_emitter.system())
            .add_system(kill_emitter.system())
            .add_system(kill_particles.system())
            .add_system(update_pos.system())
            .add_event::<StartOverwaitEmitter>()
            .init_resource::<ParticleMaterials>()
            .init_resource::<CurrentEmitters>()
            ;
    }
}
