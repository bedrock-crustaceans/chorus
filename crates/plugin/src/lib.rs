//! The Chorus plugin ABI.
//!
//! This crate is the contract, not the integration. It owns the `chorus:plugin` WIT package, the
//! host side bindings generated from it, and the wasmtime engine that runs plugin components. It
//! deliberately knows nothing about Bevy or the game: the bridge in `chorus::plugin` is what
//! reaches into the `World`.

pub mod abi;
pub mod metadata;

wasmtime::component::bindgen!({
    path: "wit",
    world: "plugin",
});
