use super::data::{MATERIAL_ADJUSTMENTS, material_name, missing_identifiers};
use crate::shared::biome::Biome;
use crate::shared::biome_documents::ROOT;
use chorus_block::block_id::{COARSE_DIRT, PODZOL};

#[test]
fn every_generated_biome_has_surface_data() {
    assert!(missing_identifiers().iter().all(|biome| *biome == Biome::DAPPLED_FOREST));
}

#[test]
fn old_growth_taiga_adjustments_resolve_block_objects() {
    for biome in [Biome::OLD_GROWTH_PINE_TAIGA, Biome::OLD_GROWTH_SPRUCE_TAIGA] {
        let adjustments = &ROOT[biome.pack_name()]["components"][MATERIAL_ADJUSTMENTS]["adjustments"];
        assert_eq!(material_name(&adjustments[0]["materials"]["top_material"]), Some(COARSE_DIRT));
        assert_eq!(material_name(&adjustments[1]["materials"]["top_material"]), Some(PODZOL));
    }
}
