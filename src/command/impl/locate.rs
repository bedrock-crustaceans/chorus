use crate::command::command_definition::CommandDefinition;
use crate::command::context::CommandContext;
use crate::command::parameter::{CommandEnum, CommandParameter};
use crate::entity::components::transform::Transform;
use crate::level::{DimensionId, Level};
use crate::values;
use bevy_ecs::prelude::{Entity, Resource, World};
use chorus_core::permission::PermissionLevel;

const SEARCH_RADIUS_CHUNKS: i32 = 100;
const BIOME_SEARCH_RADIUS_BLOCKS: i32 = 6400;

const STRUCTURES: CommandEnum = CommandEnum::new(
    "Structure",
    values![
        "village",
        "stronghold",
        "temple",
        "mineshaft",
        "mansion",
        "buried_treasure",
        "pillager_outpost",
        "monument",
        "ruins",
        "shipwreck",
        "ruined_portal",
        "ancient_city",
        "trial_chambers",
        "trail_ruins",
        "abandoned_camp",
        "fortress",
        "bastion",
        "end_city"
    ],
);

const BIOMES: CommandEnum = CommandEnum::new(
    "Biome",
    values![
        "minecraft:ocean",
        "minecraft:deep_ocean",
        "minecraft:frozen_ocean",
        "minecraft:deep_frozen_ocean",
        "minecraft:cold_ocean",
        "minecraft:deep_cold_ocean",
        "minecraft:lukewarm_ocean",
        "minecraft:deep_lukewarm_ocean",
        "minecraft:warm_ocean",
        "minecraft:mushroom_island",
        "minecraft:plains",
        "minecraft:sunflower_plains",
        "minecraft:ice_plains",
        "minecraft:ice_plains_spikes",
        "minecraft:desert",
        "minecraft:swampland",
        "minecraft:mangrove_swamp",
        "minecraft:forest",
        "minecraft:flower_forest",
        "minecraft:birch_forest",
        "minecraft:roofed_forest",
        "minecraft:pale_garden",
        "minecraft:birch_forest_mutated",
        "minecraft:mega_taiga",
        "minecraft:redwood_taiga_mutated",
        "minecraft:taiga",
        "minecraft:cold_taiga",
        "minecraft:savanna",
        "minecraft:savanna_plateau",
        "minecraft:extreme_hills",
        "minecraft:extreme_hills_mutated",
        "minecraft:extreme_hills_plus_trees",
        "minecraft:savanna_mutated",
        "minecraft:jungle",
        "minecraft:jungle_edge",
        "minecraft:bamboo_jungle",
        "minecraft:mesa",
        "minecraft:mesa_bryce",
        "minecraft:mesa_plateau_stone",
        "minecraft:meadow",
        "minecraft:cherry_grove",
        "minecraft:grove",
        "minecraft:snowy_slopes",
        "minecraft:frozen_peaks",
        "minecraft:jagged_peaks",
        "minecraft:stony_peaks",
        "minecraft:river",
        "minecraft:frozen_river",
        "minecraft:beach",
        "minecraft:cold_beach",
        "minecraft:stone_beach",
        "minecraft:dripstone_caves",
        "minecraft:lush_caves",
        "minecraft:deep_dark",
        "minecraft:dappled_forest",
        "minecraft:sulfur_caves",
        "minecraft:hell",
        "minecraft:soulsand_valley",
        "minecraft:crimson_forest",
        "minecraft:warped_forest",
        "minecraft:basalt_deltas",
        "minecraft:the_end"
    ],
);

pub static LOCATE_COMMAND: CommandDefinition = CommandDefinition::new("locate", "Shows the coordinates of the nearest structure or biome", |context, args| {
    let biome = args.has("biome");
    let requested = args.string("kind").ok_or("no structure or biome given")?.to_owned();
    let label = if requested.contains(':') { requested.clone() } else { format!("minecraft:{requested}") };
    let name = if biome { format!("biome:{requested}") } else { requested.clone() };
    let radius = if biome { BIOME_SEARCH_RADIUS_BLOCKS } else { SEARCH_RADIUS_CHUNKS };
    let (origin, dimension_id) = origin(context);
    let from = if biome { origin } else { (origin.0 >> 4, origin.1 >> 4) };
    let sender = context.sender();
    let world = context.world_mut();
    let job = world
        .resource_mut::<Level>()
        .dimension_mut(dimension_id)
        .and_then(|dimension| dimension.start_locate(&name, from, radius))
        .ok_or("this dimension cannot locate that name")?;
    world.get_resource_or_insert_with(PendingLocates::default).0.push(PendingLocate {
        dimension: dimension_id,
        job,
        sender,
        name: label,
        origin,
        radius,
        biome,
    });
    Ok(())
})
.permission(PermissionLevel::Operator)
.overloads(crate::overloads![
    [CommandParameter::literal("structure"), CommandParameter::enumeration("kind", STRUCTURES)],
    [CommandParameter::literal("biome"), CommandParameter::enumeration("kind", BIOMES)]
]);

