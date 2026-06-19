<br />
<div align="center">

[![Chorus-OSS][chorus_logo_url]][chorus_url]

# Chorus

Minecraft: Bedrock server software, open source and written in Rust

[![rust][rust_badge_url]][rust_url]
[![minecraft][minecraft_badge_url]][minecraft_url]
[![protocol][protocol_badge_url]][protocol_url]
[![license][license_badge_url]][license_url]

</div>

Chorus is modern Minecraft Bedrock Edition server software written in Rust, built on top of [`bedrock-rs`](https://github.com/bedrock-crustaceans/bedrock-rs) - a foundational library for MCBE tooling in Rust.

## Building

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable recommended)

### Steps

```bash
git clone https://github.com/bedrock-crustaceans/Chorus.git
cd Chorus
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

If you're unsure where to start or want to discuss ideas before building, join the [Discord](https://discord.bedrock-crustaceans.org) first.

## Community

Join our Discord to follow development, ask questions, and get involved on [Discord](https://discord.bedrock-crustaceans.org)

## License

Chorus is licensed under the [Apache License 2.0](LICENSE).

<!-- CHORUS -->

[chorus_logo_url]: .github/img/chorus.256.png
[chorus_url]: https://bedrock-crustaceans.org/chorus

<!-- CHORUS -->

<!-- BADGES -->

[minecraft_badge_url]: https://img.shields.io/badge/minecraft-26.30-black?style=flat-square
[minecraft_url]: https://www.minecraft.net/en-us/article/minecraft-26-30-bedrock-changelog
[protocol_badge_url]: https://img.shields.io/badge/protocol-v1001-white?style=flat-square
[protocol_url]: https://github.com/Mojang/bedrock-protocol-docs
[rust_badge_url]: https://img.shields.io/badge/rust-2024-%23D34516?style=flat-square&logo=rust&logoColor=%23D34516&labelColor=white
[rust_url]: https://rust-lang.org/
[license_badge_url]: https://img.shields.io/github/license/bedrock-crustaceans/chorus?style=flat-square
[license_url]: LICENSE

<!-- BADGES -->
