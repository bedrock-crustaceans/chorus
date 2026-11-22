use super::place::Rotation;
use super::template::Template;

#[derive(Debug, Clone, PartialEq)]
pub struct PlacedEntity {
    pub identifier: String,
    pub position: (f64, f64, f64),
}

pub fn entities_at(template: &Template, origin: (i32, i32, i32), rotation: Rotation, mirror: bool) -> Vec<PlacedEntity> {
    template
        .entities
        .iter()
        .map(|entity| {
            let [x, y, z] = entity.position;
            let local = if mirror { (x, y, 1.0 - z) } else { (x, y, z) };
            let (dx, dy, dz) = rotation.apply_point(local);
            PlacedEntity {
                identifier: entity.identifier.clone(),
                position: (f64::from(origin.0) + dx, f64::from(origin.1) + dy, f64::from(origin.2) + dz),
            }
        })
        .collect()
}

pub fn report_structure_entities(structure: &str, entities: &[PlacedEntity], inside: impl Fn(f64, f64) -> bool) {
    for entity in entities.iter().filter(|entity| inside(entity.position.0, entity.position.2)) {
        tracing::debug!(
            structure,
            entity = %entity.identifier,
            x = entity.position.0,
            y = entity.position.1,
            z = entity.position.2,
            "structure entities are not spawned yet"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::structure::data::StructureData;

    #[test]
    fn bastion_mob_entities_stay_in_the_cell_of_their_block_under_every_rotation() {
        let data = StructureData::shared().expect("structure data");
        let library = data.library.as_ref().expect("templates");
        let template = library.get("bastion/mobs/hoglin").expect("a bastion mob template");
        assert_eq!(template.entities.len(), 1);
        assert_eq!(template.entities[0].identifier, "minecraft:hoglin");
        for rotation in Rotation::ALL {
            for mirror in [false, true] {
                let placed = entities_at(&template, (100, 40, -50), rotation, mirror);
                let (x, y, z) = placed[0].position;
                assert_eq!((x.floor() as i32, z.floor() as i32), (100, -50), "{rotation:?} {mirror}");
                assert_eq!(y, 41.0);
            }
        }
    }
}
