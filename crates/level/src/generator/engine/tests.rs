use std::sync::Arc;
use std::time::{Duration, Instant};

use bevy_tasks::{AsyncComputeTaskPool, TaskPool};

use crate::chunk::Chunk;
use crate::generator::dimension::{Generator, WorldGenerator};
use crate::generator::error::{PhaseError, PhaseFailure};
use crate::generator::footprint::footprints;
use crate::generator::phase::{PhaseContext, PhaseId};
use crate::generator::tasks::{TaskKey, TaskStatus};

use super::focus::nearest_distance;
use super::*;
use glam::IVec2;

fn brute_force(foci: &[IVec2], cell: IVec2) -> Option<f64> {
    foci.iter()
        .map(|focus| (((cell.x - focus.x) as f64).powi(2) + ((cell.y - focus.y) as f64).powi(2)).sqrt())
        .reduce(f64::min)
}

#[test]
fn no_foci_have_no_distance() {
    assert_eq!(nearest_distance(&[], IVec2::new(3, 4)), None);
}

#[test]
fn the_nearest_focus_matches_a_scan_of_every_focus() {
    let mut state = 0x2545f4914f6cdd1du64;
    let mut next = |span: i32| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % span as u64) as i32 - span / 2
    };
    for count in [1usize, 2, 5, 40, 150] {
        let mut foci: Vec<IVec2> = (0..count).map(|_| IVec2::new(next(600), next(600))).collect();
        foci.sort_unstable_by_key(|focus| focus.x);
        for _ in 0..500 {
            let cell = IVec2::new(next(800), next(800));
            assert_eq!(nearest_distance(&foci, cell), brute_force(&foci, cell), "{count} foci, cell {cell:?}");
        }
    }
}

mod mail {
    use super::*;
    use crate::generator::phase::{Mailing, Phase, SAME_CELL};
    use std::sync::Mutex;

    struct Toy;

    impl Generator for Toy {
        type Terminal = SinkPhase;
    }

    struct SourcePhase;

    impl Phase<Toy> for SourcePhase {
        type Output = ();

        const RETAIN: usize = 16;

        fn run(_: &Toy, cell: IVec2, context: &mut PhaseContext<Toy>) -> Result<(), PhaseError> {
            SOURCE_RUNS.lock().unwrap().push(cell);
            context.send::<SourcePhase>(cell, 100 + cell.x);
            context.send::<SourcePhase>(IVec2::new(cell.x + 1, cell.y), 200 + cell.x);
            context.send::<SourcePhase>(IVec2::new(cell.x + 50, cell.y), 999);
            Ok(())
        }
    }

    impl Mailing<Toy> for SourcePhase {
        type Mail = i32;
    }

    const READ_FROM: &[(i32, i32)] = &[(-1, 0), (0, 0), (1, 0)];
    type Received = Vec<(IVec2, i32)>;
    static SEEN: Mutex<Vec<(IVec2, Received)>> = Mutex::new(Vec::new());
    static SOURCE_RUNS: Mutex<Vec<IVec2>> = Mutex::new(Vec::new());

    struct SinkPhase;

    impl Phase<Toy> for SinkPhase {
        type Output = Chunk;

        fn dependencies() -> Vec<crate::generator::phase::Dependency<Toy>> {
            vec![crate::generator::phase::Dependency::on::<SourcePhase>(READ_FROM).with_mail(SAME_CELL)]
        }

        fn run(_: &Toy, cell: IVec2, context: &mut PhaseContext<Toy>) -> Result<Chunk, PhaseError> {
            let received = context.mail::<SourcePhase>(cell).map(|(source, mail)| (source, *mail)).collect();
            SEEN.lock().unwrap().push((cell, received));
            Ok(Chunk::new(cell.x, cell.y, -4, 24, 0, 1))
        }
    }

    fn finish(graph: &mut Engine<Toy>, cells: &[IVec2]) {
        AsyncComputeTaskPool::get_or_init(TaskPool::default);
        for &cell in cells {
            graph.request(cell);
        }
        let mut done = 0;
        let start = Instant::now();
        while done < cells.len() {
            done += WorldGenerator::tick(graph).len();
            assert!(start.elapsed() < Duration::from_secs(30), "generation never finished");
        }
    }

