use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use crate::shared::structure::place::{Library, Options, Rotation};
use crate::shared::structure::plan::Plan;

pub const TRAPDOOR_CHANCE: f32 = 0.5;
const MIN_DEPTH: i32 = 4;
const DEPTH_RANGE: i32 = 8;
const SEGMENT_HEIGHT: i32 = 3;
const TOP_CONNECTION: (i32, i32, i32) = (3, -1, 5);
const MIDDLE_CONNECTION: (i32, i32, i32) = (1, 2, 1);
const BOTTOM_ENTRY: (i32, i32, i32) = (3, 5, 7);

pub fn place(plan: &mut Plan, library: &Library, random: &mut MersenneTwister, start: (i32, i32)) -> Probed<()> {
    let (x0, z0) = (start.0 * 16, start.1 * 16);
    let origin = (x0, plan.floor_height(x0, z0)? - 1, z0);
    let rotation = Rotation::from_index(random.next_int_bounded(4));
    let options = Options::default();
    let template = |name: &str| library.templates.get(name);
    if random.next_float() < TRAPDOOR_CHANCE {
        if let Some(top) = template("igloo/igloo_top_no_trapdoor") {
            library.place(plan, &top, origin, rotation, options);
        }
        return Ok(());
    }
    let (Some(top), Some(middle), Some(bottom)) = (template("igloo/igloo_top_trapdoor"), template("igloo/igloo_middle"), template("igloo/igloo_bottom")) else {
        return Ok(());
    };
    let chests = library.place(plan, &top, origin, rotation, options);
    random.skip(chests);
    let connected = |a: (i32, i32, i32), b: (i32, i32, i32)| {
        let (a, b) = (rotation.apply(a), rotation.apply(b));
        (origin.0 + a.0 - b.0, origin.1 + a.1 - b.1, origin.2 + a.2 - b.2)
    };
    let depth = random.next_int_bounded(DEPTH_RANGE) + MIN_DEPTH;
    for step in 0..depth {
        let at = connected((TOP_CONNECTION.0, TOP_CONNECTION.1 - step * SEGMENT_HEIGHT, TOP_CONNECTION.2), MIDDLE_CONNECTION);
        library.place(plan, &middle, at, rotation, options);
    }
    let at = connected((TOP_CONNECTION.0, TOP_CONNECTION.1 - depth * SEGMENT_HEIGHT, TOP_CONNECTION.2), BOTTOM_ENTRY);
    for (index, _) in &bottom.markers {
        let block = bottom.blocks[*index];
        let (dx, dy, dz) = rotation.apply((block.x as i32, block.y as i32, block.z as i32));
        plan.set(at.0 + dx, at.1 + dy, at.2 + dz, plan.ids.air);
    }
    library.place(plan, &bottom, at, rotation, options);
    Ok(())
}
