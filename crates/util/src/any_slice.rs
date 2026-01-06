use std::{any::Any, sync::Arc};

struct ArcSlice<T>(Arc<[T]>);

pub struct AnySlice {
    inner: Box<dyn Any + Send + Sync>,
    len: usize,
}

impl AnySlice {
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn at(&self, index: usize) -> AnySliceElementRef<'_> {
        AnySliceElementRef { slice: self, index }
    }

    pub fn downcast_ref<T: Send + Sync + 'static>(&self) -> Option<&[T]> {
        self.inner
            .downcast_ref::<ArcSlice<T>>()
            .map(|w| w.0.as_ref())
    }

    pub fn downcast_arc<T: Send + Sync + 'static>(&self) -> Option<Arc<[T]>> {
        self.inner
            .downcast_ref::<ArcSlice<T>>()
            .map(|w| Arc::clone(&w.0))
    }
}

impl<T: Send + Sync + 'static> From<Vec<T>> for AnySlice {
    fn from(value: Vec<T>) -> Self {
        Self {
            len: value.len(),
            inner: Box::new(ArcSlice(Arc::from(value))),
        }
    }
}

impl<T: Send + Sync + 'static> From<Arc<[T]>> for AnySlice {
    fn from(value: Arc<[T]>) -> Self {
        Self {
            len: value.len(),
            inner: Box::new(ArcSlice(value)),
        }
    }
}

pub struct AnySliceElementRef<'a> {
    slice: &'a AnySlice,
    index: usize,
}

impl<'a> AnySliceElementRef<'a> {
    pub fn downcast_ref<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.slice
            .downcast_ref()
            .and_then(|s: &[T]| s.get(self.index))
    }
}