    #[test]
    fn a_phase_receives_only_the_mail_it_declared_in_the_order_it_declared_it() {
        let mut graph = Engine::new(Toy);
        finish(&mut graph, &[IVec2::new(5, 0)]);
        let seen = SEEN.lock().unwrap();
        let (_, received) = seen.iter().find(|(cell, _)| *cell == IVec2::new(5, 0)).expect("the sink ran");
        assert_eq!(received, &[(IVec2::new(4, 0), 204), (IVec2::new(5, 0), 105)]);
    }

    #[test]
    fn mail_is_dropped_once_every_cell_that_could_read_it_has_finished() {
        let mut graph = Engine::new(Toy);
        let row: Vec<IVec2> = (20..24).map(|x| IVec2::new(x, 0)).collect();
        finish(&mut graph, &row);
        let source = PhaseId::of::<SourcePhase>();
        let cell = |x| IVec2::new(x, 0);
        assert!(!graph.mail_alive(source, cell(21), 0, cell(21)), "its reader finished");
        assert!(graph.mail_alive(source, cell(23), 0, cell(24)), "nothing finished there yet");
        assert!(!graph.mail_alive(source, cell(22), 0, cell(72)), "no cell lists that target");

        graph.sweep_retained();
        for (key, task) in graph.graph.tasks.iter().filter(|(_, task)| task.retained.is_some()) {
            assert!(
                graph
                    .graph
                    .mail
                    .get(&key)
                    .is_none_or(|mail| mail.out.iter().all(|(target, _)| graph.mail_alive(source, task.cell, task.hop, *target))),
                "retained tasks keep only mail somebody can still read"
            );
        }
    }

    #[test]
    fn a_node_with_pruned_mail_is_rebuilt_for_a_cell_that_needs_it_again() {
        let mut graph = Engine::new(Toy);
        let row: Vec<IVec2> = (300..304).map(|x| IVec2::new(x, 0)).collect();
        finish(&mut graph, &row);
        let first = SEEN.lock().unwrap().iter().find(|(cell, _)| *cell == IVec2::new(300, 0)).cloned().expect("the sink ran");
        let runs = |x: i32| SOURCE_RUNS.lock().unwrap().iter().filter(|cell| cell.x == x).count();
        let before = runs(299);

        graph.sweep_retained();
        finish(&mut graph, &[IVec2::new(300, 0)]);

        assert!(runs(299) > before, "the retained source had pruned the mail for that cell, so it was built again");
        let again = SEEN.lock().unwrap().iter().rev().find(|(cell, _)| *cell == IVec2::new(300, 0)).cloned().expect("the sink ran again");
        assert_eq!(again, first, "the cell sees exactly what it saw the first time");
    }

    #[test]
    fn the_retention_budget_caps_how_many_nodes_of_a_phase_are_kept() {
        let mut graph = Engine::new(Toy);
        graph.set_focus(&[(10, 10)], 20);
        graph.set_retention_budget(0.01);
        let cells: Vec<IVec2> = (0..20).flat_map(|x| (0..20).map(move |z| IVec2::new(x, z))).collect();
        finish(&mut graph, &cells);
        assert!(
            graph.retained_count() <= graph.retention_cap(),
            "kept {} against a cap of {}",
            graph.retained_count(),
            graph.retention_cap()
        );
        assert!(graph.retention_cap() < 40, "the cap has to be below the 40 edge tasks for this to test anything");
    }

    #[test]
    fn footprints_follow_the_declared_offsets() {
        let footprints = footprints::<Toy>();
        assert_eq!(footprints[&(PhaseId::of::<SinkPhase>(), 0)], vec![(0, 0)]);
        assert_eq!(footprints[&(PhaseId::of::<SourcePhase>(), 0)], vec![(-1, 0), (0, 0), (1, 0)]);
    }

    #[test]
    fn a_cancelled_request_keeps_its_pipeline_and_a_new_request_reuses_it() {
        let mut graph = Engine::new(Toy);
        let cell = IVec2::new(7, 0);
        graph.request(cell);
        let tasks = graph.graph.tasks.len();
        graph.cancel(cell);
        assert_eq!(graph.abandoned.len(), 1);
        assert_eq!(graph.graph.tasks.len(), tasks, "nothing was torn down");

        graph.request(cell);
        assert!(graph.abandoned.is_empty());
        assert_eq!(graph.graph.tasks.len(), tasks, "the request found the pipeline again");
    }

