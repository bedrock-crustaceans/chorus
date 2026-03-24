use bedrockrs::proto::compression::Compression;
use bedrockrs::proto::connection::Connection;
use bedrockrs::proto::v800::packets::{NetworkSettingsPacket, RequestNetworkSettingsPacket};
use bedrockrs::proto::v800::{ProtoHelperV800, GamePackets};
use bedrockrs::proto::v800::enums::PacketCompressionAlgorithm;
use log::debug;

pub struct LoginHandler;

impl LoginHandler {
    pub async fn handle(mut conn: Connection) {
        debug!("Got connection from {:?}", conn.get_socket_addr());

        loop {
            let packets = match conn.recv::<ProtoHelperV800>().await {
                Ok(pk_vec) => pk_vec,
                Err(e) => {
                    debug!("Connection closed: {:?}", e);
                    break;
                }
            };

            for packet in packets {
                if let Err(e) = Self::handle_packet(&mut conn, packet).await {
                    debug!("Error handling packet: {:?}", e);
                }
            }
        }
    }

    async fn handle_packet(conn: &mut Connection, packet: GamePackets) -> anyhow::Result<()> {
        use GamePackets::*;

        match packet {
            RequestNetworkSettings(pk) => {
                Self::handle_request_network_settings(conn, pk).await?
            }
            _ => debug!("Unhandled packet: {:?}", packet),
        }

        Ok(())
    }

    async fn handle_request_network_settings(
        conn: &mut Connection,
        packet: RequestNetworkSettingsPacket,
    ) -> anyhow::Result<()> {
        conn.recv::<ProtoHelperV800>().await?;
        
        conn.send::<ProtoHelperV800>(&[GamePackets::NetworkSettings(NetworkSettingsPacket {
            compression_threshold: 1,
            compression_algorithm: PacketCompressionAlgorithm::None,
            client_throttle_enabled: false,
            client_throttle_threshold: 0,
            client_throttle_scalar: 0.0,
        })])
            .await?;

        conn.compression = Some(Compression::None);

        conn.recv::<ProtoHelperV800>().await?;
        Ok(())
    }
}