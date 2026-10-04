# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Chorus is Minecraft Bedrock Edition server software written in Rust. It is built on top of [`bedrock-rs`](https://github.com/bedrock-crustaceans/bedrock-rs) and uses Bevy ECS (`bevy_app` / `bevy_ecs`, no full Bevy) as the server tick/scheduling backbone.

`bedrock-rs`, `bevy-raknet`, `bevy-nethernet` and `nbtx` are git dependencies pinned to `branch = "main"` in `Cargo.toml`; nothing is vendored. To develop against a local checkout of one of them, add a temporary `[patch]` section and do not commit it.

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

```bash
# Build or test one crate
cargo build -p chorus_worldgen
cargo test -p chorus_worldgen

# Worldgen regression checks (ignored by default; run in release)
cargo test --release -p chorus_worldgen -- --ignored output_checksum --nocapture
```

### Workspace layout

Chorus is a Cargo workspace. The root package `chorus` (`src/`) is the server: network, commands, players, forms, resource packs, the tick loop and the binary. Lower layers live in `crates/`:

| Crate | Path | Contents |
|---|---|---|
| `chorus_core` | `crates/core` | config, math, utils, version info, the `BedrockProtocol` alias, tick schedule types (`Tick`, `TickSet`, `TickClock`, `JobQueue`, `TICK_RATE`). Error types live in the module that uses them. |
| `chorus_block` | `crates/block` | block definitions, states, components, `block_registry`, block permutation hashing |
| `chorus_entity` | `crates/entity` | entity components and NBT structs |
| `chorus_item` | `crates/item` | item stacks, `item_registry` and its bundled JSON |
| `chorus_level` | `crates/level` | world state: chunk, sub-chunk, palettes, biome ids and the biome definition list, dimension types, the `Level` resource, level messages, and the generation framework under `generator` (`Dimension`, `Generator`, `WorldGenerator`, the phase graph, phase errors) |
| `chorus_worldgen` | `crates/worldgen` | the concrete generators: `flat`, `random`, `void` and `overworld` |

Dependencies only point downwards (core, then block, then item and level, then worldgen, then the root). The root `chorus` crate re-exports the old module paths (`chorus::block`, `chorus::level`, `chorus::level::generator::r#impl` for worldgen, `chorus::utils`, `chorus::registry::block_registry`, ...), so app code and downstream users such as Pyrite keep their imports. Shared dependency versions live in `[workspace.dependencies]`.

## Architecture

### Custom tick loop on Bevy ECS

`Chorus::init()` (`src/lib.rs`) builds a Bevy `App` with `ChorusPlugin` and a custom runner, `LoopRunner::run`. Game logic runs in a custom `Tick` schedule with three chained sets, `TickSet::First`, `TickSet::Update`, `TickSet::Last`. There is no Bevy `FixedUpdate`.

- `run_fixed_tick` (added to `RunFixedMainLoop`) accumulates elapsed time in `TickClock` and runs the `Tick` schedule once per 50 ms (`TICK_RATE = 20.0` in `crates/core/src/schedule.rs`), catching up at most `MAX_TICKS_PER_UPDATE = 5` ticks per `app.update()`.
- After each update, `LoopRunner` drains the `JobQueue` resource (a queue of registered `SystemId`s) until the tick deadline, then spin-sleeps the remainder. Use `JobQueue::push` for deferred work that should use spare time in the tick (e.g. `Level::queue_poll_generation`).
- `Server::start_tick` / `Server::end_tick` run in `TickSet::First` / `TickSet::Last` and maintain `ServerState` (tick counter, runtime-ID generator) and `ServerMetrics` (TPS / MSPT).

### Plugin tree

```
ChorusPlugin (src/lib.rs)        — Config::setup(), Tick schedule, TaskPoolPlugin, logger
└── Server (src/server/mod.rs)   — ServerState, ServerMetrics, tick metrics
    ├── Registry (src/registry/) — Startup: BlockRegistry, CommandRegistry, ResourcePacks, ItemRegistry, init_level; generation polling
    └── Network (src/network/network.rs)
        ├── PacketHandlers        — all packet handler systems, in Tick / TickSet::Update
        ├── LoginAuthOIDC         — optional OIDC auth resource
        ├── RakServerPlugin       — bevy-raknet transport
        ├── NetherServerPlugin    — bevy-nethernet LAN signaling transport
        └── NetherHttpServerPlugin
```

### Network & Session lifecycle

