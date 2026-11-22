mod canopy;
mod parse;
mod roots;
mod trunk;

pub use canopy::{Canopy, CherryCanopy, MangroveCanopy, PoplarCanopy, SimpleCanopy};
pub use parse::{MALFORMED, block};
pub use roots::{BaseCluster, MangroveRoots};
pub use trunk::{AcaciaBranches, AcaciaTrunk, CherryTrunk, FallenTrunk, FancyTrunk, MangroveTrunk, MegaBranches, MegaTrunk, PoplarTrunk, SimpleTrunk, Trunk};

pub const UNSUPPORTED_SHAPE: &str = "unsupported tree shape";