    #[test]
    fn an_abandoned_pipeline_is_released_once_no_focus_can_reach_it() {
        let mut graph = Engine::new(Toy);
        graph.set_focus(&[(7, 0)], 4);
        graph.request(IVec2::new(7, 0));
        graph.cancel(IVec2::new(7, 0));
        graph.set_focus(&[(8, 0)], 4);
        assert_eq!(graph.abandoned.len(), 1, "still within reach of the focus");

        graph.set_focus(&[(500, 500)], 4);
        assert!(graph.abandoned.is_empty());
        assert_eq!(graph.graph.tasks.len(), 0, "the whole pipeline is gone");
    }

    #[test]
    fn mail_for_cells_outside_the_view_region_is_not_kept() {
        let mut graph = Engine::new(Toy);
        graph.set_focus(&[(0, 0)], 3);
        let source = PhaseId::of::<SourcePhase>();
        assert!(graph.mail_alive(source, IVec2::new(2, 0), 0, IVec2::new(2, 0)));
        assert!(!graph.mail_alive(source, IVec2::new(60, 0), 0, IVec2::new(60, 0)));
    }
}

mod demand {
    use super::*;
    use crate::generator::phase::{Dependency, Phase, Request, SAME_CELL};
    use std::sync::Mutex;

    const REQUEST_DISTANCE: i32 = 2;
    const SOURCE_STRIDE_X: i32 = 37;
    const SOURCE_STRIDE_Z: i32 = 101;

    struct Toy;

    impl Generator for Toy {
        type Terminal = ReaderPhase;
    }

    struct BasePhase;

    impl Phase<Toy> for BasePhase {
        type Output = i32;

        fn run(_: &Toy, cell: IVec2, _: &mut PhaseContext<Toy>) -> Result<i32, PhaseError> {
            Ok(REQUEST_DISTANCE + (cell.x & 1))
        }
    }

    static SOURCE_RUNS: Mutex<Vec<IVec2>> = Mutex::new(Vec::new());
    static RESOLVES: Mutex<Vec<IVec2>> = Mutex::new(Vec::new());

    struct SourcePhase;

    impl Phase<Toy> for SourcePhase {
        type Output = i32;

        const RETAIN: usize = 64;

        fn run(_: &Toy, cell: IVec2, _: &mut PhaseContext<Toy>) -> Result<i32, PhaseError> {
            SOURCE_RUNS.lock().unwrap().push(cell);
            Ok(cell.x * SOURCE_STRIDE_X + cell.y * SOURCE_STRIDE_Z + 1)
        }
    }

    fn distance(cell: IVec2, context: &PhaseContext<Toy>) -> i32 {
        context.try_get::<BasePhase>(cell).map_or(REQUEST_DISTANCE, |value| *value)
    }

    fn target_of(cell: IVec2, context: &PhaseContext<Toy>) -> IVec2 {
        IVec2::new(cell.x + distance(cell, context), cell.y - 3)
    }

    fn value_at(cell: IVec2) -> i32 {
        let target = IVec2::new(cell.x + REQUEST_DISTANCE + (cell.x & 1), cell.y - 3);
        target.x * SOURCE_STRIDE_X + target.y * SOURCE_STRIDE_Z + 1
    }

    struct ReaderPhase;

    impl Phase<Toy> for ReaderPhase {
        type Output = Chunk;

        fn dependencies() -> Vec<Dependency<Toy>> {
            vec![Dependency::on::<BasePhase>(SAME_CELL)]
        }

        fn resolve(_: &Toy, cell: IVec2, context: &PhaseContext<Toy>) -> Vec<Request<Toy>> {
            RESOLVES.lock().unwrap().push(cell);
            let target = target_of(cell, context);
            vec![Request::of::<SourcePhase>(target), Request::of::<SourcePhase>(target)]
        }

        fn run(_: &Toy, cell: IVec2, context: &mut PhaseContext<Toy>) -> Result<Chunk, PhaseError> {
            let source = *context.get::<SourcePhase>(target_of(cell, context))?;
            let mut chunk = Chunk::new(cell.x, cell.y, -4, 24, 0, 1);
            chunk.set_block(0, 0, 0, 0, source);
            Ok(chunk)
        }
    }

    fn step<G: Generator>(graph: &mut Engine<G>, key: TaskKey) {
        let Some((handle, cell, context)) = graph.start(key).expect("a task failed") else { return };
        let (outcome, pushes) = Engine::<G>::execute(handle, &graph.generator, cell, context);
        if outcome.is_ok() {
            graph.store_mail(key, pushes);
        }
        graph.complete(key, outcome);
    }

