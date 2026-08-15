use crate::network::BedrockProtocol;
use crate::player::Player;
use bedrock::protocol::ProtoVersionPackets;
use bevy_ecs::message::{Message, MessageWriter};
use bevy_ecs::prelude::Entity;

#[derive(Message, Clone, Debug)]
pub struct FormResponseMessage {
    pub entity: Entity,
    pub form_id: u32,
}

pub fn handle_modal_form_response(
    entity: Entity,
    packet: &<BedrockProtocol as ProtoVersionPackets>::ModalFormResponsePacket,
    player: &mut Player,
    form_writer: &mut MessageWriter<FormResponseMessage>,
) {
    let Some((_form, on_response)) = player.forms_pending.remove(&packet.form_id) else {
        return;
    };

    on_response();

    form_writer.write(FormResponseMessage { entity, form_id: packet.form_id });
}
