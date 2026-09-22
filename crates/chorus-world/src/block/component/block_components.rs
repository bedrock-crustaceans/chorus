use crate::block::component::block_component::BlockComponent;
use atomicow::CowArc;
use std::any::TypeId;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BlockComponents {
    map: HashMap<TypeId, CowArc<'static, dyn BlockComponent>>,
}

impl Default for BlockComponents {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockComponents {
    pub fn new() -> Self {
        Self { map: HashMap::new() }
    }

    pub fn insert(&mut self, component: CowArc<'static, dyn BlockComponent>) {
        let component_ref: &dyn BlockComponent = &*component;
        let type_id = component_ref.as_any().type_id();

        self.map.insert(type_id, component);
    }

    pub fn get<T: BlockComponent>(&self) -> Option<&T> {
        self.map.get(&TypeId::of::<T>()).and_then(|component| {
            let component: &dyn BlockComponent = &**component;
            component.as_any().downcast_ref::<T>()
        })
    }

    pub fn contains<T: BlockComponent>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }
}
