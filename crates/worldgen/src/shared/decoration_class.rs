use std::marker::PhantomData;
use std::sync::Arc;

use chorus_level::generator::dimension::Generator;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext};
use glam::IVec2;

use super::into_array;

const CLASS_BY_PARITY: [u8; 4] = [0, 1, 3, 2];

pub fn class_of(cell: (i32, i32)) -> u8 {
    CLASS_BY_PARITY[((cell.0 & 1) | (cell.1 & 1) << 1) as usize]
}

pub fn is_class<const CLASS: u8>(cell: IVec2) -> bool {
    class_of((cell.x, cell.y)) == CLASS
}

pub fn earlier_offsets(reader: u8, earlier: u8) -> &'static [(i32, i32)] {
    match (reader, earlier) {
        (1, 0) => &[(-1, -2), (-1, 0), (-1, 2), (1, -2), (1, 0), (1, 2)],
        (2, 0) => &[(-1, -1), (-1, 1), (1, -1), (1, 1)],
        (2, 1) => &[(-2, -1), (0, -1), (2, -1), (-2, 1), (0, 1), (2, 1)],
        (3, 0) => &[(-2, -1), (0, -1), (2, -1), (-2, 1), (0, 1), (2, 1)],
        (3, 1) => &[(-1, -1), (-1, 1), (1, -1), (1, 1)],
        (3, 2) => &[(-1, -2), (-1, 0), (-1, 2), (1, -2), (1, 0), (1, 2)],
        _ => &[],
    }
}

pub trait Decorates: Generator {
    type Decoration: Send + Sync + 'static;

    const RETAIN: usize;

    fn inputs() -> Vec<Dependency<Self>>;

    fn decorate(&self, cell: IVec2, context: &PhaseContext<Self>, earlier: &[((i32, i32), &Self::Decoration)]) -> Result<Self::Decoration, PhaseError>;
}

pub struct DecoratePhase<G, const CLASS: u8>(PhantomData<fn() -> G>);

impl<G: Decorates, const CLASS: u8> Phase<G> for DecoratePhase<G, CLASS> {
    type Output = Arc<G::Decoration>;

    const RETAIN: usize = G::RETAIN;

    fn dependencies() -> Vec<Dependency<G>> {
        let mut dependencies = G::inputs();
        if CLASS > 0 {
            dependencies.push(Dependency::on::<DecoratePhase<G, 0>>(earlier_offsets(CLASS, 0)));
        }
        if CLASS > 1 {
            dependencies.push(Dependency::on::<DecoratePhase<G, 1>>(earlier_offsets(CLASS, 1)));
        }
        if CLASS > 2 {
            dependencies.push(Dependency::on::<DecoratePhase<G, 2>>(earlier_offsets(CLASS, 2)));
        }
        dependencies
    }

    fn run(generator: &G, cell: IVec2, context: &mut PhaseContext<G>) -> Result<Arc<G::Decoration>, PhaseError> {
        let mut earlier = Vec::new();
        if CLASS > 0 {
            gather_earlier::<G, CLASS, 0>(context, cell, &mut earlier)?;
        }
        if CLASS > 1 {
            gather_earlier::<G, CLASS, 1>(context, cell, &mut earlier)?;
        }
        if CLASS > 2 {
            gather_earlier::<G, CLASS, 2>(context, cell, &mut earlier)?;
        }
        earlier.sort_by_key(|(at, _)| (class_of(*at), at.0, at.1));
        let earlier: Vec<((i32, i32), &G::Decoration)> = earlier.iter().map(|(at, decoration)| (*at, &**decoration)).collect();
        generator.decorate(cell, context, &earlier).map(Arc::new)
    }
}

fn gather_earlier<G: Decorates, const CLASS: u8, const EARLIER: u8>(context: &PhaseContext<G>, cell: IVec2, found: &mut Vec<((i32, i32), Arc<G::Decoration>)>) -> Result<(), PhaseError> {
    for &(dx, dz) in earlier_offsets(CLASS, EARLIER) {
        let at = cell + IVec2::new(dx, dz);
        found.push(((at.x, at.y), (*context.get::<DecoratePhase<G, EARLIER>>(at)?).clone()));
    }
    Ok(())
}

pub fn on_decorations<G: Decorates>(offsets: &'static [(i32, i32)]) -> Vec<Dependency<G>> {
    vec![
        Dependency::on::<DecoratePhase<G, 0>>(offsets).if_dependency(is_class::<0>),
        Dependency::on::<DecoratePhase<G, 1>>(offsets).if_dependency(is_class::<1>),
        Dependency::on::<DecoratePhase<G, 2>>(offsets).if_dependency(is_class::<2>),
        Dependency::on::<DecoratePhase<G, 3>>(offsets).if_dependency(is_class::<3>),
    ]
}

pub fn decorations_around<G: Decorates, const N: usize>(context: &PhaseContext<G>, cell: IVec2, offsets: &[(i32, i32)]) -> Result<[Arc<G::Decoration>; N], PhaseError> {
    let mut around = Vec::with_capacity(offsets.len());
    for &(dx, dz) in offsets {
        around.push(decoration_at(context, cell + IVec2::new(dx, dz))?);
    }
    Ok(into_array(around))
}

fn decoration_at<G: Decorates>(context: &PhaseContext<G>, at: IVec2) -> Result<Arc<G::Decoration>, PhaseError> {
    Ok(match class_of((at.x, at.y)) {
        0 => (*context.get::<DecoratePhase<G, 0>>(at)?).clone(),
        1 => (*context.get::<DecoratePhase<G, 1>>(at)?).clone(),
        2 => (*context.get::<DecoratePhase<G, 2>>(at)?).clone(),
        _ => (*context.get::<DecoratePhase<G, 3>>(at)?).clone(),
    })
}

pub fn neighborhood<G: Generator, P: Phase<G>>(context: &PhaseContext<G>, cell: IVec2) -> Result<[P::Output; 9], PhaseError>
where
    P::Output: Clone,
{
    let mut found = Vec::with_capacity(super::CHUNK_NEIGHBORHOOD.len());
    for &(dx, dz) in super::CHUNK_NEIGHBORHOOD {
        found.push((*context.get::<P>(cell + IVec2::new(dx, dz))?).clone());
    }
    Ok(into_array(found))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_parities_map_to_distinct_classes() {
        let classes: Vec<u8> = [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().map(class_of).collect();
        assert_eq!(classes, vec![0, 1, 3, 2]);
        assert_eq!(class_of((-1, -2)), class_of((1, 0)));
        let cell = IVec2::new(5, -3);
        let count = [is_class::<0>(cell), is_class::<1>(cell), is_class::<2>(cell), is_class::<3>(cell)]
            .into_iter()
            .filter(|&member| member)
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn earlier_offsets_land_on_cells_of_the_earlier_class() {
        for (reader, earlier) in [(1, 0), (2, 0), (2, 1), (3, 0), (3, 1), (3, 2)] {
            for x in 0..2 {
                for z in 0..2 {
                    if class_of((x, z)) != reader {
                        continue;
                    }
                    for &(dx, dz) in earlier_offsets(reader, earlier) {
                        assert_eq!(class_of((x + dx, z + dz)), earlier, "reader {reader} at ({x}, {z}) offset ({dx}, {dz})");
                    }
                }
            }
        }
    }
}
