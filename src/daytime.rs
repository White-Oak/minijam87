use std::fmt::Display;

use bevy::{core::FixedTimestep, prelude::*};

const MINUTES_IN_TICK: u32 = 1;
const TICKS_IN_RUN: u32 = 1;
const MAX_TICKS: u32 = MINUTES_IN_TICK * 60 * 24;

#[derive(Debug)]
pub struct Daytime(u32);

pub struct TickEvent;

impl Default for Daytime {
    fn default() -> Self {
        Self(7 * 60 / MINUTES_IN_TICK)
    }
}

impl Daytime {
    fn get_minutes(&self) -> u32 {
        self.0 % 60
    }

    fn get_hours(&self) -> u32 {
        self.0 / 60
    }

    fn add(&mut self, ticks: u32) {
        self.0 = (self.0 + ticks) % MAX_TICKS
    }
}

impl Display for Daytime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}", self.get_hours(), self.get_minutes())
    }
}

fn update_daytime(mut daytime: ResMut<Daytime>, mut events: EventWriter<TickEvent>) {
    daytime.add(TICKS_IN_RUN);
    for _ in 0..TICKS_IN_RUN {
        events.send(TickEvent);
    }
}

pub struct DaytimePlugin;

impl Plugin for DaytimePlugin {
    fn build(&self, app: &mut bevy::prelude::AppBuilder) {
        app.init_resource::<Daytime>()
            .add_system_set(
                SystemSet::new()
                    .with_run_criteria(FixedTimestep::steps_per_second(8.))
                    .with_system(update_daytime.system()),
            )
            .add_event::<TickEvent>();
    }
}
