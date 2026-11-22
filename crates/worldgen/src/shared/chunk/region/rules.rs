use super::Region;
use crate::shared::blocks::BlockId;
use glam::IVec3;

pub(super) const NO_RULE: u16 = 0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RuleKind {
    None = 0,
    Replaceable = 1,
    Ore = 2,
    Vegetation = 3,
    AnyExceptProtected = 4,
}

impl Region<'_> {
    pub fn begin_feature(&mut self) {
        self.rule = NO_RULE;
    }

    pub fn set_rule(&mut self, kinds: &[u16], kind: RuleKind) {
        self.rule = self.register_rule(kinds, kind);
    }

    pub fn set_any_rule(&mut self) {
        self.rule = self.register_rule(&[], RuleKind::AnyExceptProtected);
    }

    fn register_rule(&mut self, kinds: &[u16], kind: RuleKind) -> u16 {
        match self.rules.iter().position(|known| &*known.0 == kinds && known.1 == kind) {
            Some(index) => index as u16 + 1,
            None => {
                self.rules.push((kinds.to_vec().into_boxed_slice(), kind));
                self.rules.len() as u16
            }
        }
    }

    pub fn replace(&mut self, pos: IVec3, block: BlockId, may_replace: &[u16]) -> bool {
        let rule = self.register_rule(may_replace, RuleKind::Ore);
        self.set_with_rule(pos, block, rule)
    }
}