    fn drive<G: Generator>(graph: &mut Engine<G>) -> Vec<(IVec2, i32)> {
        while let Some(key) = graph.pop_ready() {
            step(graph, key);
        }
        let finished = std::mem::take(&mut graph.finished);
        finished
            .into_iter()
            .map(|(cell, outcome)| {
                let output = outcome.expect("the terminal phase failed");
                let chunk = output.downcast::<Chunk>().expect("the terminal outputs a chunk");
                (cell, chunk.get_block(0, 0, 0, 0).expect("the chunk has a block"))
            })
            .collect()
    }

    fn run_until_ready<G: Generator>(graph: &mut Engine<G>, target: TaskKey) {
        while let Some(key) = graph.pop_ready() {
            if key == target {
                return;
            }
            step(graph, key);
        }
        panic!("the task never became ready");
    }

    fn collect(order: &[IVec2], tick_between: bool) -> Vec<(IVec2, i32)> {
        let mut graph = Engine::new(Toy);
        let mut finished = Vec::new();
        if tick_between {
            for &cell in order {
                graph.request(cell);
                finished.extend(drive(&mut graph));
            }
        } else {
            for &cell in order {
                graph.request(cell);
            }
            finished = drive(&mut graph);
        }
        finished.sort_unstable_by_key(|&(cell, _)| (cell.x, cell.y));
        finished
    }

    fn started_error<G: Generator>(graph: &mut Engine<G>) -> Arc<PhaseFailure> {
        let key = graph.pop_ready().expect("a task is ready");
        match graph.start(key) {
            Err(failure) => failure,
            Ok(_) => panic!("the request was not rejected"),
        }
    }

    #[test]
    fn a_phase_receives_the_cell_it_requests_after_reading_a_declared_dependency() {
        let cell = IVec2::new(20, 7);
        let target = IVec2::new(22, 4);
        let mut graph = Engine::new(Toy);
        graph.request(cell);
        assert_eq!(drive(&mut graph), vec![(cell, value_at(cell))]);
        assert_eq!(SOURCE_RUNS.lock().unwrap().iter().filter(|&&ran| ran == target).count(), 1);
    }

    #[test]
    fn the_requested_set_does_not_depend_on_request_order_or_scheduling() {
        let cells = [IVec2::new(100, 5), IVec2::new(101, 5), IVec2::new(102, 5)];
        let mut expected: Vec<(IVec2, i32)> = cells.iter().map(|&cell| (cell, value_at(cell))).collect();
        expected.sort_unstable_by_key(|&(cell, _)| (cell.x, cell.y));
        let mut reversed = cells;
        reversed.reverse();

        assert_eq!(collect(&cells, false), expected);
        assert_eq!(collect(&reversed, false), expected);
        assert_eq!(collect(&cells, true), expected);
    }

    #[test]
    fn duplicate_requests_share_one_node_and_one_computation() {
        let cell = IVec2::new(40, 9);
        let target = IVec2::new(42, 6);
        let mut graph = Engine::new(Toy);
        graph.request(cell);
        let reader = *graph.graph.index.get(&(PhaseId::of::<ReaderPhase>(), cell, 0)).expect("the reader exists");
        run_until_ready(&mut graph, reader);
        assert!(graph.start(reader).expect("the reader resolves").is_none(), "it waits for the cell it asked for");
        let dep_count = graph.graph.tasks[reader].deps.len();
        let source = *graph.graph.index.get(&(PhaseId::of::<SourcePhase>(), target, 0)).expect("the requested cell exists");
        let source_edges = graph.graph.tasks[reader].deps.iter().filter(|&&dep| dep == source).count();
        let remaining = match &graph.graph.tasks[reader].status {
            TaskStatus::Pending { remaining } => *remaining,
            _ => panic!("the reader waits"),
        };

        assert_eq!(dep_count, 2, "the declared dependency and one shared request");
        assert_eq!(source_edges, 1, "duplicates do not add a second edge");
        assert_eq!(remaining, 1, "duplicates do not count twice");
        assert_eq!(drive(&mut graph), vec![(cell, value_at(cell))]);
        assert_eq!(SOURCE_RUNS.lock().unwrap().iter().filter(|&&ran| ran == target).count(), 1);
        assert_eq!(
            RESOLVES.lock().unwrap().iter().filter(|&&resolved| resolved == cell).count(),
            2,
            "resolve ran again once its request finished and asked for nothing new"
        );
    }

    const CHAIN_LENGTH: i32 = 12;

