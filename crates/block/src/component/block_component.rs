use std::any::Any;
use std::fmt::Debug;
use std::sync::Arc;

pub trait AsAny: Any {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait BlockComponentIntoArc: BlockComponent {
    fn into_arc(self) -> Arc<dyn BlockComponent>;
}

impl<T: BlockComponent> BlockComponentIntoArc for T {
    fn into_arc(self) -> Arc<dyn BlockComponent> {
        Arc::new(self)
    }
}

pub trait BlockComponent: Debug + AsAny + Send + Sync + 'static {}
