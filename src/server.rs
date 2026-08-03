use crate::config::Config;
use crate::network::network::Network;
use crate::registry::Registry;
use crate::utils::rolling_avg::RollingAvg;
use bevy_app::{App, FixedFirst, FixedLast, FixedUpdate, Plugin, Startup};
use bevy_ecs::prelude::{Res, Resource};
use bevy_ecs::system::ResMut;
use bevy_time::{Fixed, Time};
use std::time::Instant;
use tracing::info;

pub const TICK_RATE: f64 = 20.0;

pub struct Server;

#[derive(Resource)]
pub struct ServerState {
    tick: i64,
    tick_instant: Instant,

    runtime_id: u64,
}

impl ServerState {
    pub fn get_runtime_id(&mut self) -> u64 {
        let id = self.runtime_id;
        self.runtime_id = self.runtime_id.wrapping_add(1);
        id
    }
}

#[derive(Resource)]
pub struct ServerMetrics {
    tps_min: f64,
    tps_avg: RollingAvg<f64>,
    mspt_max: f64,
    mspt_avg: RollingAvg<f64>,
}

impl Plugin for Server {
    fn build(&self, app: &mut App) {
        app.insert_resource(ServerState {
            tick: 0,
            tick_instant: Instant::now(),

            runtime_id: 1,
        })
        .insert_resource(ServerMetrics {
            tps_min: 20.0,
            tps_avg: RollingAvg::new(20),
            mspt_max: 0.0,
            mspt_avg: RollingAvg::new(20),
        })
        .insert_resource(Time::<Fixed>::from_hz(TICK_RATE))
        .add_systems(Startup, Server::start)
        .add_systems(FixedFirst, Server::start_tick)
        .add_systems(FixedUpdate, Server::tick)
        .add_systems(FixedLast, Server::end_tick)
        .add_plugins(Registry)
        .add_plugins(Network);
    }
}

impl Server {
    pub fn start(config: Res<Config>) {
        info!("Started on {}:{}.", config.ip, config.port);
    }

    pub fn start_tick(mut server_state: ResMut<ServerState>) {
        server_state.tick += 1;
        server_state.tick_instant = Instant::now();
    }

    pub fn tick(server_state: Res<ServerState>, server_metrics: Res<ServerMetrics>) {
        if server_state.tick % 20 == 0 {
            info!(
                "T: {}, TPS Min: {:.2}, MSPT Max: {:.2}, TPS Avg: {:.2}, MSPT Avg: {:.2}",
                server_state.tick,
                server_metrics.tps_min,
                server_metrics.mspt_max,
                server_metrics.tps_avg.get_avg(),
                server_metrics.mspt_avg.get_avg()
            );
        }
    }

    pub fn end_tick(time: Res<Time>, server_state: Res<ServerState>, mut server_metrics: ResMut<ServerMetrics>) {
        let mspt = server_state.tick_instant.elapsed().as_secs_f64() * 1_000.;
        let tps = 1. / time.delta_secs_f64();

        server_metrics.tps_min = server_metrics.tps_min.min(tps);
        server_metrics.tps_avg.add(tps);
        server_metrics.mspt_max = server_metrics.mspt_max.max(mspt);
        server_metrics.mspt_avg.add(mspt);
    }
}
