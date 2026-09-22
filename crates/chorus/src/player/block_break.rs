use glam::{IVec3, Vec3};

const FX_INTERVAL_TICKS: u32 = 5;
const MAX_PLAYER_DISTANCE: f32 = 16.0;

pub enum BreakTick {
    Continue { fx: bool },
    Stop,
}

pub struct BlockBreakHandler {
    position: IVec3,
    face: i32,
    block_id: i32,
    speed: f32,
    progress: f32,
    fx_ticker: u32,
}

impl BlockBreakHandler {
    pub fn new(position: IVec3, face: i32, block_id: i32, speed: f32) -> Self {
        Self {
            position,
            face,
            block_id,
            speed,
            progress: 0.,
            fx_ticker: 0,
        }
    }

    pub fn position(&self) -> IVec3 {
        self.position
    }

    pub fn face(&self) -> i32 {
        self.face
    }

    pub fn block_id(&self) -> i32 {
        self.block_id
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn set_face(&mut self, face: i32) {
        self.face = face;
    }

    pub fn targets(&self, position: IVec3) -> bool {
        self.position == position
    }

    pub fn update(&mut self, player_position: Vec3) -> BreakTick {
        let center = self.position.as_vec3() + Vec3::splat(0.5);
        if player_position.distance_squared(center) > MAX_PLAYER_DISTANCE * MAX_PLAYER_DISTANCE {
            return BreakTick::Stop;
        }

        self.progress += self.speed;
        if self.progress >= 1. {
            return BreakTick::Stop;
        }

        let fx = self.fx_ticker % FX_INTERVAL_TICKS == 0;
        self.fx_ticker += 1;

        BreakTick::Continue { fx }
    }
}

pub fn break_speed(hardness: f32, needs_tool: bool) -> f32 {
    if hardness < 0. {
        return 0.;
    }

    let multiplier = if needs_tool { 5.0 } else { 1.5 };
    let ticks = hardness * multiplier * 20.;

    if ticks > 0. { 1. / ticks } else { 1. }
}