    struct Chain;

    impl Generator for Chain {
        type Terminal = WalkerPhase;
    }

    struct LinkPhase;

    impl Phase<Chain> for LinkPhase {
        type Output = i32;

        fn run(_: &Chain, cell: IVec2, _: &mut PhaseContext<Chain>) -> Result<i32, PhaseError> {
            Ok(cell.x % 3 + 2)
        }
    }

    fn walk(cell: IVec2, context: &PhaseContext<Chain>) -> Vec<IVec2> {
        let mut path = vec![cell];
        let mut at = cell;
        while at.x - cell.x < CHAIN_LENGTH {
            let Some(step) = context.try_get::<LinkPhase>(at) else { break };
            at = IVec2::new(at.x + *step, at.y);
            path.push(at);
        }
        path
    }

    struct WalkerPhase;

    impl Phase<Chain> for WalkerPhase {
        type Output = Chunk;

        fn resolve(_: &Chain, cell: IVec2, context: &PhaseContext<Chain>) -> Vec<Request<Chain>> {
            walk(cell, context).into_iter().map(Request::of::<LinkPhase>).collect()
        }

        fn run(_: &Chain, cell: IVec2, context: &mut PhaseContext<Chain>) -> Result<Chunk, PhaseError> {
            let path = walk(cell, context);
            let mut chunk = Chunk::new(cell.x, cell.y, -4, 24, 0, 1);
            chunk.set_block(0, 0, 0, 0, path.len() as i32);
            Ok(chunk)
        }
    }

    #[test]
    fn a_phase_keeps_resolving_until_a_round_asks_for_nothing_new() {
        let cell = IVec2::new(30, 2);
        let mut expected = vec![cell];
        let mut at = cell;
        while at.x - cell.x < CHAIN_LENGTH {
            at = IVec2::new(at.x + at.x % 3 + 2, at.y);
            expected.push(at);
        }
        let mut graph = Engine::new(Chain);
        graph.request(cell);
        assert_eq!(drive(&mut graph), vec![(cell, expected.len() as i32)]);
    }

    fn tick_until_finished<G: Generator>(graph: &mut Engine<G>) -> Vec<(i32, i32, Result<Chunk, Arc<PhaseFailure>>)> {
        AsyncComputeTaskPool::get_or_init(TaskPool::default);
        let start = Instant::now();
        loop {
            let finished = WorldGenerator::tick(graph);
            if !finished.is_empty() {
                return finished;
            }
            assert!(start.elapsed() < Duration::from_secs(30), "generation never finished");
        }
    }

    #[test]
    fn resolving_on_the_pool_follows_requests_to_the_same_output() {
        let cell = IVec2::new(30, 2);
        let mut expected = vec![cell];
        let mut at = cell;
        while at.x - cell.x < CHAIN_LENGTH {
            at = IVec2::new(at.x + at.x % 3 + 2, at.y);
            expected.push(at);
        }
        let mut graph = Engine::new(Chain);
        graph.request(cell);
        let finished = tick_until_finished(&mut graph);
        assert_eq!(finished.len(), 1);
        let chunk = finished.into_iter().next().unwrap().2.expect("the walker finished");
        assert_eq!(chunk.get_block(0, 0, 0, 0), Some(expected.len() as i32));
        assert_eq!(graph.in_flight, 0);
    }

    #[test]
    fn a_request_cycle_still_fails_on_the_pool_path() {
        let mut graph = Engine::new(CycleToy);
        graph.request(IVec2::new(0, 0));
        let finished = tick_until_finished(&mut graph);
        let failure = finished.into_iter().next().unwrap().2.err().expect("the cycle fails");
        assert!(failure.to_string().contains("would make the graph depend on itself"), "{failure}");
    }

    struct SelfToy;

    impl Generator for SelfToy {
        type Terminal = SelfPhase;
    }

    struct SelfPhase;

    impl Phase<SelfToy> for SelfPhase {
        type Output = Chunk;

        fn resolve(_: &SelfToy, cell: IVec2, _: &PhaseContext<SelfToy>) -> Vec<Request<SelfToy>> {
            vec![Request::of::<SelfPhase>(cell)]
        }

        fn run(_: &SelfToy, _: IVec2, _: &mut PhaseContext<SelfToy>) -> Result<Chunk, PhaseError> {
            unreachable!("a self request is rejected before the phase runs")
        }
    }

    struct CycleToy;

