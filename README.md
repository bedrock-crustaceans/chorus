<br />
<div align="center">

[![Chorus-OSS][chorus_logo_url]][chorus_url]

# Chorus

Open-source Minecraft: Bedrock Edition server software, written in Rust

[![rust][rust_badge_url]][rust_url]
[![minecraft][minecraft_badge_url]][minecraft_url]
[![protocol][protocol_badge_url]][protocol_url]
[![license][license_badge_url]][license_url]

</div>

Chorus is a Minecraft Bedrock Edition server written in Rust. It is built on [`bedrock-rs`](https://github.com/bedrock-crustaceans/bedrock-rs), a foundational library for Bedrock tooling, and uses Bevy ECS to drive its tick loop and scheduling.

## Features

- **Performance.** Chorus is optimized for memory and speed, and massively outperforms other Bedrock servers under the same conditions. It runs equally well on one thread or many, scaling with the resources it is given.
- **Extensibility.** Chorus takes full advantage of Bevy's architecture to be as customizable as possible. Replace existing behavior without forking, disable what you don't need for a leaner build, or use Chorus as a library for your own server software, as [Pyrite](https://github.com/bedrock-crustaceans/pyrite) (Beta 1.7.3 for Bedrock) does.
- **World generation.** Fully accurate Bedrock world generation in the Overworld, Nether and End:
  - Terrain, caves, aquifers, ore veins and decoration.
  - Every vanilla structure, from villages, mansions and trial chambers to Nether fortresses, bastions and end cities.

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

## Contributors

<p align="center">
  <a href="https://github.com/bedrock-crustaceans/chorus/graphs/contributors">
    <img src=".github/assets/contributors.svg" alt="Contributors" width="100%" />
  </a>
</p>

## License

Chorus is licensed under the [Apache License 2.0](LICENSE).

<!-- CHORUS -->

[chorus_logo_url]: .github/img/chorus.256.png
[chorus_url]: https://github.com/bedrock-crustaceans/chorus

<!-- CHORUS -->

<!-- BADGES -->

[minecraft_badge_url]: https://img.shields.io/badge/minecraft-26.60-black?style=flat-square
[minecraft_url]: https://www.minecraft.net/en-us/article/minecraft--bedrock-edition-26-60-changelog
[protocol_badge_url]: https://img.shields.io/badge/protocol-v2225-white?style=flat-square
[protocol_url]: https://github.com/Mojang/bedrock-protocol-docs
[rust_badge_url]: https://img.shields.io/badge/rust-2024-%23D34516?style=flat-square&logo=rust&logoColor=%23D34516&labelColor=white
[rust_url]: https://rust-lang.org/
[license_badge_url]: https://img.shields.io/github/license/bedrock-crustaceans/chorus?style=flat-square
[license_url]: LICENSE

<!-- BADGES -->
