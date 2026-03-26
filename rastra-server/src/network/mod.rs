use bedrockrs::proto::{ProtoVersion, V944};
use rastra_api::modules;

modules!(
    pub handler,
    pub network
);

/*
NOTE: Do not waste your time submitting pull requests changing game, protocol or rak version.
Our maintainers are handling protocol changes immediately when the version released.
Pull requests changing game, protocol or rak version will be closed.
*/
pub const GAME_VERSION: &str = V944::GAME_VERSION;
pub const PROTOCOL_VERSION: u32 = V944::PROTOCOL_VERSION;
pub const RAK_VERSION: u8 = V944::RAKNET_VERSION;