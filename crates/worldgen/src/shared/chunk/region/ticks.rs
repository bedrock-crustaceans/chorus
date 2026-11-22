use super::{CENTER, Region};
use crate::shared::chunk::proto::{FluidTick, RoutedTick};
use glam::IVec3;

#[derive(Clone, Copy, Debug)]
pub struct FluidSchedule {
    pub pos: IVec3,
    pub lava: bool,
    pub delay: u64,
    pub on_chunk_queue: bool,
}

const QUEUE_REACH: i32 = 8;

impl Region<'_> {
    pub fn chunk_fluid_ticks(&self) -> &[FluidTick] {
        self.chunks[CENTER].fluid_ticks()
    }

    pub fn queue_reaches(&self, pos: IVec3) -> bool {
        [-QUEUE_REACH, QUEUE_REACH]
            .iter()
            .all(|dx| [-QUEUE_REACH, QUEUE_REACH].iter().all(|dz| self.has_column(pos.x + dx, pos.z + dz)))
    }

    pub fn schedule_fluid(&mut self, pos: IVec3, lava: bool, delay: u64) {
        let on_chunk_queue = !self.queue_reaches(pos);
        self.fluid_schedule.push(FluidSchedule { pos, lava, delay, on_chunk_queue });
    }

    pub fn schedule_on_chunk(&mut self, pos: IVec3, lava: bool, delay: u64) {
        self.fluid_schedule.push(FluidSchedule {
            pos,
            lava,
            delay,
            on_chunk_queue: true,
        });
    }

    pub fn route_tick(&mut self, tick: RoutedTick) {
        if let Some(slot) = self.slot(tick.pos.x, tick.pos.z) {
            self.routed[slot].push(tick);
        }
    }

    pub fn inherited_ticks(&self) -> &[RoutedTick] {
        self.chunks[CENTER].inherited_ticks()
    }

    pub fn take_fluid_schedule(&mut self) -> Vec<FluidSchedule> {
        std::mem::take(&mut self.fluid_schedule)
    }

    pub fn set_pending_fluid(&mut self, ticks: Vec<RoutedTick>) {
        self.chunks[CENTER].set_pending_fluid(ticks);
    }

    pub fn structure_magma_ticks(&self) -> &[(u8, i16, u8)] {
        self.chunks[CENTER].magma_ticks()
    }
}