    impl Generator for CycleToy {
        type Terminal = CycleA;
    }

    struct CycleA;

    impl Phase<CycleToy> for CycleA {
        type Output = Chunk;

        fn resolve(_: &CycleToy, cell: IVec2, _: &PhaseContext<CycleToy>) -> Vec<Request<CycleToy>> {
            vec![Request::of::<CycleB>(IVec2::new(cell.x + 1, cell.y))]
        }

        fn run(_: &CycleToy, _: IVec2, _: &mut PhaseContext<CycleToy>) -> Result<Chunk, PhaseError> {
            unreachable!("a cycle is rejected before the phase runs")
        }
    }

    struct CycleB;

    impl Phase<CycleToy> for CycleB {
        type Output = Chunk;

        fn resolve(_: &CycleToy, cell: IVec2, _: &PhaseContext<CycleToy>) -> Vec<Request<CycleToy>> {
            vec![Request::of::<CycleA>(IVec2::new(cell.x - 1, cell.y))]
        }

        fn run(_: &CycleToy, _: IVec2, _: &mut PhaseContext<CycleToy>) -> Result<Chunk, PhaseError> {
            unreachable!("a cycle is rejected before the phase runs")
        }
    }

    #[test]
    fn a_self_request_fails_instead_of_hanging() {
        let mut graph = Engine::new(SelfToy);
        graph.request(IVec2::new(0, 0));
        let failure = started_error(&mut graph);
        assert!(matches!(failure.error, PhaseError::RequestCycle { .. }), "{failure}");
        assert_eq!(graph.pending_dispatch.len(), 0, "nothing was left ready");
    }

    #[test]
    fn a_two_node_cycle_fails_instead_of_hanging() {
        let mut graph = Engine::new(CycleToy);
        graph.request(IVec2::new(0, 0));
        let first = graph.pop_ready().expect("the first task is ready");
        assert!(graph.start(first).expect("the first task resolves").is_none(), "it waits for the task it asked for");
        let failure = started_error(&mut graph);
        assert!(matches!(failure.error, PhaseError::RequestCycle { .. }), "{failure}");
    }

    struct PlainToy;

    impl Generator for PlainToy {
        type Terminal = PlainPhase;
    }

    struct PlainBase;

    impl Phase<PlainToy> for PlainBase {
        type Output = i32;

        fn run(_: &PlainToy, cell: IVec2, _: &mut PhaseContext<PlainToy>) -> Result<i32, PhaseError> {
            Ok(cell.x - cell.y)
        }
    }

    struct PlainPhase;

    impl Phase<PlainToy> for PlainPhase {
        type Output = Chunk;

        fn dependencies() -> Vec<Dependency<PlainToy>> {
            vec![Dependency::on::<PlainBase>(SAME_CELL)]
        }

        fn run(_: &PlainToy, cell: IVec2, context: &mut PhaseContext<PlainToy>) -> Result<Chunk, PhaseError> {
            let value = *context.get::<PlainBase>(cell)?;
            let mut chunk = Chunk::new(cell.x, cell.y, -4, 24, 0, 1);
            chunk.set_block(0, 0, 0, 0, value);
            Ok(chunk)
        }
    }

    #[test]
    fn a_phase_without_requests_behaves_as_before() {
        let cell = IVec2::new(9, 4);
        let mut graph = Engine::new(PlainToy);
        graph.request(cell);
        assert_eq!(graph.graph.tasks.len(), 2, "the declared dependency is all it builds");
        assert_eq!(drive(&mut graph), vec![(cell, 5)]);
        assert_eq!(graph.started.get("PlainPhase"), Some(&1));
        assert_eq!(graph.graph.tasks.len(), 0, "the finished pipeline is released like any other");
    }

    const FOCUS_SIDE: i32 = 10;
    const FOCUS_STRIDE: i32 = 40;
    const FOCUS_RADIUS: i32 = 11;
    const FOCUS_BLOCK: i32 = 2;
    const OUTSIDE_OFFSET: i32 = 20;

    struct FocusToy;

    impl Generator for FocusToy {
        type Terminal = FocusReader;
    }

    struct FocusSource;

    impl Phase<FocusToy> for FocusSource {
        type Output = i32;

        const RETAIN: usize = 64;

        fn run(_: &FocusToy, cell: IVec2, _: &mut PhaseContext<FocusToy>) -> Result<i32, PhaseError> {
            Ok(cell.x * SOURCE_STRIDE_X + cell.y * SOURCE_STRIDE_Z + 1)
        }
    }

