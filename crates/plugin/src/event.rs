//! Event ids, the discriminant the host and plugin agree on.

use crate::chorus::plugin::event::Event;

/// The v0.1 event set. These numbers are part of the ABI: a plugin can switch on them, so they are
/// append-only. Adding an event means a new number, never a renumbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EventId {
    PlayerJoin = 0,
    PlayerLeave = 1,
    PlayerChat = 2,
    BlockBreak = 3,
    BlockPlace = 4,
}

impl EventId {
    pub const fn id(self) -> u32 {
        self as u32
    }

    /// Which event a value is, so a returned event can be checked against the dispatched one.
    pub const fn of(event: &Event) -> Self {
        match event {
            Event::PlayerJoin(_) => Self::PlayerJoin,
            Event::PlayerLeave(_) => Self::PlayerLeave,
            Event::PlayerChat(_) => Self::PlayerChat,
            Event::BlockBreak(_) => Self::BlockBreak,
            Event::BlockPlace(_) => Self::BlockPlace,
        }
    }

    /// A plugin that returns a different case than it was handed has a bug, and the host keeps the
    /// original event rather than guessing what it meant.
    pub const fn matches(self, event: &Event) -> bool {
        Self::of(event) as u32 == self as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chorus::plugin::event::{PlayerJoinData, PlayerLeaveData};
    use crate::chorus::plugin::player::Player;

    fn player() -> Player {
        Player {
            xuid: String::new(),
            name: String::from("Steve"),
            position: crate::chorus::plugin::common::Position { x: 0.0, y: 0.0, z: 0.0 },
            game_mode: crate::chorus::plugin::common::GameMode::Survival,
        }
    }

    #[test]
    fn event_ids_are_stable() {
        assert_eq!(EventId::PlayerJoin.id(), 0);
        assert_eq!(EventId::PlayerLeave.id(), 1);
        assert_eq!(EventId::PlayerChat.id(), 2);
        assert_eq!(EventId::BlockBreak.id(), 3);
        assert_eq!(EventId::BlockPlace.id(), 4);
    }

    #[test]
    fn a_returned_event_of_the_same_case_matches() {
        let event = Event::PlayerJoin(PlayerJoinData { player: player() });

        assert!(EventId::PlayerJoin.matches(&event));
    }

    #[test]
    fn a_returned_event_of_another_case_does_not_match() {
        let event = Event::PlayerLeave(PlayerLeaveData { player: player() });

        assert!(!EventId::PlayerJoin.matches(&event));
    }
}
