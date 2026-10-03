use std::time::Instant;

use super::{Java, OverworldGenerator};
use crate::block::r#impl::DEFINITIONS;
use crate::registry::block_registry::BlockRegistry;

fn generator(seed: i64) -> (OverworldGenerator<Java>, BlockRegistry) {
    let mut registry = BlockRegistry::new();
    registry.register_all(DEFINITIONS.iter().copied());
    let start = Instant::now();
    let generator = OverworldGenerator::new(seed, &registry);
    println!("generator built in {:?}", start.elapsed());
    (generator, registry)
}

#[test]
#[ignore]
fn output_checksum() {
    use crate::level::dimension_type::DimensionType;
    use crate::level::generator::dimension::Dimension;
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let (generator, _) = generator(0);
    let mut dimension = Dimension::new(DimensionType::Overworld, generator);
    let centers = [(0, 0), (48, -100), (44, 200), (-36, 108), (-156, -92), (44, 52), (-92, -144), (100, 68)];
    let positions: Vec<(i32, i32)> = centers.iter().flat_map(|&(cx, cz)| (-2..=2).flat_map(move |dx| (-2..=2).map(move |dz| (cx + dx, cz + dz)))).collect();
    dimension.request_chunks(&positions);
    let mut done = 0;
    while done < positions.len() {
        done += dimension.tick().len();
        std::thread::yield_now();
    }
    let mut total: u64 = 0xcbf29ce484222325;
    for &(x, z) in &positions {
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in dimension.get_chunk(x, z).expect("generated").serialize() {
            hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
        }
        total = (total ^ hash).wrapping_mul(0x100000001b3);
    }
    println!("checksum {total:016x} over {} chunks", positions.len());
}

#[test]
#[ignore]
fn pipeline() {
    use crate::level::dimension_type::DimensionType;
    use crate::level::generator::dimension::Dimension;
    let threads: usize = std::env::var("PIPELINE_THREADS").ok().and_then(|t| t.parse().ok()).unwrap_or(0);
    bevy_tasks::AsyncComputeTaskPool::get_or_init(|| {
        if threads == 0 {
            bevy_tasks::TaskPool::default()
        } else {
            bevy_tasks::TaskPoolBuilder::new().num_threads(threads).build()
        }
    });
    let (generator, _) = generator(0);
    let mut dimension = Dimension::new(DimensionType::Overworld, generator);
    let radius: i32 = std::env::var("PIPELINE_RADIUS").ok().and_then(|r| r.parse().ok()).unwrap_or(4);
    let positions: Vec<(i32, i32)> = (-radius..=radius).flat_map(|x| (-radius..=radius).map(move |z| (x, z))).collect();
    let start = Instant::now();
    dimension.request_chunks(&positions);
    let mut done = 0;
    while done < positions.len() {
        done += dimension.tick().len();
        std::thread::yield_now();
    }
    let elapsed = start.elapsed();
    println!("{} chunks in {elapsed:?} ({:?}/chunk)", positions.len(), elapsed / positions.len() as u32);
    assert!(positions.iter().all(|&(x, z)| dimension.get_chunk(x, z).is_some()));
}

#[test]
#[ignore]
fn biome_search_matches_cold_search() {
    let (generator, _) = generator(0);
    let mut hint = super::biome::BiomeHint::default();
    let mut column_hints = generator.biomes.column_hints();
    let mut checked = 0;
    for qz in -40..40 {
        for qx in -40..40 {
            for qy in (0..super::HEIGHT / 4).step_by(3) {
                let climate = generator.terrain.climate(qx * 52, super::MIN_Y + qy * 4, qz * 52);
                let target = [climate.temperature, climate.vegetation, climate.continents, climate.erosion, climate.depth, climate.ridges];
                assert_eq!(generator.biomes.find_near(target, &mut hint), generator.biomes.find(target));
                checked += 1;
            }
            let column = generator.terrain.surface_climate(qx * 52, qz * 52);
            let climate = column.at(super::MIN_Y);
            let ys: Vec<i32> = (-1..=super::HEIGHT / 4).map(|qy| super::MIN_Y + qy * 4).collect();
            let depths: Vec<f32> = ys.iter().map(|&y| column.at(y).depth).collect();
            let mut found = vec![super::biome::Biome::Plains; ys.len()];
            let fixed = [climate.temperature, climate.vegetation, climate.continents, climate.erosion, climate.ridges];
            generator.biomes.find_column(fixed, &depths, &mut column_hints, &mut found);
            for (&y, &biome) in ys.iter().zip(&found) {
                let climate = column.at(y);
                let target = [climate.temperature, climate.vegetation, climate.continents, climate.erosion, climate.depth, climate.ridges];
                assert_eq!(biome, generator.biomes.find(target), "column {qx},{qz} y {y}");
                checked += 1;
            }
        }
    }
    println!("{checked} lookups matched");
}

