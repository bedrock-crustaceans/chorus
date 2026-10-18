# Chorus plugins

A Chorus plugin is a single WebAssembly component. The server loads every `.wasm` file in its
`plugins/` folder at startup, reads the plugin's own metadata out of the component, and gives it a
data folder at `plugins/<name>/`. There is no manifest file to write and no need to build the
server.

The host/guest contract is the `chorus:plugin` WIT package in [`crates/plugin/wit`](../../crates/plugin/wit).
Any language with a Component Model toolchain can target it; `api/` is the Rust side.

## Layout

| Directory | What it is |
|---|---|
| `api/` | `chorus_plugin_api`, the Rust guest crate: generated bindings plus `register_plugin!` |
| `hello/` | an example plugin, and the fixture the host's tests run against |

Both carry their own empty `[workspace]` table, so they are not part of the server's Cargo
workspace and a plain `cargo build` at the repository root ignores them.

## Building a plugin

```bash
rustup target add wasm32-wasip2
cargo build --release --target wasm32-wasip2 --manifest-path examples/plugins/hello/Cargo.toml
cp examples/plugins/hello/target/wasm32-wasip2/release/hello.wasm plugins/
```

If `rustc -V` reports a distribution-packaged toolchain (Fedora's `rustc`, for instance), that
toolchain has no `wasm32-wasip2` standard library and the build fails with `can't find crate for
core`. `rustup run stable cargo` does not help, because cargo still resolves `rustc` through
`PATH`. Put the rustup toolchain first instead:

```bash
PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH" \
  cargo build --release --target wasm32-wasip2 --manifest-path examples/plugins/hello/Cargo.toml
```

## Writing one

Implement `Plugin`, hand the type to `register_plugin!`, and that is the whole shape:

```rust
use chorus_plugin_api::{Metadata, Plugin};

struct MyPlugin;

impl Plugin for MyPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            name: "my-plugin".to_owned(),
            version: "0.1.0".to_owned(),
            authors: vec!["you".to_owned()],
            description: "does a thing".to_owned(),
        }
    }
}

chorus_plugin_api::register_plugin!(MyPlugin);
```

Everything but `metadata` has a default. `on_load` is where commands get registered, `on_event`
receives the event set, and `on_command` runs your commands. Plugin code is single threaded, so
mutable state goes in a `Cell` or `RefCell` inside your type.

## What a plugin can and cannot do

It can log, send text to players, read players, register commands, and see the v0.1 events:
player join, player leave, player chat, block break and block place. Chat is blocking: the host
reads your returned event back, so you can cancel the message or rewrite it.

It cannot open sockets, read the process environment, or touch any directory but its own data
folder. Calls run inline on the server tick with a fuel limit and an epoch deadline, so a plugin
that loops forever is quarantined with a log line rather than taking the server down with it.
