use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Read};

use bedrock::level::db::Database;
use bedrock::level::subchunk::{self, SubChunk, to_offset};
use bedrock::level::types::BlockPosition;

use super::{Bedrock, OverworldGenerator};
use chorus_block::block_registry::BlockRegistry;
use chorus_block::r#impl::DEFINITIONS;
use chorus_block::state::block_state::BlockState;
use chorus_level::dimension_type::DimensionType;
use chorus_level::generator::dimension::Dimension;

const MIN_SUB_CHUNK: i8 = -4;
const SUB_CHUNKS: i8 = 24;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Layer {
    Base,
    Surface,
    Feature,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Matter {
    Air,
    Fluid,
    Solid,
}

fn bare(name: &str) -> &str {
    let name = name.split('[').next().unwrap_or(name);
    name.strip_prefix("minecraft:").unwrap_or(name)
}

fn layer(name: &str) -> Layer {
    match bare(name) {
        "air" | "stone" | "deepslate" | "water" | "lava" | "bedrock" | "tuff" | "granite" | "diorite" | "andesite" | "dirt" | "gravel" | "calcite" => Layer::Base,
        "grass_block"
        | "sand"
        | "red_sand"
        | "sandstone"
        | "red_sandstone"
        | "podzol"
        | "mycelium"
        | "mud"
        | "snow"
        | "snow_layer"
        | "ice"
        | "packed_ice"
        | "blue_ice"
        | "powder_snow"
        | "coarse_dirt"
        | "hardened_clay"
        | "stained_hardened_clay"
        | "orange_terracotta"
        | "yellow_terracotta"
        | "brown_terracotta"
        | "red_terracotta"
        | "white_terracotta"
        | "light_gray_terracotta"
        | "terracotta"
        | "clay"
        | "moss_block"
        | "pale_moss_block"
        | "dirt_with_roots"
        | "suspicious_sand"
        | "suspicious_gravel" => Layer::Surface,
        _ => Layer::Feature,
    }
}

fn matter(name: &str) -> Matter {
    match bare(name) {
        "air" | "cave_air" | "void_air" => Matter::Air,
        "water" | "flowing_water" | "lava" | "flowing_lava" => Matter::Fluid,
        _ => Matter::Solid,
    }
}

fn value_text(debug: String) -> String {
    let inner = debug.split_once('(').map(|(_, rest)| rest.strip_suffix(')').unwrap_or(rest)).unwrap_or(&debug);
    inner.trim_matches('"').to_owned()
}

fn canonical(name: &str, states: impl Iterator<Item = (String, String)>) -> String {
    let mut states: Vec<(String, String)> = states.collect();
    if states.is_empty() {
        return name.to_owned();
    }
    states.sort();
    let joined: Vec<String> = states.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("{name}[{}]", joined.join(","))
}

fn chunk_key(x: i32, z: i32, tag: u8, sub: Option<i8>) -> Vec<u8> {
    let mut key = Vec::with_capacity(10);
    key.extend_from_slice(&x.to_le_bytes());
    key.extend_from_slice(&z.to_le_bytes());
    key.push(tag);
    if let Some(sub) = sub {
        key.push(sub as u8);
    }
    key
}

struct StoredLayer {
    palette: Vec<String>,
    layer: subchunk::Layer,
}

impl StoredLayer {
    fn get(&self, x: u8, y: u8, z: u8) -> &str {
        let index = self.layer.indices().get(to_offset(BlockPosition(x, y, z))).unwrap_or(0);
        &self.palette[index as usize]
    }
}

fn first_layer(data: &[u8]) -> Option<StoredLayer> {
    let sub_chunk = SubChunk::from_disk_greedy(&mut Cursor::new(data)).ok()?;
    let layer = sub_chunk.get_layer(0)?.clone();
    let palette = layer
        .palette()
        .iter()
        .map(|block| canonical(&block.name, block.states.iter().map(|(k, v)| (k.clone(), value_text(format!("{v:?}"))))))
        .collect();
    Some(StoredLayer { palette, layer })
}

fn read_u32(reader: &mut Cursor<&[u8]>) -> Option<u32> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes).ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn biomes_3d(data: &[u8]) -> Vec<[u32; 4096]> {
    let mut reader = Cursor::new(&data[512..]);
    let mut sections: Vec<[u32; 4096]> = Vec::new();
    loop {
        let mut header = [0u8; 1];
        if reader.read_exact(&mut header).is_err() {
            break;
        }
        let bits = (header[0] >> 1) as usize;
        if bits == 0x7f {
            let previous = sections.last().copied().unwrap_or([0; 4096]);
            sections.push(previous);
            continue;
        }
        if bits == 0 {
            let Some(value) = read_u32(&mut reader) else { break };
            sections.push([value; 4096]);
            continue;
        }
        let per_word = 32 / bits;
        let words = 4096usize.div_ceil(per_word);
        let mut indices = [0u32; 4096];
        for word in 0..words {
            let Some(packed) = read_u32(&mut reader) else { return sections };
            for slot in 0..per_word {
                let index = word * per_word + slot;
                if index < 4096 {
                    indices[index] = (packed >> (slot * bits)) & ((1 << bits) - 1);
                }
            }
        }
        let Some(len) = read_u32(&mut reader) else { break };
        let palette: Vec<u32> = (0..len).filter_map(|_| read_u32(&mut reader)).collect();
        sections.push(indices.map(|index| palette.get(index as usize).copied().unwrap_or(u32::MAX)));
    }
    sections
}