#[test]
#[ignore]
fn zoom_table_matches() {
    let (generator, _) = generator(0);
    for (x, z) in [(0, 0), (-3, 7), (12, -5), (44, 200)] {
        let (chunk, _, _) = generator.standalone_noise(x, z);
        assert!(chunk.zoom_matches_direct(), "chunk {x},{z}");
    }
}

#[test]
#[ignore]
fn stage_timings() {
    use std::time::Duration;
    let (generator, _) = generator(0);
    let mut totals = [Duration::ZERO; 6];
    let (count_x, count_z) = (6, 6);
    let mut carved = std::collections::HashMap::new();
    for x in -1..=count_x {
        for z in -1..=count_z {
            let around: Vec<_> = (0..9).map(|i| generator.quart_biomes(x + i / 3 - 1, z + i % 3 - 1)).collect();
            let start = Instant::now();
            let (mut chunk, noise, mut aquifer) = generator.fill_noise(x, z, std::array::from_fn(|i| &around[i]), generator.terrain.noise_chunk(x, z));
            let after_noise = Instant::now();
            generator.build_surface(&noise, &mut chunk);
            let after_surface = Instant::now();
            generator.apply_carvers(&noise, &mut aquifer, &mut chunk);
            let after_carvers = Instant::now();
            totals[0] += after_noise - start;
            totals[1] += after_surface - after_noise;
            totals[2] += after_carvers - after_surface;
            carved.insert((x, z), chunk);
        }
    }
    for x in 0..count_x {
        for z in 0..count_z {
            let start = Instant::now();
            let region: [_; 9] = std::array::from_fn(|i| carved[&(x + i as i32 / 3 - 1, z + i as i32 % 3 - 1)].clone());
            let after_clone = Instant::now();
            let changes = generator.decorate(x, z, region);
            let after_decorate = Instant::now();
            let chunk = carved[&(x, z)].clone();
            let mut column = chunk.clone();
            changes.overlay_onto(x, z, &chunk, &mut column);
            let after_overlay = Instant::now();
            let _ = column.into_chunk(&generator.blocks);
            totals[3] += after_clone - start;
            totals[4] += after_decorate - after_clone;
            totals[5] += Instant::now() - after_overlay;
        }
    }
    let start = Instant::now();
    for x in 0..count_x {
        for z in 0..count_z {
            std::hint::black_box(generator.quart_biomes(x, z));
        }
    }
    println!("{:>12}: {:?}/chunk", "biomes", start.elapsed() / (count_x * count_z) as u32);
    let mut targets = Vec::new();
    let start = Instant::now();
    for qz in 0..24 {
        for qx in 0..24 {
            for qy in 0..super::HEIGHT / 4 {
                let climate = generator.terrain.climate(qx * 4, super::MIN_Y + qy * 4, qz * 4);
                targets.push([climate.temperature, climate.vegetation, climate.continents, climate.erosion, climate.depth, climate.ridges]);
            }
        }
    }
    let per_sample = start.elapsed() / targets.len() as u32;
    let start = Instant::now();
    for target in &targets {
        std::hint::black_box(generator.biomes.find(*target));
    }
    println!("{:>12}: climate {per_sample:?}, find {:?} per quart", "split", start.elapsed() / targets.len() as u32);
    let sample = &carved[&(0, 0)];
    let start = Instant::now();
    for y in super::MIN_Y..super::MIN_Y + super::HEIGHT {
        for z in 0..16 {
            for x in 0..16 {
                std::hint::black_box(sample.biome_at(x, y, z));
            }
        }
    }
    println!("{:>12}: {:?}/chunk", "zoom all", start.elapsed());
    let carved_count = ((count_x + 2) * (count_z + 2)) as u32;
    let decorated_count = (count_x * count_z) as u32;
    for (name, total, count) in [
        ("noise", totals[0], carved_count),
        ("surface", totals[1], carved_count),
        ("carvers", totals[2], carved_count),
        ("region clone", totals[3], decorated_count),
        ("decorate", totals[4], decorated_count),
        ("into_chunk", totals[5], decorated_count),
    ] {
        println!("{name:>12}: {:?}/chunk", total / count);
    }
}

