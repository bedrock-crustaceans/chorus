use super::entry::Connector;
use super::grow::Part;
use crate::shared::structure::Probed;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Junction {
    pub x: i32,
    pub ground: i32,
    pub z: i32,
}

pub struct Joint<'a> {
    pub source: &'a Part,
    pub source_rigid: bool,
    pub jigsaw: &'a Connector<'a>,
    pub candidate: &'a Connector<'a>,
    pub child_rigid: bool,
}

impl Joint<'_> {
    pub fn link(&self, child: &mut Part, surface: impl FnOnce() -> Probed<i32>) -> Probed<Junction> {
        let (source, jigsaw, candidate) = (self.source, self.jigsaw, self.candidate);
        let relative = jigsaw.pos.1 - source.bounds.y0;
        let step = jigsaw.front.offset();
        let offset = relative - candidate.pos.1 + step.1;
        child.ground = if self.child_rigid { source.ground - offset } else { 1 };
        let joint_y = if self.source_rigid {
            jigsaw.pos.1
        } else if self.child_rigid {
            child.bounds.y0 + candidate.pos.1
        } else {
            surface()? + offset / 2
        };
        child.junctions.push(Junction {
            x: jigsaw.pos.0,
            ground: joint_y - candidate.pos.1 + child.ground,
            z: jigsaw.pos.2,
        });
        Ok(Junction {
            x: jigsaw.pos.0 + step.0,
            ground: joint_y - relative + source.ground,
            z: jigsaw.pos.2 + step.2,
        })
    }
}
