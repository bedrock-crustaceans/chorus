use crate::network::BedrockProtocol;
use crate::network::session::Session;
use bedrock::form::forms::Form;
use bedrock::protocol::v662::packets::ModalFormRequestPacket;
use bevy_ecs::prelude::Component;
use std::collections::HashMap;

type FormCallback = Box<dyn FnOnce() + Send + Sync>;

#[derive(Component, Default)]
pub struct PendingForms {
    next_id: u32,
    pending: HashMap<u32, (Form, FormCallback)>,
}

impl PendingForms {
    pub fn send<F>(&mut self, session: &mut Session, form: Form, on_response: F)
    where
        F: FnOnce() + Send + Sync + 'static,
    {
        let Ok(json) = facet_json::to_string(&form) else {
            return;
        };

        let id = self.next_id;
        self.next_id += 1;

        session.send(BedrockProtocol::ModalFormRequestPacket(ModalFormRequestPacket { form_id: id, form_ui_json: json }.into()));

        self.pending.insert(id, (form, Box::new(on_response)));
    }

    pub fn take(&mut self, id: u32) -> Option<(Form, FormCallback)> {
        self.pending.remove(&id)
    }
}