#[test]
#[ignore]
fn terrain_overview() {
    let (generator, _) = generator(0);
    let radius = 4;
    let start = Instant::now();
    let mut chunks = Vec::new();
    for cz in -radius..radius {
        for cx in -radius..radius {
            let (mut chunk, noise, mut aquifer) = generator.standalone_noise(cx, cz);
            generator.build_surface(&noise, &mut chunk);
            generator.apply_carvers(&noise, &mut aquifer, &mut chunk);
            chunks.push(chunk);
        }
    }
    let count = chunks.len();
    println!("{count} chunks in {:?} ({:?}/chunk)", start.elapsed(), start.elapsed() / count as u32);

    let side = (radius * 2) as usize;
    for row in 0..side * 16 / 2 {
        let mut line = String::new();
        for col in 0..side * 16 {
            let (bz, bx) = (row * 2, col);
            let chunk = &chunks[(bz / 16) * side + bx / 16];
            let (x, z) = ((bx % 16) as i32, (bz % 16) as i32);
            let y = chunk.surface_height(x, z);
            let block = chunk.get(x, y, z);
            let c = if block == generator.water {
                '~'
            } else {
                let name = chunk.biome_at(x, y, z).name().as_bytes()[0] as char;
                if y > 120 { name.to_ascii_uppercase() } else { name }
            };
            line.push(c);
        }
        println!("{line}");
    }
}

#[test]
#[ignore]
fn biome_overview() {
    use super::biome::Biome;
    let (generator, _) = generator(0);
    let glyphs = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%&*+=?";
    let mut counts = std::collections::HashMap::new();
    for row in -40..40 {
        let mut line = String::new();
        for col in -80..80 {
            let (x, z) = (col * 48, row * 96);
            let y = generator.terrain.preliminary_surface_level(x, z) as i32;
            let climate = generator.terrain.climate(x, y.max(super::SEA_LEVEL), z);
            let biome = generator
                .biomes
                .find([climate.temperature, climate.vegetation, climate.continents, climate.erosion, climate.depth, climate.ridges]);
            *counts.entry(biome.name()).or_insert(0) += 1;
            let index = Biome::ALL.iter().position(|b| *b == biome).unwrap();
            line.push(glyphs.as_bytes()[index] as char);
        }
        println!("{line}");
    }
    let mut counts: Vec<_> = counts.into_iter().collect();
    counts.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    println!("{counts:?}");
}

#[test]
#[ignore]
fn carver_volume() {
    let mut carved = 0;
    for cz in -4..4 {
        for cx in -4..4 {
            super::carver::carve(0, cx, cz).visit(|_, _, bottom, top| carved += top - bottom + 1);
        }
    }
    println!("carved {carved} cells over 64 chunks");
}

