# RAstra

RAstra is modern Minecraft Bedrock Edition server software written in Rust, built on top of [`bedrock-rs`](https://github.com/bedrock-crustaceans/bedrock-rs) - a foundational library for MCBE tooling in Rust.

> [!NOTE]
> We are currently working on [`bedrock-rs`](https://github.com/bedrock-crustaceans/bedrock-rs).
> Which will provide the base for RAstra and other software written in Rust related to MCBE.

## Building

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable recommended)

### Steps

```bash
git clone https://github.com/bedrock-crustaceans/RAstra
cd RAstra
cargo build
```

For a release build:

```bash
cargo build --release
```

The compiled binary will be in `target/debug/` or `target/release/` respectively.

## Contributing

Contributions are welcome! To get started:

1. Fork the repository
2. Create a feature branch (`git checkout -b my-feature`)
3. Commit your changes (`git commit -am 'Add my feature'`)
4. Push to the branch (`git push origin my-feature`)
5. Open a Pull Request

If you're unsure where to start or want to discuss ideas before building, join the [Discord](https://discord.com/invite/VCVcrvt3JC) first.

## Community

Join our Discord to follow development, ask questions, and get involved:
👉 [discord.com/invite/VCVcrvt3JC](https://discord.com/invite/VCVcrvt3JC)

## License

RAstra is licensed under the [Apache License 2.0](LICENSE).
