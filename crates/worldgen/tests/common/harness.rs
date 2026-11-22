use std::sync::{Arc, Mutex};

use chorus_level::generator::dimension::{Generator, WorldGenerator};
use chorus_level::generator::engine::Engine;
use chorus_level::generator::phase::{Phase, PhaseHandle, PhaseId};
use glam::IVec2;

pub struct GraphHarness<G: Generator> {
    generator: Arc<G>,
    graph: Mutex<Engine<G>>,
}

impl<G: Generator> GraphHarness<G> {
    pub fn new(generator: G) -> Self {
        bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
        let graph = Engine::new(generator);
        Self {
            generator: graph.generator.clone(),
            graph: Mutex::new(graph),
        }
    }

    pub fn fetch<P: Phase<G>>(&self, cell: (i32, i32)) -> Arc<P::Output> {
        let position = IVec2::new(cell.0, cell.1);
        let mut graph = self.graph.lock().expect("the harness graph");
        graph.request_phase(PhaseHandle::of::<P>(), position);
        loop {
            graph.tick();
            for delivery in graph.take_delivered() {
                if delivery.phase == PhaseId::of::<P>() && delivery.cell == position {
                    let output = delivery.outcome.unwrap_or_else(|failure| panic!("{failure}"));
                    return output.downcast::<P::Output>().expect("the phase output type");
                }
            }
            std::thread::yield_now();
        }
    }
}

impl<G: Generator> std::ops::Deref for GraphHarness<G> {
    type Target = G;

    fn deref(&self) -> &G {
        &self.generator
    }
}
