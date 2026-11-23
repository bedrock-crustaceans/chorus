use std::marker::PhantomData;

use chorus_level::generator::dimension::Generator;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Phase, PhaseContext};
use glam::IVec2;

pub trait CellOutput<G: Generator>: 'static {
    type Value: Send + Sync + 'static;

    const NAME: &'static str;

    const RETAIN: usize;

    fn compute(generator: &G, cell: IVec2) -> Self::Value;
}

pub struct CellPhase<K>(PhantomData<fn() -> K>);

impl<G: Generator, K: CellOutput<G>> Phase<G> for CellPhase<K> {
    type Output = K::Value;

    const RETAIN: usize = K::RETAIN;

    const NAME: &'static str = K::NAME;

    fn run(generator: &G, cell: IVec2, _context: &mut PhaseContext<G>) -> Result<K::Value, PhaseError> {
        Ok(K::compute(generator, cell))
    }
}
