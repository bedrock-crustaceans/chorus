mod builder;
pub use builder::StrongholdBuilder;
mod build;
mod corridors;
mod frame;
mod layout;
mod material;
mod portal;
mod rooms;
mod spiral;

use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::BoundingBox;
use frame::{Canvas, Frame};
pub use layout::generate;
pub use material::StrongholdBlocks;
pub use spiral::spiral_starts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Door {
    Opening,
    Wood,
    Grates,
    Iron,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Straight,
    PrisonHall,
    LeftTurn,
    RightTurn,
    RoomCrossing,
    StraightStairsDown,
    StairsDown,
    FiveCrossing,
    ChestCorridor,
    Library,
    PortalRoom,
    Filler,
}

#[derive(Debug, Clone)]
pub struct Piece {
    pub kind: Kind,
    pub frame: Frame,
    pub door: Door,
    pub flags: [bool; 4],
    pub value: i32,
}

#[derive(Debug, Clone)]
pub struct Stronghold {
    pub pieces: Vec<Piece>,
    pub bounds: BoundingBox,
}

impl Stronghold {
    fn post_process(&self, canvas: &mut Canvas, random: &mut MersenneTwister) {
        let column = canvas.bounds;
        for piece in self.pieces.iter().filter(|piece| piece.frame.bounds.intersects(&column)) {
            match piece.kind {
                Kind::PortalRoom => portal::portal_room(&piece.frame, canvas, random),
                Kind::RoomCrossing | Kind::FiveCrossing | Kind::Library => rooms::post_process(piece, canvas, random),
                _ => corridors::post_process(piece, canvas, random),
            }
        }
    }

    pub fn describe(&self) -> Vec<String> {
        self.pieces
            .iter()
            .map(|piece| format!("{:?} {:?} {} depth {}", piece.kind, piece.frame.bounds, piece.frame.orientation, piece.frame.depth))
            .collect()
    }
}

pub fn place(blocks: &Blocks, ids: &StrongholdBlocks, chunk: &mut ProtoChunk, strongholds: &[&Stronghold], random: &mut MersenneTwister) {
    let mut canvas = Canvas::new(chunk, blocks, ids);
    for stronghold in strongholds {
        stronghold.post_process(&mut canvas, random);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::structure::seed::LargeFeatureSeed;
    const GENERAL_1: i64 = 4802433367836389877;

    #[test]
    fn layouts_are_repeatable_and_hold_a_portal_room() {
        let seeds = LargeFeatureSeed::new(GENERAL_1 as u32);
        for start in [(91, -52), (17, 177), (151, 218), (-89, -122)] {
            let first = generate(
                &mut seeds.blueprint(start),
                start,
                chorus_level::dimension_definition::DimensionDefinition::OVERWORLD,
                crate::shared::SEA_LEVEL,
            );
            let second = generate(
                &mut seeds.blueprint(start),
                start,
                chorus_level::dimension_definition::DimensionDefinition::OVERWORLD,
                crate::shared::SEA_LEVEL,
            );
            assert_eq!(first.describe(), second.describe());
            assert!(first.pieces.iter().any(|piece| piece.kind == Kind::PortalRoom), "{start:?}");
            assert!(first.pieces.iter().all(|piece| first.bounds.intersects(&piece.frame.bounds)));
        }
    }
}