`Network::init` (Startup) binds either a RakNet or a NetherNet server depending on `config.transport`. `Network::accept` and `Network::queue_receive` run in `PreUpdate` (after the transport plugins' sets), and `Network::flush` runs in `PostUpdate`.

Each connection becomes a `Session` Bevy component (`src/network/session/mod.rs`) on its own entity. `Session` owns the connection's `SessionState`, compression and encryption settings, and an outgoing packet queue that `flush` encodes and sends.

`SessionState` (`src/network/session/state.rs`) is a state machine:

```
Request → Login → Handshake (if encryption) → Resource → Setup → Play
```

State transitions emit a `SessionStateChangedMessage`, which handler systems observe to run entry logic (`on_enter_setup`, `on_enter_play`).

### Packet routing

`PacketHandlers` (`src/network/handler/mod.rs`) registers all handler systems in the `Tick` schedule inside `TickSet::Update`, as one long `.chain()` of grouped systems. Each state handler reads `PacketReceivedMessage`, filters by `SessionState`, and dispatches:

| Handler file | Responsibility |
|---|---|
| `handler/request.rs` | `Request` state |
| `handler/login.rs` | `Login` state |
| `handler/handshake.rs` | `Handshake` state |
| `handler/resource.rs` | `Resource` state: pack info, chunk serving, pack stack |
| `handler/setup.rs` | `Setup` state: StartGame, item/creative/biome packets |
| `handler/play.rs` | `Play` state: movement, join/quit, block update broadcasts |
| `handler/block.rs` | block break/place actions and level event broadcasts |
| `handler/inventory.rs` | inventory transactions and held item |
| `handler/chat.rs` | chat and broadcast messages |
| `handler/chunks.rs` | chunk ordering, sending, unloading, sub-chunk requests |
| `handler/form.rs` | form responses |

Commands are dispatched by `dispatch_commands` (`src/command/dispatch.rs`), an exclusive `&mut World` system in the same chain.

### Block system

`BlockDefinition` (`crates/block/src/block_definition.rs`) declares a block's identifier, states (combinatorial state values), base components, and conditional permutation overrides. `BlockDefinition::generate()` expands all permutations, computes FNV hashes, and returns maps from hash → `BlockPermutation` and hash → `BlockComponents`.

Use the `const_block!` / `const_permutation!` macros for compile-time static definitions (see `crates/block/src/impl/grass_block.rs` for a minimal example). Runtime-allocated definitions use `BlockDefinition::new(...)`. All vanilla blocks are hand-written consts under `crates/block/src/impl/` and collected in `DEFINITIONS`.

`BlockRegistry` (`crates/block/src/block_registry.rs`) is a Bevy `Resource`. Add new blocks by calling `registry.register_all([...])` inside `BlockRegistry::init`.

### Items and vanilla data

`ItemRegistry` (`crates/item/src/item_registry.rs`) is built from `crates/item/resources/item_palette.json` and `crates/item/resources/creative_items.json` at startup. The biome definition list sent to clients is built from `crates/level/resources/biome_definitions.json`.

### Level

`Level` (`crates/level/src/level.rs`) is a single global resource, created by `init_level` (`src/registry/mod.rs`), which picks the overworld generator. Chunk generation is polled through the `JobQueue`.

Worlds are saved in the vanilla Bedrock LevelDB layout under `worlds/<level_name>/` (`db/`, `level.dat`, `levelname.txt`) by `LevelStorage` (`crates/level/src/storage.rs`). A `Dimension` built `with_storage` loads chunks from disk before asking its generator, marks generated and edited chunks unsaved, and saves them when they unload, on the one-minute autosave, on `/save` and on exit. An existing `level.dat` wins over `level_seed` in the config. Palette entries are written as `{name, states, version}` NBT and read back through `HashUtils::hash_block_nbt`.

### Resource packs

`ResourcePacks::load` (`src/resource/mod.rs`) is a startup system that scans the configured `resource_packs_directory` for `.mcpack` / `.zip` files and loads them into the `ResourcePacks` Bevy resource. Behavior packs are not loaded yet; `behavior_packs_directory` is only created.

### Configuration

`chorus.toml` is read (or created with defaults) at startup by `Config::setup()` (`crates/core/src/config.rs`). Fields: `ip`, `port`, `name`, `sub_name`, `max_players`, `threads`, `transport` (`RakNet` | `NetherNet`), `nethernet_http_port`, `log_to_file`, `logs_directory`, `resource_packs_directory`, `behavior_packs_directory`, `level_name`, `level_seed`, `online_mode`, `encryption`, `log_level`, `force_accept_resource_packs`, `force_disable_vibrant_visuals`, `max_view_distance`, `max_generation_distance`.

### Protocol version

`BedrockProtocol` is a type alias for `V2193` from `bedrock-rs` (`crates/core/src/protocol.rs`, re-exported from `chorus::network`). The matching `protocol-v2193` feature is enabled on the workspace `bedrock` dependency in the root `Cargo.toml`. To change the protocol version, update the alias, the Cargo feature, and any version-specific packet imports (handlers import packet types from the specific `bedrock::protocol::vNNN` module where they were introduced).
