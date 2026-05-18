# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Chorus is Minecraft Bedrock Edition server software written in Rust. It is built on top of [`bedrock-rs`](https://github.com/bedrock-crustaceans/bedrock-rs) (vendored under `libs/bedrock-rs`) and uses Bevy ECS as the server tick/scheduling backbone.

## Commands

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run the server (creates chorus.toml on first run)
cargo run

# Check compilation without building
cargo check

# Format (max_width = 200 per rustfmt.toml)
cargo fmt

# Lint
cargo clippy
```

There are no tests at this time. The workspace has one internal crate: `crates/raknet`.

## Architecture

### Bevy ECS as the tick loop

`Chorus::init()` (`src/lib.rs`) constructs a Bevy `App` configured to tick at 20 Hz via `ScheduleRunnerPlugin` + `Time<Fixed>`. All game logic lives in Bevy systems, resources, and components.

### Plugin tree

```
Server (src/server.rs)
├── Registry (src/registry/)   — registers block definitions into BlockRegistry
└── Network (src/network/network.rs)
    ├── PacketHandlers          — per-state packet routing systems
    └── LoginAuthOIDC           — optional OIDC auth resource
```

`Server::build` inserts `ServerState` (tick counter + runtime-ID generator) and wires `FixedFirst`/`FixedLast` systems for tick metrics.

### Network & Session lifecycle

The `Network` plugin owns a Tokio runtime and a `bedrock-rs` RakNet `Listener`. A background task accepts incoming connections and sends them through a `crossbeam_channel` to the ECS world.

Each connection becomes a `Session` Bevy component (`src/network/session/mod.rs`) spawned onto an entity. `Session` bridges the synchronous ECS world to an async Tokio task via two `mpsc` channels (`ConnectionEvent` outbound, `BedrockProtocol` inbound).

`Session` holds a `SessionState` state machine:

```
Request → Login → Handshake (if encryption) → Resource → Setup → Play
```

State transitions emit a `SessionStateChangedMessage` which handler systems observe to run entry logic (`on_enter_setup`, etc.).

### Packet routing

`PacketHandlers` runs five systems every `FixedUpdate` tick. Each system reads `PacketReceivedMessage`, filters by `SessionState`, and dispatches to the relevant handler:

| Handler file | State |
|---|---|
| `handler/request.rs` | `Request` |
| `handler/login.rs` | `Login` |
| `handler/handshake.rs` | `Handshake` |
| `handler/resource.rs` | `Resource` |
| `handler/setup.rs` | `Setup` / `Play` |

### Block system

`BlockDefinition` (`src/block/block_definition.rs`) declares a block's identifier, states (combinatorial state values), base components, and conditional permutation overrides. `BlockDefinition::generate()` expands all permutations, computes FNV hashes, and returns maps from hash → `BlockPermutation` and hash → `BlockComponents`.

Use the `const_block!` / `const_permutation!` macros for compile-time static definitions (see `src/block/impl/grass_block.rs` for a minimal example). Runtime-allocated definitions use `BlockDefinition::new(...)`.

`BlockRegistry` (`src/registry/block_registry.rs`) is a Bevy `Resource`. Add new blocks by calling `registry.register_all([...])` inside `BlockRegistry::init`.

### Resource packs

`ResourcePacks::load` (`src/resource/mod.rs`) is a startup system that scans the configured `resource_packs_directory` for `.mcpack` / `.zip` files and loads them into the `ResourcePacks` Bevy resource.

### Configuration

`chorus.toml` is read (or created with defaults) at startup by `Config::setup()`. Key fields: `ip`, `port`, `threads`, `online_mode`, `encryption`, `resource_packs_directory`, `behavior_packs_directory`, `log_level`.

### Protocol version

`BedrockProtocol` is a type alias for `V975` from `bedrock-rs` (`src/network/mod.rs`). To change the protocol version, update this alias and adjust any version-specific packet imports.
