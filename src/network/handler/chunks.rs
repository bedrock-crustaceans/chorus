use crate::level::level::Level;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use bedrock::protocol::v818::packets::{
    HeightMapDataType, SubChunkDataEntry, SubChunkPacket, SubChunkRequestResult,
};
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::Query;
use bevy_ecs::system::Res;

pub fn handle_sub_chunk_request(
    mut reader: MessageReader<PacketReceivedMessage>,
    mut query: Query<&mut Session>,
    level: Res<Level>,
) {
    for ev in reader.read() {
        let BedrockProtocol::SubChunkRequestPacket(packet) = &ev.packet else {
            continue;
        };
        let Ok(mut session) = query.get_mut(ev.entity) else {
            continue;
        };

        let mut entries = Vec::with_capacity(packet.sub_chunk_pos_offsets.len());

        for offset in &packet.sub_chunk_pos_offsets {
            let cx = packet.center_pos.x + offset.offset_x as i32;
            let cy = packet.center_pos.y + offset.offset_y as i32;
            let cz = packet.center_pos.z + offset.offset_z as i32;

            let dim = level.dimension(packet.dimension_type);
            let chunk = dim.and_then(|d| d.get_chunk(cx, cz));

            let (result, data) = if dim.is_none() || chunk.is_none() {
                (SubChunkRequestResult::LevelChunkDoesntExist, None)
            } else {
                match chunk.unwrap().get_sub_chunk(cy as i8) {
                    None => (SubChunkRequestResult::LevelChunkDoesntExist, None),
                    Some(sc) if sc.is_all_air() => (SubChunkRequestResult::SuccessAllAir, None),
                    Some(sc) => (
                        SubChunkRequestResult::Success,
                        Some(sc.serialize_network(cy as i8)),
                    ),
                }
            };

            entries.push(SubChunkDataEntry {
                sub_chunk_pos_offset: offset.clone(),
                sub_chunk_request_result: result,
                serialized_sub_chunk: data,
                height_map_data_type: HeightMapDataType::NoData,
                height_map_data: None,
                render_height_map_data_type: HeightMapDataType::NoData,
                render_height_map_data: None,
                blob_id: None,
            });
        }

        session.send(BedrockProtocol::SubChunkPacket(
            SubChunkPacket {
                cache_enabled: false,
                dimension_type: packet.dimension_type,
                center_pos: packet.center_pos.clone(),
                sub_chunk_data: entries,
            }
            .into(),
        ));
    }
}
