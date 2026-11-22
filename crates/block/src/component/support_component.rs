use crate::component::block_component::BlockComponent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupportFaces {
    None,
    Top,
    All,
}

#[derive(Clone, Debug)]
pub struct SupportComponent {
    pub faces: SupportFaces,
}

impl BlockComponent for SupportComponent {}

impl SupportComponent {
    pub const TOP: Self = Self { faces: SupportFaces::Top };
    pub const ALL: Self = Self { faces: SupportFaces::All };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constants_carry_their_faces() {
        assert_eq!(SupportComponent::TOP.faces, SupportFaces::Top);
        assert_eq!(SupportComponent::ALL.faces, SupportFaces::All);
    }

    #[test]
    fn faces_are_distinct() {
        assert_ne!(SupportFaces::None, SupportFaces::Top);
        assert_ne!(SupportFaces::Top, SupportFaces::All);
        assert_ne!(SupportFaces::None, SupportFaces::All);
    }

    #[test]
    fn blocks_declare_the_support_they_give() {
        use crate::block_id;
        use crate::block_registry::BlockRegistry;
        use crate::r#impl::DEFINITIONS;
        let mut registry = BlockRegistry::new();
        registry.register_all(DEFINITIONS.iter().copied());
        let faces = |identifier: &str| {
            let id = registry.get_block_id(identifier)?;
            registry.get_components(id)?.get::<SupportComponent>().map(|support| support.faces)
        };
        assert_eq!(faces(block_id::GRASS_BLOCK), Some(SupportFaces::All));
        assert_eq!(faces(block_id::HOPPER), Some(SupportFaces::Top));
        assert_eq!(faces(block_id::AIR), None);
    }
}
