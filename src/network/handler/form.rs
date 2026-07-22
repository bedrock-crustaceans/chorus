use crate::network::BedrockProtocol;
use crate::player::Player;
use bedrock::protocol::ProtoVersionPackets;

pub fn handle_modal_form_response(packet: &<BedrockProtocol as ProtoVersionPackets>::ModalFormResponsePacket, player: &mut Player) {
    let Some((_form, on_response)) = player.forms_pending.remove(&packet.form_id) else {
        return;
    };

    on_response();
}
