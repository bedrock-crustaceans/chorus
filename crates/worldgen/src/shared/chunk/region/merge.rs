use super::changes::{Changes, Replay};
use super::rules::{NO_RULE, RuleKind};
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::tags::Tag;

fn position_key(x: u8, y: i16, z: u8) -> u32 {
    (((y + 64) as u32) << 8) | ((z as u32) << 4) | x as u32
}

fn rule_allows(changes: &Changes, rule: u16, block: BlockId, blocks: &Blocks) -> bool {
    if rule == NO_RULE {
        return false;
    }
    let (kinds, kind) = &changes.rules[rule as usize - 1];
    match kind {
        RuleKind::AnyExceptProtected => !blocks.is(block, Tag::FeaturesCannotReplace),
        _ => kinds.iter().any(|&kind| blocks.is_kind(block, kind)),
    }
}

fn source_legal(changes: &Changes, replay: &Replay, first: usize, baseline: BlockId, blocks: &Blocks) -> bool {
    baseline == replay.palette[replay.seen.get(first) as usize] || rule_allows(changes, replay.rules.get(first), baseline, blocks)
}

pub fn merge(target: &mut ProtoChunk, mail: &[&Changes], blocks: &Blocks) {
    for changes in mail {
        for entity in &changes.entities {
            target.add_entity(entity.clone());
        }
        for (x, y, z) in changes.marks.iter() {
            target.mark_post_process(x, y, z);
        }
    }
    let capacity: usize = mail.iter().map(|changes| changes.replay.as_ref().map_or(0, |replay| replay.positions.len())).sum();
    let mut entries: Vec<(u32, u32, u16)> = Vec::with_capacity(capacity);
    for (mail_index, changes) in mail.iter().enumerate() {
        let Some(replay) = &changes.replay else { continue };
        entries.extend(replay.positions.iter().enumerate().map(|(index, &(x, y, z))| (position_key(x, y, z), mail_index as u32, index as u16)));
    }
    entries.sort_unstable();
    let mut start = 0;
    while start < entries.len() {
        let key = entries[start].0;
        let mut end = start + 1;
        while end < entries.len() && entries[end].0 == key {
            end += 1;
        }
        merge_cell(key, &entries[start..end], mail, blocks, target);
        start = end;
    }
}

fn merge_cell(key: u32, run: &[(u32, u32, u16)], mail: &[&Changes], blocks: &Blocks, target: &mut ProtoChunk) {
    let (x, y, z) = ((key & 15) as i32, ((key >> 8) & 511) as i32 - 64, ((key >> 4) & 15) as i32);
    let baseline_block = target.get(x, y, z);
    let mut overlaid = baseline_block;
    let mut replayed: Option<(&Changes, usize)> = None;
    for &(_, mail_index, write_index) in run {
        let changes = mail[mail_index as usize];
        let replay = changes.replay.as_ref().expect("mailed write");
        let index = write_index as usize;
        if replay.palette[replay.seen.get(index) as usize] == overlaid {
            overlaid = replay.palette[replay.blocks.get(index) as usize];
            replayed = Some((changes, index));
        }
    }
    let mut winner: Option<(&Changes, usize)> = None;
    let mut start = 0;
    while start < run.len() {
        let mail_index = run[start].1;
        let mut end = start + 1;
        while end < run.len() && run[end].1 == mail_index {
            end += 1;
        }
        let changes = mail[mail_index as usize];
        let replay = changes.replay.as_ref().expect("mailed write");
        let first = run[start].2 as usize;
        let last = run[end - 1].2 as usize;
        if source_legal(changes, replay, first, baseline_block, blocks) {
            winner = Some((changes, last));
        }
        start = end;
    }
    let merged = winner.map(|(changes, index)| {
        let replay = changes.replay.as_ref().expect("mailed write");
        (changes, replay.palette[replay.blocks.get(index) as usize], replay.rules.get(index))
    });
    let decided = match (replayed, merged) {
        (Some((old_changes, old_index)), Some((new_changes, new_block, new_rule))) => {
            let old_replay = old_changes.replay.as_ref().expect("mailed write");
            let old_block = old_replay.palette[old_replay.blocks.get(old_index) as usize];
            let old_rule = old_replay.rules.get(old_index);
            !rule_allows(old_changes, old_rule, new_block, blocks) && rule_allows(new_changes, new_rule, old_block, blocks)
        }
        _ => false,
    };
    let block = if decided { merged.map_or(overlaid, |(_, block, _)| block) } else { overlaid };
    if block != baseline_block {
        target.set(x, y, z, block);
    }
}
