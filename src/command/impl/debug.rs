use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{CommandOverload, CommandParameter, CommandParameterType};
use crate::const_command;
use crate::network::BedrockProtocol;
use atomicow::CowArc;
use bedrock::form::elems::button::Button;
use bedrock::form::forms::Form;
use bedrock::form::forms::simple::SimpleForm;
use bedrock::protocol::v662::enums::{CommandPermissionLevel};
use bedrock::protocol::v662::packets::UpdateAbilitiesPacket;
use bedrock::protocol::v776::enums::AbilitiesIndex;
use bedrock::protocol::v776::types::{SerializedAbilitiesData, SerializedAbilitiesLayer, SerializedLayer};
use bedrock::protocol::v898::packets::CommandPermissionLevelString;
use tracing::info;

pub const DEBUG_COMMAND: CommandDefinition = const_command! {
    name: "debug",
    description: "Used for debugging",
    aliases: [],
    permission: CommandPermissionLevelString::Any,
    overloads: [
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("feature"),
                    kind: CommandParameterType::String,
                    optional: false
                }
            ])
        }
    ],
    execute: |_, sender, args| {
        match args.first() {
            Some(&"unhandled") => sender.reply(format!("Unhandled Packets: {:#?}", sender.session().unhandled_packets)),
            Some(&"form") => {
                let name = sender.name().to_owned();
                let (session, player) = sender.split();

                let Some(player) = player else {
                    return Err("must be sent by player!".to_owned());
                };

                player.send_form(
                    session,
                    Form::Simple(SimpleForm {
                        body: format!("Hello {}!", name),
                        buttons: vec![
                            Button { text: "Hey!".to_owned(), image: None },
                            Button {
                                text: "Fuck you".to_owned(),
                                image: None,
                            },
                        ],
                        title: "Simple Form".to_owned(),
                    }),
                    move || {
                        info!("{} responded!", name);
                    },
                );
            }
            Some(&"speed") => {
                let (session, player) = sender.split();

                let Some(player) = player else {
                    return Err("must be sent by player!".to_owned());
                };

                let ability_values = (1u32 << AbilitiesIndex::Build as u32)
                    | (1u32 << AbilitiesIndex::Mine as u32)
                    | (1u32 << AbilitiesIndex::DoorsAndSwitches as u32)
                    | (1u32 << AbilitiesIndex::OpenContainers as u32)
                    | (1u32 << AbilitiesIndex::AttackPlayers as u32)
                    | (1u32 << AbilitiesIndex::AttackMobs as u32);


                session.send(BedrockProtocol::UpdateAbilitiesPacket(
                    UpdateAbilitiesPacket {
                        data: SerializedAbilitiesData {
                            target_player_raw_id: player.unique_id(),
                            player_permissions: 1,
                            command_permissions: CommandPermissionLevel::Any,
                            layers: vec![SerializedLayer {
                                serialized_layer: SerializedAbilitiesLayer::Base,
                                abilities_set: 0xFFFFF,
                                ability_values,
                                fly_speed: 10.0,
                                vertical_fly_speed: 1.0,
                                walk_speed: 0.1,
                            }],
                        },
                    }
                    .into(),
                ));
            }
            _ => {}
        }
        Ok(())
    }
};