    struct FocusBase;

    impl Phase<FocusToy> for FocusBase {
        type Output = ();

        fn run(_: &FocusToy, _: IVec2, _: &mut PhaseContext<FocusToy>) -> Result<(), PhaseError> {
            Ok(())
        }
    }

    fn outside(cell: IVec2) -> IVec2 {
        IVec2::new(cell.x + OUTSIDE_OFFSET, cell.y)
    }

    fn outside_value(cell: IVec2) -> i32 {
        outside(cell).x * SOURCE_STRIDE_X + outside(cell).y * SOURCE_STRIDE_Z + 1
    }

    struct FocusReader;

    impl Phase<FocusToy> for FocusReader {
        type Output = Chunk;

        fn dependencies() -> Vec<Dependency<FocusToy>> {
            vec![Dependency::on::<FocusBase>(SAME_CELL)]
        }

        fn resolve(_: &FocusToy, cell: IVec2, _: &PhaseContext<FocusToy>) -> Vec<Request<FocusToy>> {
            vec![Request::of::<FocusSource>(outside(cell))]
        }

        fn run(_: &FocusToy, cell: IVec2, context: &mut PhaseContext<FocusToy>) -> Result<Chunk, PhaseError> {
            let source = *context.get::<FocusSource>(outside(cell))?;
            let mut chunk = Chunk::new(cell.x, cell.y, -4, 24, 0, 1);
            chunk.set_block(0, 0, 0, 0, source);
            Ok(chunk)
        }
    }

    fn foci() -> Vec<(i32, i32)> {
        (0..FOCUS_SIDE).flat_map(|x| (0..FOCUS_SIDE).map(move |z| (x * FOCUS_STRIDE, z * FOCUS_STRIDE))).collect()
    }

    fn focus_block() -> Vec<IVec2> {
        let mut wanted = Vec::new();
        for (x, z) in foci() {
            wanted.push(IVec2::new(x, z));
            for dx in -FOCUS_BLOCK..=FOCUS_BLOCK {
                for dz in -FOCUS_BLOCK..=FOCUS_BLOCK {
                    if (dx, dz) != (0, 0) {
                        wanted.push(IVec2::new(x + dx, z + dz));
                    }
                }
            }
        }
        wanted
    }

    #[test]
    fn a_node_demanded_outside_every_focus_runs_before_the_nearer_ready_work() {
        let mut graph = Engine::new(FocusToy);
        graph.set_focus(&foci(), FOCUS_RADIUS);
        let wanted = focus_block();
        for &cell in &wanted {
            graph.request(cell);
        }

        let target = outside(wanted[0]);
        let tasks = graph.graph.tasks.len();
        let mut steps = 0;
        while let Some(key) = graph.pop_ready() {
            let task = &graph.graph.tasks[key];
            let demanded = graph.graph.phases.id(task) == PhaseId::of::<FocusSource>() && task.cell == target;
            if demanded {
                step(&mut graph, key);
                break;
            }
            steps += 1;
            step(&mut graph, key);
        }
        assert!(steps * 4 < tasks, "{steps} of {tasks} tasks ran before the demanded task, so it waited for unrelated work");

        let finished = drive(&mut graph);
        let value = finished.iter().find(|(cell, _)| *cell == wanted[0]).map(|&(_, value)| value);
        assert_eq!(value, Some(outside_value(wanted[0])));
        assert_eq!(graph.started.get("FocusSource"), Some(&(wanted.len() as u64)), "every demanded cell ran exactly once");
    }

    #[test]
    fn the_requested_output_does_not_depend_on_the_order_the_demands_are_made_in() {
        let wanted = focus_block();
        let mut reversed = wanted.clone();
        reversed.reverse();
        let expected: Vec<(IVec2, i32)> = {
            let mut values: Vec<(IVec2, i32)> = wanted.iter().map(|&cell| (cell, outside_value(cell))).collect();
            values.sort_unstable_by_key(|&(cell, _)| (cell.x, cell.y));
            values
        };

        let mut first = Vec::new();
        let mut last = Vec::new();
        for (index, order) in [&wanted, &reversed].into_iter().enumerate() {
            let mut graph = Engine::new(FocusToy);
            graph.set_focus(&foci(), FOCUS_RADIUS);
            for &cell in order.iter() {
                graph.request(cell);
            }
            let mut finished = drive(&mut graph);
            finished.sort_unstable_by_key(|&(cell, _)| (cell.x, cell.y));
            if index == 0 {
                first = finished
            } else {
                last = finished
            }
            assert_eq!(graph.started.get("FocusSource"), Some(&(wanted.len() as u64)), "every demand ran once in this order");
        }
        assert_eq!(first, expected);
        assert_eq!(last, expected);
    }