struct Tally {
    total: usize,
    equal: usize,
    matter_equal: usize,
    by_layer: BTreeMap<Layer, (usize, usize)>,
    pairs: HashMap<(String, String), usize>,
    by_band: BTreeMap<i32, (usize, usize)>,
    heights: (usize, usize, i64),
    biomes: (usize, usize),
    biome_pairs: HashMap<(u32, u32), usize>,
}

#[test]
#[ignore]
fn bds_parity() {
    let db_path = std::env::var("BDS_DB").expect("set BDS_DB to a bedrock world's db directory");
    let seed: i64 = std::env::var("BDS_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(12345);
    let area: Vec<i32> = std::env::var("BDS_AREA")
        .unwrap_or_else(|_| "0,0,9,9".into())
        .split(',')
        .map(|v| v.trim().parse().expect("BDS_AREA is x0,z0,x1,z1 in chunks"))
        .collect();
    let (x0, z0, x1, z1) = (area[0], area[1], area[2], area[3]);

    let database = Database::open(&db_path).expect("open bedrock db");
    let mut registry = BlockRegistry::new();
    registry.register_all(DEFINITIONS.iter().copied());
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let mut dimension = Dimension::new(DimensionType::Overworld, OverworldGenerator::<Bedrock>::new(seed, &registry));
    let positions: Vec<(i32, i32)> = (x0..=x1).flat_map(|x| (z0..=z1).map(move |z| (x, z))).collect();
    dimension.request_chunks(&positions);
    let mut done = 0;
    while done < positions.len() {
        done += dimension.tick().len();
    }

    let mut ours_names: HashMap<i32, String> = HashMap::new();
    let mut ours_name = |runtime: i32| -> String {
        ours_names
            .entry(runtime)
            .or_insert_with(|| match registry.get_permutation(runtime) {
                Some(permutation) => canonical(
                    permutation.get_identifier(),
                    permutation.get_states().iter().map(|(k, v)| {
                        let text = match v {
                            BlockState::Bool(b) => (*b as u8).to_string(),
                            BlockState::Int(i) => i.to_string(),
                            BlockState::Enum(s) => s.to_string(),
                        };
                        (k.to_string(), text)
                    }),
                ),
                None => format!("unknown#{runtime}"),
            })
            .clone()
    };
    let biome_names: HashMap<u32, String> = super::shared::biome::Biome::ALL.iter().map(|biome| (biome.bedrock_id() as u32, biome.name().to_owned())).collect();

    let mut tally = Tally {
        total: 0,
        equal: 0,
        matter_equal: 0,
        by_layer: BTreeMap::new(),
        pairs: HashMap::new(),
        by_band: BTreeMap::new(),
        heights: (0, 0, 0),
        biomes: (0, 0),
        biome_pairs: HashMap::new(),
    };
    let mut missing = Vec::new();
    for &(cx, cz) in &positions {
        let mut theirs: Vec<Option<StoredLayer>> = Vec::new();
        for sub in MIN_SUB_CHUNK..MIN_SUB_CHUNK + SUB_CHUNKS {
            let data = database.get(chunk_key(cx, cz, 0x2f, Some(sub))).ok().flatten().map(Vec::<u8>::from);
            theirs.push(data.and_then(|bytes| first_layer(&bytes)));
        }
        if theirs.iter().all(Option::is_none) {
            missing.push((cx, cz));
            continue;
        }
        let ours = dimension.get_chunk(cx, cz).expect("generated");
        let their_biomes = database
            .get(chunk_key(cx, cz, 0x2b, None))
            .ok()
            .flatten()
            .map(|buffer| biomes_3d(&Vec::<u8>::from(buffer)))
            .unwrap_or_default();
        for lx in 0..16u8 {
            for lz in 0..16u8 {
                let (mut our_top, mut their_top) = (None, None);
                for y in (-64..320).rev() {
                    let sub = ((y >> 4) - MIN_SUB_CHUNK as i32) as usize;
                    let ly = (y & 15) as u8;
                    let their = theirs[sub].as_ref().map_or("minecraft:air", |layer| layer.get(lx, ly, lz)).to_owned();
                    let our = ours_name(ours.get_block(lx, y, lz, 0).unwrap_or(0));
                    if our_top.is_none() && matter(&our) != Matter::Air && layer(&our) != Layer::Feature {
                        our_top = Some(y);
                    }
                    if their_top.is_none() && matter(&their) != Matter::Air && layer(&their) != Layer::Feature {
                        their_top = Some(y);
                    }
                    tally.total += 1;
                    let band = tally.by_band.entry(y.div_euclid(32) * 32).or_default();
                    band.0 += 1;
                    let layer = layer(&our).max(layer(&their));
                    let entry = tally.by_layer.entry(layer).or_default();
                    entry.0 += 1;
                    if matter(&our) == matter(&their) {
                        tally.matter_equal += 1;
                    }
                    if our == their {
                        tally.equal += 1;
                        entry.1 += 1;
                        band.1 += 1;
                    } else {
                        *tally.pairs.entry((bare(&our).to_owned(), bare(&their).to_owned())).or_default() += 1;
                    }
                    if let Some(section) = their_biomes.get(sub) {
                        let their_biome = section[((lx as usize) << 8) | ((lz as usize) << 4) | ly as usize];
                        let our_biome = ours.get_biome(lx, y, lz).unwrap_or(-1) as u32;
                        tally.biomes.0 += 1;
                        if our_biome == their_biome {
                            tally.biomes.1 += 1;
                        } else {
                            *tally.biome_pairs.entry((our_biome, their_biome)).or_default() += 1;
                        }
                    }
                }
                tally.heights.0 += 1;
                let (our_top, their_top) = (our_top.unwrap_or(-64), their_top.unwrap_or(-64));
                if our_top == their_top {
                    tally.heights.1 += 1;
                }
                tally.heights.2 += (our_top - their_top).abs() as i64;
            }
        }
    }

    let percent = |part: usize, whole: usize| if whole == 0 { 0.0 } else { part as f64 * 100.0 / whole as f64 };
    let compared = positions.len() - missing.len();
    println!(
        "bds parity, seed {seed}, chunks {x0},{z0}..{x1},{z1}: {compared} compared, {} skipped (not saved by bds)",
        missing.len()
    );
    if !missing.is_empty() {
        println!("  skipped: {missing:?}");
    }
    println!("  blocks identical      {:6.2}%", percent(tally.equal, tally.total));
    println!("  air/fluid/solid match {:6.2}%", percent(tally.matter_equal, tally.total));
    println!(
        "  terrain height match  {:6.2}% of columns, mean |dy| {:.2}",
        percent(tally.heights.1, tally.heights.0),
        tally.heights.2 as f64 / tally.heights.0.max(1) as f64
    );
    println!("  biome match           {:6.2}%", percent(tally.biomes.1, tally.biomes.0));
    println!("  by layer (worst of ours/theirs):");
    for (layer, (total, equal)) in &tally.by_layer {
        println!("    {layer:?}: {:6.2}% of {total}", percent(*equal, *total));
    }
    println!("  by y band:");
    for (band, (total, equal)) in &tally.by_band {
        println!("    y {band:4}..{:4}: {:6.2}%", band + 31, percent(*equal, *total));
    }
    let mut pairs: Vec<_> = tally.pairs.into_iter().collect();
    pairs.sort_by_key(|pair| std::cmp::Reverse(pair.1));
    println!("  top block mismatches (ours -> bedrock):");
    for ((ours, theirs), count) in pairs.iter().take(30) {
        println!("    {count:8}  {ours} -> {theirs}");
    }
    let mut biome_pairs: Vec<_> = tally.biome_pairs.into_iter().collect();
    biome_pairs.sort_by_key(|pair| std::cmp::Reverse(pair.1));
    println!("  top biome mismatches (ours -> bedrock):");
    let biome = |id: u32| biome_names.get(&id).cloned().unwrap_or_else(|| format!("#{id}"));
    for ((ours, theirs), count) in biome_pairs.iter().take(15) {
        println!("    {count:8}  {} -> {}", biome(*ours), biome(*theirs));
    }
}

#[test]
#[ignore]
fn bds_probe() {
    let db_path = std::env::var("BDS_DB").expect("set BDS_DB");
    let seed: i64 = std::env::var("BDS_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(12345);
    let area: Vec<i32> = std::env::var("BDS_AREA").unwrap_or_else(|_| "0,0,9,9".into()).split(',').map(|v| v.trim().parse().unwrap()).collect();
    let database = Database::open(&db_path).expect("open bedrock db");
    let mut registry = BlockRegistry::new();
    registry.register_all(DEFINITIONS.iter().copied());
    let generator = OverworldGenerator::<Bedrock>::new(seed, &registry);
    let (mut cells, mut uniform) = (0usize, 0usize);
    let mut offsets: BTreeMap<(i32, i32, i32), (usize, usize)> = BTreeMap::new();
    let mut floor_theirs = [0usize; 6];
    let mut floor_ours = [0usize; 6];
    let mut floor_columns = 0usize;
    let mut pattern = String::new();
    for cx in area[0]..=area[2] {
        for cz in area[1]..=area[3] {
            let Some(raw) = database.get(chunk_key(cx, cz, 0x2b, None)).ok().flatten() else { continue };
            let biomes = biomes_3d(&Vec::<u8>::from(raw));
            let quarts: HashMap<(i32, i32), _> = (-1..=1)
                .flat_map(|dx| (-1..=1).map(move |dz| (dx, dz)))
                .map(|(dx, dz)| ((dx, dz), generator.quart_biomes(cx + dx, cz + dz)))
                .collect();
            let quart = |qx: i32, qy: i32, qz: i32| {
                let (dx, dz) = (qx.div_euclid(4), qz.div_euclid(4));
                quarts[&(dx, dz)].get(qx.rem_euclid(4), qy.clamp(0, 95), qz.rem_euclid(4)).bedrock_id() as u32
            };
            for (section, values) in biomes.iter().enumerate().take(24) {
                for qx in 0..4 {
                    for qz in 0..4 {
                        for qyl in 0..4 {
                            let first = values[((qx * 4) << 8) | ((qz * 4) << 4) | (qyl * 4)];
                            let mut same = true;
                            for x in 0..4 {
                                for z in 0..4 {
                                    for y in 0..4 {
                                        same &= values[((qx * 4 + x) << 8) | ((qz * 4 + z) << 4) | (qyl * 4 + y)] == first;
                                    }
                                }
                            }
                            cells += 1;
                            if !same {
                                continue;
                            }
                            uniform += 1;
                            let qy = section as i32 * 4 + qyl as i32;
                            for ox in -1..=1 {
                                for oy in -1..=1 {
                                    for oz in -1..=1 {
                                        let entry = offsets.entry((ox, oy, oz)).or_default();
                                        entry.0 += 1;
                                        if quart(qx as i32 + ox, qy + oy, qz as i32 + oz) == first {
                                            entry.1 += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            let Some(bottom) = database.get(chunk_key(cx, cz, 0x2f, Some(-4))).ok().flatten() else { continue };
            let Some(layer) = first_layer(&Vec::<u8>::from(bottom)) else { continue };
            let (mut chunk, noise, mut aquifer) = generator.standalone_noise(cx, cz);
            generator.build_surface(&noise, &mut chunk);
            generator.apply_carvers(&noise, &mut aquifer, &mut chunk);
            for x in 0..16u8 {
                for z in 0..16u8 {
                    floor_columns += 1;
                    for dy in 0..6u8 {
                        if bare(layer.get(x, dy, z)) == "bedrock" {
                            floor_theirs[dy as usize] += 1;
                        }
                        if generator.blocks.name_of(chunk.get(x as i32, -64 + dy as i32, z as i32)) == chorus_block::block_id::BEDROCK {
                            floor_ours[dy as usize] += 1;
                        }
                    }
                }
            }
            if (cx, cz) == (area[0], area[1]) {
                for dy in 1..5u8 {
                    pattern.push_str(&format!("  y{}:\n", -64 + dy as i32));
                    for z in 0..16u8 {
                        let row: String = (0..16u8).map(|x| if bare(layer.get(x, dy, z)) == "bedrock" { '#' } else { '.' }).collect();
                        pattern.push_str(&format!("    {row}\n"));
                    }
                }
            }
        }
    }
    println!("biome cells uniform in bedrock: {uniform}/{cells}");
    let mut ranked: Vec<_> = offsets.into_iter().map(|(offset, (total, hit))| (hit as f64 / total.max(1) as f64, offset)).collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (rate, offset) in ranked.iter().take(5) {
        println!("  quart offset {offset:?}: {:.2}% of uniform cells match", rate * 100.0);
    }
    println!("bedrock floor share per y (bedrock vs ours) over {floor_columns} columns:");
    for dy in 0..6 {
        println!(
            "  y {}: {:.3} vs {:.3}",
            -64 + dy as i32,
            floor_theirs[dy] as f64 / floor_columns as f64,
            floor_ours[dy] as f64 / floor_columns as f64
        );
    }
    println!("bedrock floor pattern in chunk {},{} (x across, z down):\n{pattern}", area[0], area[1]);
}

#[test]
#[ignore]
fn bds_fluid_probe() {
    let db_path = std::env::var("BDS_DB").expect("set BDS_DB");
    let seed: i64 = std::env::var("BDS_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(12345);
    let database = Database::open(&db_path).expect("open bedrock db");
    let mut registry = BlockRegistry::new();
    registry.register_all(DEFINITIONS.iter().copied());
    let generator = OverworldGenerator::<Bedrock>::new(seed, &registry);
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_y: BTreeMap<i32, usize> = BTreeMap::new();
    for cx in 1..=8 {
        for cz in 1..=8 {
            let theirs: Vec<Option<StoredLayer>> = (MIN_SUB_CHUNK..MIN_SUB_CHUNK + SUB_CHUNKS)
                .map(|sub| database.get(chunk_key(cx, cz, 0x2f, Some(sub))).ok().flatten().and_then(|b| first_layer(&Vec::<u8>::from(b))))
                .collect();
            let their = |x: i32, y: i32, z: i32| -> Matter {
                if !(0..16).contains(&x) || !(0..16).contains(&z) || !(-64..320).contains(&y) {
                    return Matter::Solid;
                }
                theirs[((y >> 4) + 4) as usize]
                    .as_ref()
                    .map_or(Matter::Air, |layer| matter(layer.get(x as u8, (y & 15) as u8, z as u8)))
            };
            let (noise_only, _, _) = generator.standalone_noise(cx, cz);
            let (mut carved, noise, mut aquifer) = generator.standalone_noise(cx, cz);
            generator.build_surface(&noise, &mut carved);
            generator.apply_carvers(&noise, &mut aquifer, &mut carved);
            let ours = |chunk: &super::shared::proto::ProtoChunk, x: i32, y: i32, z: i32| -> Matter {
                if !(0..16).contains(&x) || !(0..16).contains(&z) {
                    return Matter::Solid;
                }
                matter(generator.blocks.name_of(chunk.get(x, y, z)))
            };
            for x in 1..15 {
                for z in 1..15 {
                    for y in -60..64 {
                        let (o, t) = (ours(&carved, x, y, z), their(x, y, z));
                        if o == t || o == Matter::Air || t == Matter::Air {
                            continue;
                        }
                        let near_air = |f: &dyn Fn(i32, i32, i32) -> Matter| {
                            [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)]
                                .iter()
                                .any(|&(dx, dy, dz)| f(x + dx, y + dy, z + dz) != Matter::Solid)
                        };
                        let noise_fill = ours(&noise_only, x, y, z);
                        let kind = match (o == Matter::Fluid, noise_fill == Matter::Fluid, near_air(&|a, b, c| their(a, b, c))) {
                            (true, true, _) => "ours fluid from noise fill (aquifer), bedrock solid",
                            (true, false, _) => "ours fluid from carver, bedrock solid",
                            (false, _, true) => "ours solid, bedrock fluid next to its open space",
                            (false, _, false) => "ours solid, bedrock fluid enclosed",
                        };
                        *counts.entry(kind).or_default() += 1;
                        *by_y.entry(y.div_euclid(16) * 16).or_default() += 1;
                    }
                }
            }
        }
    }
    println!("fluid/solid disagreements: {counts:#?}");
    println!("by y: {by_y:?}");
}
