use crate::debug::hud::page::{BossFrame, FrameChanges};
use crate::network::BedrockProtocol;
use bedrock::protocol::v662::types::ActorUniqueID;
use bedrock::protocol::v712::packets::{SetTitlePacket, TitleType};
use bedrock::protocol::v2193::packets::{BossEventPacket, BossEventUpdateType};

const BOSS_BAR_OVERLAY_PROGRESS: u8 = 0;

pub fn action_bar(text: &str) -> BedrockProtocol {
    BedrockProtocol::SetTitlePacket(
        SetTitlePacket {
            title_type: TitleType::Actionbar,
            title_text: text.to_owned(),
            fade_in_time: 0,
            stay_time: 0,
            fade_out_time: 0,
            xuid: String::new(),
            platform_online_id: String::new(),
            filtered_title_text: text.to_owned(),
        }
        .into(),
    )
}

fn boss_event(own_unique_id: i64, event_type: BossEventUpdateType, frame: &BossFrame) -> BedrockProtocol {
    BedrockProtocol::BossEventPacket(
        BossEventPacket {
            target_actor_id: ActorUniqueID(own_unique_id),
            event_type,
            name: frame.title.clone(),
            filtered_name: frame.title.clone(),
            health_percent: frame.progress,
            color: frame.color as u8,
            overlay: BOSS_BAR_OVERLAY_PROGRESS,
        }
        .into(),
    )
}

pub fn boss_bar_add(own_unique_id: i64, frame: &BossFrame) -> BedrockProtocol {
    boss_event(own_unique_id, BossEventUpdateType::Add, frame)
}

pub fn boss_bar_remove(own_unique_id: i64, frame: &BossFrame) -> BedrockProtocol {
    boss_event(own_unique_id, BossEventUpdateType::Remove, frame)
}

pub fn boss_bar_updates(own_unique_id: i64, frame: &BossFrame, changes: FrameChanges) -> Vec<BedrockProtocol> {
    let mut packets = Vec::new();
    if changes.title {
        packets.push(boss_event(own_unique_id, BossEventUpdateType::UpdateName, frame));
    }
    if changes.progress {
        packets.push(boss_event(own_unique_id, BossEventUpdateType::UpdatePercent, frame));
    }
    if changes.color {
        packets.push(boss_event(own_unique_id, BossEventUpdateType::UpdateStyle, frame));
    }
    packets
}
