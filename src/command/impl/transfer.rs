use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{ArgumentType, CommandParameter};
use crate::network::session::Session;
use crate::overloads;
use crate::player::identity::PlayerIdentity;
use bedrock::protocol::v2168::packets::TransferPlayerPacket;
use chorus_core::permission::PermissionLevel;
use chorus_core::protocol::BedrockProtocol;

pub const TRANSFER_COMMAND: CommandDefinition = CommandDefinition::new("transfer", "Transfers a player to another server", |context, args| {
    let name = args.string("pfidOrMsa").unwrap_or_default();

    let world = context.world_mut();
    let mut query = world.query::<(&mut Session, &PlayerIdentity)>();

    let (mut target, _) = query
        .iter_mut(world)
        .find(|(_, ident)| ident.name().eq_ignore_ascii_case(name))
        .ok_or_else(|| format!("No player named \"{name}\" is online."))?;

    let packet = BedrockProtocol::TransferPlayerPacket(
        TransferPlayerPacket {
            server_address: args.string("server").unwrap_or_default().to_string(),
            server_port: args.int("port").unwrap_or(19132) as u16,
            reload_world: false,
            gatherings_config: None,
        }
        .into(),
    );

    target.as_mut().send(packet);

    Ok(())
})
.overloads(overloads![[
    CommandParameter::new("pfidOrMsa", ArgumentType::String),
    CommandParameter::new("server", ArgumentType::String),
    CommandParameter::new("port", ArgumentType::Int).optional(),
]])
.permission(PermissionLevel::Owner);
