use std::any::Any;
use std::sync::Arc;

use super::builder::{Built, StructureContext};
use super::{MissingInput, Probed, StartProbe, StructureKind, Structures};

#[derive(Default)]
pub struct Starts {
    kinds: Vec<StructureKind>,
    built: Vec<(StructureKind, Built)>,
}

impl Starts {
    pub fn has(&self, kind: StructureKind) -> bool {
        self.kinds.contains(&kind)
    }

    pub(super) fn get<T: Any + Send + Sync>(&self, kind: StructureKind) -> Option<Arc<T>> {
        let (_, built) = self.built.iter().find(|(known, _)| *known == kind)?;
        built.clone().downcast::<T>().ok()
    }
}

impl Structures {
    pub(super) fn starts_at(&self, probe: &dyn StartProbe, cell: (i32, i32)) -> Probed<Arc<Starts>> {
        probe.starts_at(cell).ok_or(MissingInput { what: "structure starts", cell })
    }

    pub fn build_starts(&self, probe: &dyn StartProbe, cell: (i32, i32)) -> Probed<Starts> {
        let context = StructureContext { structures: self, probe };
        let mut starts = Starts::default();
        for (builder, &kind) in self.builders.iter().zip(&self.kinds) {
            if !self.is_start(probe, kind, cell)? {
                continue;
            }
            starts.kinds.push(kind);
            if let Some(built) = builder.build(&context, kind, cell)? {
                starts.built.push((kind, built));
            }
        }
        Ok(starts)
    }
}
