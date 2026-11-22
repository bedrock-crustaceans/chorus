use std::sync::{Arc, Mutex};

use rustc_hash::FxHashMap;

use chorus_level::generator::dimension::WorldGenerator;
use chorus_level::generator::engine::Engine;
use chorus_level::generator::phase::{Phase, PhaseHandle, PhaseId};
use chorus_worldgen::bedrock::overworld::OverworldGenerator;
use chorus_worldgen::bedrock::overworld::phases::{CarverPhase, FillPhase, HeightsPhase, RawTerrainPhase, StartsPhase};
use chorus_worldgen::bedrock::overworld::structure_probe::Supply;
use chorus_worldgen::shared::chunk::proto::ProtoChunk;
use chorus_worldgen::shared::structure::{ColumnHeights, Starts};
use chorus_worldgen::shared::terrain::NoiseChunk;
use glam::IVec2;

pub struct GraphSupply {
    generator: Arc<OverworldGenerator>,
    graph: Mutex<Engine<OverworldGenerator>>,
    heights: Mutex<FxHashMap<(i32, i32), Arc<ColumnHeights>>>,
    chunks: Mutex<FxHashMap<(i32, i32), Arc<ProtoChunk>>>,
    raw_chunks: Mutex<FxHashMap<(i32, i32), Arc<ProtoChunk>>>,
    starts: Mutex<FxHashMap<(i32, i32), Arc<Starts>>>,
}

impl GraphSupply {
    pub fn new(seed: i64) -> Self {
        bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
        let graph = Engine::new(OverworldGenerator::new(seed, &super::registry()).expect("the generator builds"));
        Self {
            generator: graph.generator.clone(),
            graph: Mutex::new(graph),
            heights: Mutex::default(),
            chunks: Mutex::default(),
            raw_chunks: Mutex::default(),
            starts: Mutex::default(),
        }
    }

    pub fn generator(&self) -> Arc<OverworldGenerator> {
        self.generator.clone()
    }

    fn fetch<P: Phase<OverworldGenerator>>(&self, cell: (i32, i32)) -> Arc<P::Output> {
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

    pub fn require<P: Phase<OverworldGenerator>>(&self, cell: (i32, i32)) {
        self.fetch::<P>(cell);
    }

    pub fn noise(&self, cell: (i32, i32)) -> (ProtoChunk, NoiseChunk, chorus_worldgen::bedrock::overworld::Aquifer) {
        let output = self.fetch::<FillPhase>(cell);
        let (chunk, noise, aquifer) = &*output;
        (chunk.clone(), noise.clone(), aquifer.clone())
    }
}

fn remembered<T: Clone>(table: &Mutex<FxHashMap<(i32, i32), T>>, cell: (i32, i32), load: impl FnOnce() -> T) -> T {
    if let Some(known) = table.lock().expect("the harness table").get(&cell) {
        return known.clone();
    }
    let value = load();
    table.lock().expect("the harness table").insert(cell, value.clone());
    value
}

impl Supply for GraphSupply {
    fn heights(&self, cell: (i32, i32)) -> Option<Arc<ColumnHeights>> {
        Some(remembered(&self.heights, cell, || self.fetch::<HeightsPhase>(cell)))
    }

    fn chunk(&self, cell: (i32, i32)) -> Option<Arc<ProtoChunk>> {
        Some(remembered(&self.chunks, cell, || self.fetch::<CarverPhase>(cell)))
    }

    fn raw_chunk(&self, cell: (i32, i32)) -> Option<Arc<ProtoChunk>> {
        Some(remembered(&self.raw_chunks, cell, || self.fetch::<RawTerrainPhase>(cell)))
    }

    fn starts(&self, cell: (i32, i32)) -> Option<Arc<Starts>> {
        Some(remembered(&self.starts, cell, || self.fetch::<StartsPhase>(cell)))
    }
}
