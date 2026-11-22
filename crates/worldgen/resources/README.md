# Worldgen resources

Data we write ourselves, embedded in the binary by `build.rs` (it walks this folder, so a new file needs no
registration). Everything that comes from Bedrock Dedicated Server 1.26.60.6 is in the `bedrock_vanilla` crate, see its
README.

| Path                         | Contents                                                                                          |
|------------------------------|---------------------------------------------------------------------------------------------------|
| `features/<id>.json`         | Features BDS implements in code and ships no document for, one file per feature, named after its identifier without `minecraft:`. Same layout as the vanilla feature documents, with Chorus types (`chorus:block_pile`, `disk`, `seagrass`, `kelp`, `spread_plant`, `random_walk_plant`, `water_edge_column`, `mushroom`). |
| `biomes.json`                | The biome registry: `generated` lists the biomes the overworld generator places (modern name, optional pack name when it differs, Bedrock id) and `other` the remaining pack biomes (nether, end, legacy) that structure tags and `/locate` resolve to an id. A `Biome` is its name (a `CowArc<'static, str>` key); chunk data stores the Bedrock id, and the registry converts between the two. `build.rs` generates a `Biome::NAME` constant per generated row and `Biome::ALL` from it, in file order. |
| `structure_sets/<name>.json` | One structure's placement: identifier, kind (the identifier of a registered structure builder, the 15 built-in names or one a plugin registers), rule, placement, reach, biome filter, terrain adaptation and optional `biome_tags`. |

All Chorus-owned keys, enum values and kind names are snake_case.

`biome_tags` is a map of tag name to a list of Bedrock pack biome names (`"beach"`, `"cold_beach"`, ...). Tags are merged
over the `has_structure_*` tags of the vanilla biome documents, a set tag replacing a vanilla tag of the same name. An
unknown biome name or a tag defined by two sets fails the load.

The structure templates are not encoded at build time. `chorus_structure_data::TemplateStore` parses the `structures` table of the vanilla pack (and of any other pack in the `PackSet`) when the structure data loads, about 90 ms
with every core, and the block states are interned into the state pool at the same time.