fn origin(context: &CommandContext) -> ((i32, i32), i32) {
    let dimension = context.get::<DimensionId>().map_or(0, |dimension| dimension.0);
    if let Some(transform) = context.get::<Transform>() {
        return ((transform.position.x.floor() as i32, transform.position.z.floor() as i32), dimension);
    }
    let spawn = context.world().get_resource::<Level>().map_or_else(Default::default, |level| level.spawn);
    ((spawn.x, spawn.z), dimension)
}

struct PendingLocate {
    dimension: i32,
    job: u64,
    sender: Entity,
    name: String,
    origin: (i32, i32),
    radius: i32,
    biome: bool,
}

#[derive(Resource, Default)]
pub struct PendingLocates(Vec<PendingLocate>);

pub fn has_pending_locates(pending: Option<bevy_ecs::prelude::Res<PendingLocates>>) -> bool {
    pending.is_some_and(|pending| !pending.0.is_empty())
}

pub fn deliver_locates(world: &mut World) {
    let mut finished = Vec::new();
    {
        let mut level = world.resource_mut::<Level>();
        let ids: Vec<i32> = level.dimensions().map(|dimension| dimension.id()).collect();
        for id in ids {
            if let Some(dimension) = level.dimension_mut(id) {
                finished.extend(dimension.take_located().into_iter().map(|(job, located)| (id, job, located)));
            }
        }
    }
    if finished.is_empty() {
        return;
    }
    let mut pending = std::mem::take(&mut world.resource_mut::<PendingLocates>().0);
    for (dimension, job, located) in finished {
        let Some(index) = pending.iter().position(|entry| entry.dimension == dimension && entry.job == job) else {
            continue;
        };
        let entry = pending.swap_remove(index);
        let message = match located {
            Some(found) => {
                let (block_x, block_z) = found.position.unwrap_or((found.x * 16 + 8, found.z * 16 + 8));
                let height = found.y.map_or_else(|| "(y?)".to_owned(), |y| y.to_string());
                let distance = (((block_x - entry.origin.0) as f64).powi(2) + ((block_z - entry.origin.1) as f64).powi(2)).sqrt().round() as i64;
                format!("The nearest {} is at block {block_x}, {height}, {block_z} ({distance} blocks away)", entry.name)
            }
            None if entry.biome => format!("Could not find a biome of type {} within a reasonable distance", entry.name),
            None => format!("Could not find a {} within {} chunks", entry.name, entry.radius),
        };
        CommandContext::new(world, entry.sender).reply(message);
    }
    world.resource_mut::<PendingLocates>().0.extend(pending);
}

#[cfg(test)]
mod tests {
    use super::BIOMES;
    use crate::command::args::parse;
    use crate::command::parameter::{CommandEnum, CommandParameter};
    use crate::registry::command_registry::CommandRegistry;
    use chorus_core::permission::PermissionLevel;

    const OVERLOADS: &[crate::command::parameter::CommandOverload] = crate::overloads![[CommandParameter::literal("biome"), CommandParameter::enumeration("kind", BIOMES)]];

    const STRUCTURE_OVERLOADS: &[crate::command::parameter::CommandOverload] = crate::overloads![[
        CommandParameter::literal("structure"),
        CommandParameter::enumeration("kind", CommandEnum::new("Structure", crate::values!["village"]))
    ]];

    #[test]
    fn the_biome_overload_keeps_the_namespaced_identifier() {
        let args = parse("locate", OVERLOADS, "biome minecraft:ocean", &CommandRegistry::new(), PermissionLevel::Operator).expect("a biome request");
        assert!(args.has("biome"));
        assert_eq!(args.string("kind"), Some("minecraft:ocean"));
        let args = parse("locate", STRUCTURE_OVERLOADS, "structure village", &CommandRegistry::new(), PermissionLevel::Operator).expect("a structure request");
        assert!(!args.has("biome"));
        assert_eq!(args.string("kind"), Some("village"));
    }
}