#[test]
#[ignore]
fn feature_counts() {
    let (generator, _) = generator(0);
    let carved = |x: i32, z: i32| {
        let (mut chunk, noise, mut aquifer) = generator.standalone_noise(x, z);
        generator.build_surface(&noise, &mut chunk);
        generator.apply_carvers(&noise, &mut aquifer, &mut chunk);
        chunk
    };
    let mut counts = std::collections::BTreeMap::new();
    let start = Instant::now();
    let owners = 4;
    let (base_x, base_z) = std::env::var("FEATURE_ORIGIN")
        .ok()
        .and_then(|origin| origin.split_once(',').and_then(|(x, z)| Some((x.parse().ok()?, z.parse().ok()?))))
        .unwrap_or((0, 0));
    println!("biome at origin: {:?}", carved(base_x, base_z).biome_at(8, 63, 8));
    for ox in base_x..base_x + owners {
        for oz in base_z..base_z + owners {
            let region: [_; 9] = std::array::from_fn(|i| carved(ox + i as i32 / 3 - 1, oz + i as i32 % 3 - 1));
            let baseline = carved(ox, oz);
            let changes = generator.decorate(ox, oz, region);
            let mut column = baseline.clone();
            changes.overlay_onto(ox, oz, &baseline, &mut column);
            for y in column.min_y..=column.max_y() {
                for z in 0..16 {
                    for x in 0..16 {
                        let (before, after) = (baseline.get(x, y, z), column.get(x, y, z));
                        if before != after {
                            *counts.entry(generator.blocks.name_of(after).to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }
    println!("decorated {} owners in {:?}", owners * owners, start.elapsed());
    println!("{counts:?}");
}

#[test]
#[ignore]
fn biome_locations() {
    let (generator, _) = generator(0);
    let mut found = std::collections::BTreeMap::new();
    for ring in 0i32..120 {
        for cz in -ring..=ring {
            for cx in -ring..=ring {
                if cx.abs() != ring && cz.abs() != ring {
                    continue;
                }
                let (x, z) = (cx * 64 + 8, cz * 64 + 8);
                let y = generator.terrain.preliminary_surface_level(x, z) as i32;
                let climate = generator.terrain.climate(x, y.max(super::SEA_LEVEL), z);
                let biome = generator
                    .biomes
                    .find([climate.temperature, climate.vegetation, climate.continents, climate.erosion, climate.depth, climate.ridges]);
                found.entry(biome.name()).or_insert((cx * 4, cz * 4));
                let deep = generator.terrain.climate(x, -40, z);
                let deep = generator.biomes.find([deep.temperature, deep.vegetation, deep.continents, deep.erosion, deep.depth, deep.ridges]);
                found.entry(deep.name()).or_insert((cx * 4, cz * 4));
            }
        }
    }
    for (biome, (x, z)) in found {
        println!("{biome}: {x},{z}");
    }
}

#[test]
#[ignore]
fn missing_blocks() {
    let (generator, registry) = generator(0);
    let air = generator.blocks.runtime(super::blocks::AIR);
    let mut missing: Vec<&str> = generator
        .blocks
        .entries()
        .iter()
        .filter(|entry| entry.runtime == air && entry.name != crate::block::block_id::AIR)
        .map(|entry| entry.name)
        .collect();
    missing.sort();
    missing.dedup();
    println!("{missing:?}");
    let inexact: Vec<_> = generator
        .blocks
        .entries()
        .iter()
        .filter(|entry| !entry.states.is_empty() && registry.get_block_id_with_states(entry.name, &entry.states) != Some(entry.runtime))
        .map(|entry| (entry.name, &entry.states))
        .collect();
    println!("{inexact:?}");
}

#[test]
#[ignore]
fn block_entities() {
    let (generator, _) = generator(0);
    let carved = |x: i32, z: i32| {
        let (mut chunk, noise, mut aquifer) = generator.standalone_noise(x, z);
        generator.build_surface(&noise, &mut chunk);
        generator.apply_carvers(&noise, &mut aquifer, &mut chunk);
        chunk
    };
    let mut found = 0;
    for ox in 0..4 {
        for oz in 0..4 {
            let region: [_; 9] = std::array::from_fn(|i| carved(ox + i as i32 / 3 - 1, oz + i as i32 % 3 - 1));
            let baseline = carved(ox, oz);
            let changes = generator.decorate(ox, oz, region);
            let mut column = baseline.clone();
            changes.overlay_onto(ox, oz, &baseline, &mut column);
            let data = column.into_chunk(&generator.blocks).serialize_block_entities(None);
            let text = String::from_utf8_lossy(&data);
            for id in ["MobSpawner", "Chest", "Beehive"] {
                if text.contains(id) {
                    found += 1;
                    println!("chunk {ox},{oz} has {id} ({} bytes)", data.len());
                }
            }
        }
    }
    assert!(found > 0);
}

#[test]
#[ignore]
fn moving_center_generates_nearest_first() {
    use crate::level::dimension_type::DimensionType;
    use crate::level::generator::dimension::Dimension;
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let (generator, _) = generator(0);
    let mut dimension = Dimension::new(DimensionType::Overworld, generator);
    let positions: Vec<(i32, i32)> = (-8..=16).flat_map(|x| (-8..=8).map(move |z| (x, z))).collect();
    let request_around = |dimension: &mut Dimension, center: (i32, i32)| {
        let distance = |&(x, z): &(i32, i32)| ((x - center.0).pow(2) + (z - center.1).pow(2)) as u32;
        let mut ranked: Vec<_> = positions.iter().map(|position| (*position, distance(position))).collect();
        ranked.sort_unstable_by_key(|&(_, rank)| std::cmp::Reverse(rank));
        for ((x, z), rank) in ranked {
            dimension.request_chunk(x, z, rank);
        }
    };
    request_around(&mut dimension, (0, 0));
    let mut done = 0;
    while done < 10 {
        done += dimension.tick().len();
    }
    request_around(&mut dimension, (12, 0));
    let mut after = Vec::new();
    while after.len() < 60 {
        after.extend(dimension.tick().into_iter().map(|(x, z)| (x - 12).pow(2) + z.pow(2)));
    }
    let late = &after[20..60];
    let near = late.iter().filter(|&&distance| distance <= 25).count();
    println!("distances to the new center after moving: {after:?}");
    assert!(near * 10 >= late.len() * 9, "{near} of {} near the new center", late.len());
}

#[test]
#[ignore]
fn survival_audit() {
    use super::blocks::{AIR, BlockId, Blocks};
    use super::features::BlockView;
    use crate::level::dimension_type::DimensionType;
    use crate::level::generator::dimension::Dimension;
    use glam::IVec3;
    use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let (generator, _) = generator(0);
    let generator = std::sync::Arc::new(generator);
    let mut centers = BTreeMap::new();
    for ring in 0i32..60 {
        for cz in -ring..=ring {
            for cx in -ring..=ring {
                if cx.abs() != ring && cz.abs() != ring {
                    continue;
                }
                let (x, z) = (cx * 64 + 8, cz * 64 + 8);
                let y = generator.terrain.preliminary_surface_level(x, z) as i32;
                let climate = generator.terrain.climate(x, y.max(super::SEA_LEVEL), z);
                let biome = generator
                    .biomes
                    .find([climate.temperature, climate.vegetation, climate.continents, climate.erosion, climate.depth, climate.ridges]);
                centers.entry(biome.name()).or_insert((cx * 4, cz * 4));
            }
        }
    }
    let (owned, _) = self::generator(0);
    let mut dimension = Dimension::new(DimensionType::Overworld, owned);
    let mut positions = HashSet::new();
    for &(cx, cz) in centers.values() {
        for dx in -3..=3 {
            for dz in -3..=3 {
                positions.insert((cx + dx, cz + dz));
            }
        }
    }
    let positions: Vec<(i32, i32)> = positions.into_iter().collect();
    dimension.request_chunks(&positions);
    let mut done = 0;
    while done < positions.len() {
        done += dimension.tick().len();
    }
    let blocks = &generator.blocks;
    let mut by_runtime: HashMap<i32, BlockId> = HashMap::new();
    for id in 0..blocks.len() as BlockId {
        if !blocks.entry(id).flooded {
            by_runtime.entry(blocks.runtime(id)).or_insert(id);
        }
    }
    struct World<'a> {
        blocks: &'a Blocks,
        dimension: &'a Dimension,
        by_runtime: &'a HashMap<i32, BlockId>,
    }
    impl BlockView for World<'_> {
        fn blocks(&self) -> &Blocks {
            self.blocks
        }
        fn get(&self, pos: IVec3) -> BlockId {
            self.dimension
                .get_block(pos.x, pos.y, pos.z, 0)
                .and_then(|runtime| self.by_runtime.get(&runtime).copied())
                .unwrap_or(AIR)
        }
    }
    let world = World {
        blocks,
        dimension: &dimension,
        by_runtime: &by_runtime,
    };
    let shapes = generator.edition.shapes(generator.water);
    let mut broken: BTreeMap<&str, (usize, Vec<IVec3>)> = BTreeMap::new();
    let mut leaves: Vec<IVec3> = Vec::new();
    let mut logs: Vec<IVec3> = Vec::new();
    for (biome, &(cx, cz)) in &centers {
        let _ = biome;
        for chunk_x in cx - 1..=cx + 1 {
            for chunk_z in cz - 1..=cz + 1 {
                for y in -64..320 {
                    for lz in 0..16 {
                        for lx in 0..16 {
                            let pos = IVec3::new((chunk_x << 4) + lx, y, (chunk_z << 4) + lz);
                            let state = world.get(pos);
                            if blocks.is_air(state) || blocks.entry(state).liquid {
                                continue;
                            }
                            if blocks.is(state, super::tags::Tag::Leaves) {
                                leaves.push(pos);
                            } else if blocks.is(state, super::tags::Tag::Logs) {
                                logs.push(pos);
                            }
                            let updated = shapes.update_from_neighbours(&world, pos);
                            if blocks.is_air(updated) || updated == generator.water {
                                let entry = broken.entry(blocks.name_of(state)).or_default();
                                entry.0 += 1;
                                if entry.1.len() < 3 {
                                    entry.1.push(pos);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for (name, (count, examples)) in &broken {
        let described: Vec<String> = examples
            .iter()
            .map(|&pos| format!("{pos} on {} under {}", blocks.name_of(world.get(pos - IVec3::Y)), blocks.name_of(world.get(pos + IVec3::Y))))
            .collect();
        println!("would break: {name} x{count} e.g. {described:?}");
    }
    let leaf_set: HashSet<IVec3> = leaves.iter().copied().collect();
    let mut distance: HashMap<IVec3, u32> = HashMap::new();
    let mut queue: VecDeque<IVec3> = VecDeque::new();
    for &log in &logs {
        queue.push_back(log);
        distance.insert(log, 0);
    }
    while let Some(pos) = queue.pop_front() {
        let d = distance[&pos];
        if d >= 6 {
            continue;
        }
        for offset in [IVec3::X, -IVec3::X, IVec3::Y, -IVec3::Y, IVec3::Z, -IVec3::Z] {
            let next = pos + offset;
            if leaf_set.contains(&next) && !distance.contains_key(&next) {
                distance.insert(next, d + 1);
                queue.push_back(next);
            }
        }
    }
    let far: Vec<IVec3> = leaves.iter().copied().filter(|pos| !distance.contains_key(pos)).collect();
    let near_edge = |pos: &IVec3| centers.values().any(|&(cx, cz)| (pos.x >> 4) - cx == 0 && (pos.z >> 4) - cz == 0);
    for pos in far.iter().filter(|p| near_edge(p)).take(10) {
        let mut nearby: BTreeMap<&str, usize> = BTreeMap::new();
        for offset in (-3..=3).flat_map(|dx| (-3..=3).flat_map(move |dy| (-3..=3).map(move |dz| IVec3::new(dx, dy, dz)))) {
            let neighbour = world.get(*pos + offset);
            if !blocks.is_air(neighbour) {
                *nearby.entry(blocks.name_of(neighbour)).or_default() += 1;
            }
        }
        println!("far leaf {pos} {}: {nearby:?}", blocks.name_of(world.get(*pos)));
    }
    println!(
        "{} leaves, {} more than 6 from a log (centre chunks: {}) e.g. {:?}",
        leaves.len(),
        far.len(),
        far.iter().filter(|p| near_edge(p)).count(),
        far.iter().filter(|p| near_edge(p)).take(5).collect::<Vec<_>>()
    );
}
