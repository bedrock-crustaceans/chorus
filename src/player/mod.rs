use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::player::block_break::BlockBreakHandler;
use crate::player::gamemode::Gamemode;
use crate::player::inventory::PlayerInventory;
use bedrock::form::forms::Form;
use bedrock::protocol::v662::packets::{ModalFormRequestPacket, SetPlayerGameTypePacket};
use bevy_ecs::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

pub mod block_break;
pub mod gamemode;
pub mod identity;
pub mod inventory;

#[derive(Component)]
pub struct Player {
    unique_id: i64,
    runtime_id: u64,
    gamemode: Gamemode,

    pub chunks_radius: i32,
    /// None until the first chunk order run.
    pub chunks_center: Option<(i32, i32)>,
    pub chunks_pending: VecDeque<(i32, i32)>,
    pub chunks_sent: HashSet<(i32, i32)>,

    pub block_break: Option<BlockBreakHandler>,
    pub inventory: PlayerInventory,

    pub forms_id: u32,
    pub forms_pending: HashMap<u32, (Form, Box<dyn FnOnce() + Send + Sync>)>,
}

impl Player {
    pub fn new(runtime_id: u64) -> Self {
        Self {
            unique_id: rand::random(),
            runtime_id,
            gamemode: Gamemode::default(),

            chunks_radius: 0,
            chunks_center: None,
            chunks_pending: VecDeque::new(),
            chunks_sent: HashSet::new(),

            block_break: None,
            inventory: PlayerInventory::new(),

            forms_id: 0,
            forms_pending: HashMap::new(),
        }
    }

    pub fn unique_id(&self) -> i64 {
        self.unique_id
    }

    pub fn runtime_id(&self) -> u64 {
        self.runtime_id
    }

    pub fn gamemode(&self) -> Gamemode {
        self.gamemode
    }

    pub fn set_gamemode(&mut self, session: &mut Session, gamemode: Gamemode) {
        self.gamemode = gamemode;

        session.send(BedrockProtocol::SetPlayerGameTypePacket(
            SetPlayerGameTypePacket {
                player_game_type: gamemode.game_type(),
            }
            .into(),
        ));
    }

    pub fn send_form<F>(&mut self, session: &mut Session, form: Form, on_response: F)
    where
        F: FnOnce() + Send + Sync + 'static,
    {
        let Ok(json) = serde_json::to_string(&form) else {
            return;
        };

        let id = self.forms_id;
        self.forms_id += 1;

        session.send(BedrockProtocol::ModalFormRequestPacket(ModalFormRequestPacket { form_id: id, form_ui_json: json }.into()));

        self.forms_pending.insert(id, (form, Box::new(on_response)));
    }
}
