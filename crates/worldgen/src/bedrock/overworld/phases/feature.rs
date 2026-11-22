use super::super::OverworldGenerator;
use super::chain;
use super::structure::{structured, structured_baseline};
use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::chunk::region::{Changes, merge};
use crate::shared::decoration_class::{class_of, is_class};
use crate::shared::{CHUNK_NEIGHBORHOOD, into_array};
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Mailing, Phase, PhaseContext, Request};
use glam::IVec2;

pub(super) const OWNERS: &[(i32, i32)] = &[(-1, -1), (0, -1), (1, -1), (-1, 0), (0, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

const ORTHOGONAL_X: &[(i32, i32)] = &[(-1, 0), (1, 0)];

const ORTHOGONAL_Z: &[(i32, i32)] = &[(0, -1), (0, 1)];

const DIAGONALS: &[(i32, i32)] = &[(-1, -1), (-1, 1), (1, -1), (1, 1)];

trait Earlier: Mailing<OverworldGenerator, Mail = Changes> {
    fn earlier(context: &PhaseContext<OverworldGenerator>, target: IVec2) -> Vec<(IVec2, &Changes)>;
}

fn decorate_class<P: Earlier>(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<(), PhaseError> {
    let mut chunks = Vec::with_capacity(9);
    for &(dx, dz) in CHUNK_NEIGHBORHOOD {
        let source = cell + IVec2::new(dx, dz);
        let baseline = structured_baseline(context, source)?;
        let mut chunk = (*baseline).clone();
        {
            let mut edit = chunk.edit();
            let mut inherited = Vec::new();
            let mail: Vec<(IVec2, &Changes)> = P::earlier(context, source);
            for (_, changes) in &mail {
                if (dx, dz) == (0, 0) {
                    inherited.extend_from_slice(&changes.routed_ticks);
                }
            }
            merge_phases(&mail, &generator.blocks, &mut edit);
            edit.set_inherited_ticks(inherited);
        }
        chunks.push(chunk);
    }
    let chunks: [ProtoChunk; 9] = into_array(chunks);
    let slots = generator.decorate(cell.x, cell.y, chunks, &*context)?;
    for (changes, &(dx, dz)) in slots.into_iter().zip(CHUNK_NEIGHBORHOOD) {
        context.send::<P>(cell + IVec2::new(dx, dz), changes);
    }
    Ok(())
}

pub struct FeaturePhase0;

impl Phase<OverworldGenerator> for FeaturePhase0 {
    type Output = ();

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![structured(CHUNK_NEIGHBORHOOD).if_dependent(is_class::<0>)]
    }

    fn resolve(generator: &OverworldGenerator, cell: IVec2, context: &PhaseContext<OverworldGenerator>) -> Vec<Request<OverworldGenerator>> {
        if !is_class::<0>(cell) {
            return Vec::new();
        }
        generator.decoration_requests((cell.x, cell.y), context)
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<(), PhaseError> {
        if !is_class::<0>(cell) {
            return Ok(());
        }
        decorate_class::<Self>(generator, cell, context)
    }
}

impl Mailing<OverworldGenerator> for FeaturePhase0 {
    type Mail = Changes;
}

impl Earlier for FeaturePhase0 {
    fn earlier(_inputs: &PhaseContext<OverworldGenerator>, _target: IVec2) -> Vec<(IVec2, &Changes)> {
        Vec::new()
    }
}

pub struct FeaturePhase1;

impl Phase<OverworldGenerator> for FeaturePhase1 {
    type Output = ();

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![
            structured(CHUNK_NEIGHBORHOOD).if_dependent(is_class::<1>),
            chain::<FeaturePhase0>(ORTHOGONAL_X).with_mail(CHUNK_NEIGHBORHOOD).if_dependent(is_class::<1>),
        ]
    }

    fn resolve(generator: &OverworldGenerator, cell: IVec2, context: &PhaseContext<OverworldGenerator>) -> Vec<Request<OverworldGenerator>> {
        if !is_class::<1>(cell) {
            return Vec::new();
        }
        generator.decoration_requests((cell.x, cell.y), context)
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<(), PhaseError> {
        if !is_class::<1>(cell) {
            return Ok(());
        }
        decorate_class::<Self>(generator, cell, context)
    }
}

impl Mailing<OverworldGenerator> for FeaturePhase1 {
    type Mail = Changes;
}

impl Earlier for FeaturePhase1 {
    fn earlier(context: &PhaseContext<OverworldGenerator>, target: IVec2) -> Vec<(IVec2, &Changes)> {
        context.mail::<FeaturePhase0>(target).collect()
    }
}

pub struct FeaturePhase2;

impl Phase<OverworldGenerator> for FeaturePhase2 {
    type Output = ();

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![
            structured(CHUNK_NEIGHBORHOOD).if_dependent(is_class::<2>),
            chain::<FeaturePhase0>(DIAGONALS).with_mail(CHUNK_NEIGHBORHOOD).if_dependent(is_class::<2>),
            chain::<FeaturePhase1>(ORTHOGONAL_Z).with_mail(CHUNK_NEIGHBORHOOD).if_dependent(is_class::<2>),
        ]
    }

    fn resolve(generator: &OverworldGenerator, cell: IVec2, context: &PhaseContext<OverworldGenerator>) -> Vec<Request<OverworldGenerator>> {
        if !is_class::<2>(cell) {
            return Vec::new();
        }
        generator.decoration_requests((cell.x, cell.y), context)
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<(), PhaseError> {
        if !is_class::<2>(cell) {
            return Ok(());
        }
        decorate_class::<Self>(generator, cell, context)
    }
}

impl Mailing<OverworldGenerator> for FeaturePhase2 {
    type Mail = Changes;
}

impl Earlier for FeaturePhase2 {
    fn earlier(context: &PhaseContext<OverworldGenerator>, target: IVec2) -> Vec<(IVec2, &Changes)> {
        let mut mail: Vec<(IVec2, &Changes)> = context.mail::<FeaturePhase0>(target).collect();
        mail.extend(context.mail::<FeaturePhase1>(target));
        mail
    }
}

pub struct FeaturePhase3;

impl Phase<OverworldGenerator> for FeaturePhase3 {
    type Output = ();

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<OverworldGenerator>> {
        vec![
            structured(CHUNK_NEIGHBORHOOD).if_dependent(is_class::<3>),
            chain::<FeaturePhase0>(ORTHOGONAL_Z).with_mail(CHUNK_NEIGHBORHOOD).if_dependent(is_class::<3>),
            chain::<FeaturePhase1>(DIAGONALS).with_mail(CHUNK_NEIGHBORHOOD).if_dependent(is_class::<3>),
            chain::<FeaturePhase2>(ORTHOGONAL_X).with_mail(CHUNK_NEIGHBORHOOD).if_dependent(is_class::<3>),
        ]
    }

    fn resolve(generator: &OverworldGenerator, cell: IVec2, context: &PhaseContext<OverworldGenerator>) -> Vec<Request<OverworldGenerator>> {
        if !is_class::<3>(cell) {
            return Vec::new();
        }
        generator.decoration_requests((cell.x, cell.y), context)
    }

    fn run(generator: &OverworldGenerator, cell: IVec2, context: &mut PhaseContext<OverworldGenerator>) -> Result<(), PhaseError> {
        if !is_class::<3>(cell) {
            return Ok(());
        }
        decorate_class::<Self>(generator, cell, context)
    }
}

impl Mailing<OverworldGenerator> for FeaturePhase3 {
    type Mail = Changes;
}

impl Earlier for FeaturePhase3 {
    fn earlier(context: &PhaseContext<OverworldGenerator>, target: IVec2) -> Vec<(IVec2, &Changes)> {
        let mut mail: Vec<(IVec2, &Changes)> = context.mail::<FeaturePhase0>(target).collect();
        mail.extend(context.mail::<FeaturePhase1>(target));
        mail.extend(context.mail::<FeaturePhase2>(target));
        mail
    }
}

pub(super) fn merge_phases(mail: &[(IVec2, &Changes)], blocks: &Blocks, target: &mut ProtoChunk) {
    let mut start = 0;
    while start < mail.len() {
        let phase = class_of((mail[start].0.x, mail[start].0.y));
        let mut end = start + 1;
        while end < mail.len() && class_of((mail[end].0.x, mail[end].0.y)) == phase {
            end += 1;
        }
        let group: Vec<&Changes> = mail[start..end].iter().map(|(_, changes)| *changes).collect();
        merge(target, &group, blocks);
        start = end;
    }
}