    struct SharedToy;

    impl Generator for SharedToy {
        type Terminal = SharedReader;
    }

    const SHARED: (i32, i32) = (30, -7);

    fn shared() -> IVec2 {
        IVec2::new(SHARED.0, SHARED.1)
    }

    fn shared_value() -> i32 {
        SHARED.0 * SOURCE_STRIDE_X + SHARED.1 * SOURCE_STRIDE_Z + 1
    }

    static SHARED_RUNS: Mutex<u32> = Mutex::new(0);

    struct SharedSource;

    impl Phase<SharedToy> for SharedSource {
        type Output = i32;

        const RETAIN: usize = 8;

        fn run(_: &SharedToy, _: IVec2, _: &mut PhaseContext<SharedToy>) -> Result<i32, PhaseError> {
            *SHARED_RUNS.lock().unwrap() += 1;
            Ok(shared_value())
        }
    }

    struct SharedReader;

    impl Phase<SharedToy> for SharedReader {
        type Output = Chunk;

        fn resolve(_: &SharedToy, _: IVec2, _: &PhaseContext<SharedToy>) -> Vec<Request<SharedToy>> {
            vec![Request::of::<SharedSource>(shared())]
        }

        fn run(_: &SharedToy, cell: IVec2, context: &mut PhaseContext<SharedToy>) -> Result<Chunk, PhaseError> {
            let source = *context.get::<SharedSource>(shared())?;
            let mut chunk = Chunk::new(cell.x, cell.y, -4, 24, 0, 1);
            chunk.set_block(0, 0, 0, 0, source);
            Ok(chunk)
        }
    }

    #[test]
    fn two_cells_that_demand_the_same_key_share_one_run() {
        let mut graph = Engine::new(SharedToy);
        let cells = [IVec2::new(0, 0), IVec2::new(1, 0)];
        for &cell in &cells {
            graph.request(cell);
        }
        let finished = drive(&mut graph);
        assert_eq!(finished.len(), cells.len());
        assert!(finished.iter().all(|&(_, value)| value == shared_value()));
        assert_eq!(*SHARED_RUNS.lock().unwrap(), 1, "the shared demand ran once");
        assert_eq!(graph.graph.index.len(), 1, "one task serves both cells");
    }

    #[test]
    fn demanded_nodes_are_retained_up_to_the_budget_and_rebuilt_after_eviction() {
        let mut graph = Engine::new(Toy);
        graph.set_focus(&[(0, 0), (40, 0)], 3);
        graph.set_retention_budget(0.1);
        let cells: Vec<IVec2> = (0..7).flat_map(|x| (0..7).map(move |z| IVec2::new(x - 3, z - 3))).collect();
        for &cell in &cells {
            graph.request(cell);
        }
        let finished = drive(&mut graph);
        let cap = graph.retention_cap();
        assert!(cap < cells.len().saturating_mul(8), "the cap has to bite for this to test anything");
        assert!(graph.retained_count() <= cap, "kept {} against a cap of {cap}", graph.retained_count());
        let retained = graph.retention_sizes().into_iter().find(|size| size.phase == "SourcePhase" && size.footprint_cells == 1);
        assert!(retained.is_some_and(|size| size.retained > 0), "demanded tasks count against the budget");
        let runs = |cell: IVec2| SOURCE_RUNS.lock().unwrap().iter().filter(|&&ran| ran == cell).count();
        let rebuilt = cells[0];
        let target = IVec2::new(rebuilt.x + REQUEST_DISTANCE + (rebuilt.x & 1), rebuilt.y - 3);
        let before = runs(target);

        for &cell in cells.iter().rev() {
            graph.request(cell);
        }
        let mut again = drive(&mut graph);
        let mut first = finished;
        first.sort_unstable_by_key(|&(cell, _)| (cell.x, cell.y));
        again.sort_unstable_by_key(|&(cell, _)| (cell.x, cell.y));
        assert_eq!(again, first, "a rebuilt demand produces the same values");
        assert!(runs(target) > before, "the evicted demand is rebuilt");
        assert!(graph.retained_count() <= cap);
    }
}
