//! Vanilla game data shipped with the server.
//!
//! The files under `assets/` are dumps of the data the client expects for the protocol chorus
//! speaks. Only the ones a registry actually reads are embedded, the rest stay on disk until
//! something needs them.

/// The item table the client resolves every item id against.
pub const RUNTIME_ITEM_STATES: &str = include_str!("../assets/runtime_item_states.json");

/// The creative menu, built on top of [`RUNTIME_ITEM_STATES`].
pub const CREATIVE_ITEMS: &str = include_str!("../assets/creative_items.json");
