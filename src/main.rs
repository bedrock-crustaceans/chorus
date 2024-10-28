use std::collections::HashMap;
use crate::config::RAstraConfig;
use bedrockrs::proto::login::login_to_server;
use bedrockrs::proto::login::provider::DefaultLoginProvider;
use bedrockrs::proto::{connection, listener};
use log::{debug, error, info, warn};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::process::exit;
use std::str::FromStr;
use std::time::Duration;
use bedrockrs::proto::gamepackets::GamePackets;
use bedrockrs::proto::packets::chunk_publisher_update::ChunkPublisherUpdatePacket;
use bedrockrs::proto::packets::chunk_radius_updated::ChunkRadiusUpdatedPacket;
use bedrockrs::proto::packets::level_chunk::LevelChunkPacket;
use bedrockrs::proto::types::block_pos::BlockPos;
use bedrockrs::proto::types::chunk_pos::ChunkPos;
use bedrockrs::world::palette::PalettedStorage;

mod config;
mod logger;

fn main() {
    let config = config::setup_config();

    logger::setup_logger(config.log_to_file, &config.logs_directory);

    debug!("Initialization started!");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.threads)
        .enable_all()
        .build()
        .unwrap_or_else(|err| {
            error!("An unexpected Error occurred while trying to create the Tokio-Runtime, Err: {err:?}");
            exit(1)
        });

    runtime.block_on(async {
        debug!("Tokio-Runtime initialized!");
        rastra_main(config).await;
    });
}

async fn rastra_main(_config: RAstraConfig) {
    info!("Hello World!");

    let mut listener = listener::Listener::new_raknet(
        _config.display_name,
        _config.display_sub_name,
        "1.21.0".to_string(),
        100,
        0,
        SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::from_str("127.0.0.1").unwrap(),
            19132,
        )),
        false,
    )
    .await
    .unwrap();

    listener.start().await.unwrap();

    loop {
        let conn = listener.accept().await.unwrap();

        tokio::spawn(handle_conn(conn));
    }
}

async fn handle_conn(connection: connection::Connection) {
    let mut shard = connection
        .into_shard(Duration::from_millis(1000 / 20), 256)
        .await;

    login_to_server(&mut shard, DefaultLoginProvider::new())
        .await
        .unwrap();

    loop {
        let pk = shard.recv().await.unwrap();
        match pk {
            GamePackets::SetLocalPlayerAsInitialized(_) => {
                tokio::time::sleep(Duration::from_millis(1000)).await;

                let mut palette: Vec<nbtx::Value> = Vec::new();

                let mut fields: HashMap<String, nbtx::Value> = HashMap::new();
                fields.insert(String::from("name"), nbtx::Value::String(String::from("minecraft:air")));

                let compound = nbtx::Value::Compound(fields);
                palette.push(compound);

                let mut fields: HashMap<String, nbtx::Value> = HashMap::new();
                fields.insert(String::from("name"), nbtx::Value::String(String::from("minecraft:dirt")));

                let compound = nbtx::Value::Compound(fields);
                palette.push(compound);

                let mut blocks = [0; 4096];

                for i in 0..16*16 {
                    blocks[i] = 1;
                }

                let storage = PalettedStorage {
                    blocks,
                    palette
                };

                // Serialize to chunk data
                let mut chunk_serialized_data: Vec<u8> = Vec::new();
                chunk_serialized_data.push(8); // format version 8
                chunk_serialized_data.push(1); // only 1 storage (further ones are used for water)
                chunk_serialized_data.extend_from_slice(&storage.encode(true));

                for x in -8..8 {
                    for y in -8..8 {
                        shard.send(GamePackets::LevelChunk(LevelChunkPacket {
                            chunk_position: ChunkPos::new(x, y),
                            dimension_id: VAR::new(0),
                            sub_chunk_count: VAR::new(1),
                            cache_enabled: false,
                            serialized_chunk_data: chunk_serialized_data.clone(),
                            client_needs_to_request_subchunks: false,
                            client_request_subchunk_limit: VAR::new(0xFFFFFFFFu32 as i32),
                        })).await.unwrap();
                    }
                }

                shard.send(GamePackets::ChunkPublisherUpdate(ChunkPublisherUpdatePacket{
                    position: BlockPos::new(0, 0, 0),
                    radius: VAR::new(16*16),
                    saved_chunks: vec![]
                })).await.unwrap();
            }
            GamePackets::PacketViolationWarning(pk) => {
                warn!("======= PacketViolationWARNING =======");
                warn!("PK ID:    {}", pk.violating_packet_id.into_inner());
                warn!("SEVERITY: {}", pk.severity.into_inner());
                warn!("KIND:     {}", pk.kind.into_inner());
                warn!("CONTEXT:  {}", pk.context);
            }
            GamePackets::RequestChunkRadius(pk) => {
                shard.send(GamePackets::ChunkRadiusUpdate(ChunkRadiusUpdatedPacket{
                    chunk_radius: VAR::new(pk.chunk_radius_max as i32),
                })).await.unwrap();
                shard.flush().await.unwrap();
            }
            _ => {}
        }
    }
}
