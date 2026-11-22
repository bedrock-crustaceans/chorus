use rustc_hash::FxHashSet;

use chorus_level::generator::dimension::Generator;
use chorus_level::generator::engine::LocateStep;
use chorus_level::generator::phase::{PhaseId, Request};

pub(super) struct Demand {
    asked: FxHashSet<(PhaseId, (i32, i32))>,
}

impl Demand {
    pub(super) fn new() -> Self {
        Self { asked: FxHashSet::default() }
    }

    pub(super) fn fresh<G: Generator>(&mut self, requests: impl IntoIterator<Item = Request<G>>) -> Vec<Request<G>> {
        requests
            .into_iter()
            .filter(|request| self.asked.insert((request.handle.phase, (request.cell.x, request.cell.y))))
            .collect()
    }

    pub(super) fn step<G: Generator>(&mut self, requests: impl IntoIterator<Item = Request<G>>) -> LocateStep<G> {
        let demands = self.fresh(requests);
        if demands.is_empty() { LocateStep::Done(None) } else { LocateStep::Demand(demands) }
    }
}
